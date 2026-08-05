use std::{collections::HashMap, net::IpAddr, str::FromStr};

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use ipnetwork::IpNetwork;
use sqlx::{FromRow, PgPool, Row, types::Json};
use uuid::Uuid;

use super::types::{
    AdCollectionIssue, AdCollectionRequest, AdCollectionRun, AdCollector, AdDomain, AdEntity,
    AdEntityKind, AdEntityListQuery, AdListQuery,
};

const COLLECTION_RUN_RETENTION_PER_TARGET: i64 = 100;

#[derive(FromRow)]
pub(crate) struct AdCollectionRunRow {
    id: Uuid,
    organization_id: Uuid,
    network_id: Uuid,
    daemon_id: Option<Uuid>,
    credential_id: Option<Uuid>,
    target_host_id: Option<Uuid>,
    target_ip: IpNetwork,
    discovery_id: Option<Uuid>,
    session_id: Uuid,
    collection_key: String,
    collector: String,
    status: String,
    started_at: DateTime<Utc>,
    completed_at: DateTime<Utc>,
    domain_count: i64,
    entity_count: i64,
    truncated: bool,
    inventory_applied: bool,
    issues: Json<Vec<AdCollectionIssue>>,
    created_at: DateTime<Utc>,
}

impl TryFrom<AdCollectionRunRow> for AdCollectionRun {
    type Error = anyhow::Error;

    fn try_from(row: AdCollectionRunRow) -> Result<Self> {
        Ok(Self {
            id: row.id,
            organization_id: row.organization_id,
            network_id: row.network_id,
            daemon_id: row.daemon_id,
            credential_id: row.credential_id,
            target_host_id: row.target_host_id,
            target_ip: row.target_ip.ip(),
            discovery_id: row.discovery_id,
            session_id: row.session_id,
            collection_key: row.collection_key,
            collector: row.collector.parse().map_err(anyhow::Error::msg)?,
            status: row.status.parse().map_err(anyhow::Error::msg)?,
            started_at: row.started_at,
            completed_at: row.completed_at,
            domain_count: row.domain_count.try_into()?,
            entity_count: row.entity_count.try_into()?,
            truncated: row.truncated,
            inventory_applied: row.inventory_applied,
            issues: row.issues.0,
            created_at: row.created_at,
        })
    }
}

/// Identity proven by the authenticated ingestion handler. The collection key
/// is derived server-side from the stored credential, exact live host, and
/// exact target IP.
#[derive(Debug, Clone)]
pub struct VerifiedAdTarget {
    pub organization_id: Uuid,
    pub network_id: Uuid,
    pub daemon_id: Uuid,
    pub credential_id: Uuid,
    pub target_host_id: Uuid,
    pub target_ip: IpAddr,
    pub discovery_id: Uuid,
    pub session_id: Uuid,
    pub collection_key: String,
    pub collector: AdCollector,
}

#[derive(FromRow)]
pub(crate) struct AdEntityRow {
    id: Uuid,
    organization_id: Uuid,
    network_id: Uuid,
    domain_id: Uuid,
    collection_run_id: Uuid,
    kind: String,
    external_id: String,
    name: String,
    dns_name: Option<String>,
    parent_external_id: Option<String>,
    related_external_id: Option<String>,
    site_name: Option<String>,
    operating_system: Option<String>,
    operating_system_version: Option<String>,
    network_prefix: Option<String>,
    is_enabled: Option<bool>,
    observed_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<AdEntityRow> for AdEntity {
    type Error = anyhow::Error;

    fn try_from(row: AdEntityRow) -> Result<Self> {
        Ok(Self {
            id: row.id,
            organization_id: row.organization_id,
            network_id: row.network_id,
            domain_id: row.domain_id,
            collection_run_id: row.collection_run_id,
            kind: AdEntityKind::from_str(&row.kind).map_err(anyhow::Error::msg)?,
            external_id: row.external_id,
            name: row.name,
            dns_name: row.dns_name,
            parent_external_id: row.parent_external_id,
            related_external_id: row.related_external_id,
            site_name: row.site_name,
            operating_system: row.operating_system,
            operating_system_version: row.operating_system_version,
            network_prefix: row.network_prefix,
            is_enabled: row.is_enabled,
            observed_at: row.observed_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

pub async fn organization_for_network(pool: &PgPool, network_id: Uuid) -> Result<Option<Uuid>> {
    let row = sqlx::query("SELECT organization_id FROM networks WHERE id = $1")
        .bind(network_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|row| row.get("organization_id")))
}

/// Resolve the live host owning an exact IP on a network. More than one match
/// is treated as ambiguous and therefore unauthorized by the handler.
pub async fn live_hosts_for_target_ip(
    pool: &PgPool,
    network_id: Uuid,
    target_ip: IpAddr,
) -> Result<Vec<Uuid>> {
    Ok(sqlx::query_scalar(
        r#"SELECT DISTINCT host_id
           FROM ip_addresses
           WHERE network_id = $1 AND ip_address = $2 AND valid_to IS NULL
           ORDER BY host_id"#,
    )
    .bind(network_id)
    .bind(target_ip)
    .fetch_all(pool)
    .await?)
}

/// Check the persisted host credential junction, including its optional exact
/// IP-address scope. This is the authoritative source after an integration
/// target has been attached to a discovered host.
pub async fn host_assignment_authorizes(
    pool: &PgPool,
    network_id: Uuid,
    host_id: Uuid,
    target_ip: IpAddr,
    credential_id: Uuid,
) -> Result<bool> {
    Ok(sqlx::query_scalar::<_, bool>(
        r#"SELECT EXISTS (
               SELECT 1
               FROM host_credentials hc
               JOIN hosts h ON h.id = hc.host_id AND h.valid_to IS NULL
               JOIN ip_addresses ip
                 ON ip.host_id = h.id
                AND ip.network_id = $1
                AND ip.ip_address = $2
                AND ip.valid_to IS NULL
               WHERE hc.host_id = $3
                 AND hc.credential_id = $4
                 AND (hc.ip_address_ids IS NULL OR ip.id = ANY(hc.ip_address_ids))
           )"#,
    )
    .bind(network_id)
    .bind(target_ip)
    .bind(host_id)
    .bind(credential_id)
    .fetch_one(pool)
    .await?)
}

/// Persist a collection run and, only for a complete successful response,
/// atomically replace current inventory. Target-scoped advisory locking and
/// completed-at ordering prevent concurrent or delayed runs from regressing
/// newer inventory.
pub async fn ingest_collection(
    pool: &PgPool,
    target: &VerifiedAdTarget,
    request: &AdCollectionRequest,
) -> Result<AdCollectionRun> {
    let run_id = Uuid::new_v4();
    let domain_count: i64 = request.domains.len().try_into()?;
    let entity_count: i64 = request
        .domains
        .iter()
        .map(|domain| domain.entities.len())
        .sum::<usize>()
        .try_into()?;
    let mut transaction = pool.begin().await?;

    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(&target.collection_key)
        .execute(&mut *transaction)
        .await
        .context("lock AD collection target")?;

    // Compare inside PostgreSQL so the request parameter and stored value use
    // the same timestamp precision. Comparing a nanosecond-resolution Rust
    // value with PostgreSQL's microsecond-resolution value can otherwise make
    // an equal-timestamp replay appear a fraction newer.
    let has_same_or_newer_complete: bool = sqlx::query_scalar(
        r#"SELECT EXISTS (
               SELECT 1 FROM ad_collection_runs
               WHERE network_id = $1 AND collection_key = $2
                 AND status = 'succeeded' AND truncated = FALSE
                 AND completed_at >= $3
           )"#,
    )
    .bind(target.network_id)
    .bind(&target.collection_key)
    .bind(request.completed_at)
    .fetch_one(&mut *transaction)
    .await
    .context("read latest complete AD collection")?;
    let replaces_inventory = request.replaces_inventory() && !has_same_or_newer_complete;

    sqlx::query(
        r#"INSERT INTO ad_collection_runs (
               id, organization_id, network_id, daemon_id, credential_id,
               target_host_id, target_ip, discovery_id, session_id,
               collection_key, collector, status, started_at, completed_at,
               domain_count, entity_count, truncated, inventory_applied, issues
           ) VALUES (
               $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12,
               $13, $14, $15, $16, $17, $18, $19
           )"#,
    )
    .bind(run_id)
    .bind(target.organization_id)
    .bind(target.network_id)
    .bind(target.daemon_id)
    .bind(target.credential_id)
    .bind(target.target_host_id)
    .bind(target.target_ip)
    .bind(target.discovery_id)
    .bind(target.session_id)
    .bind(&target.collection_key)
    .bind(target.collector.to_string())
    .bind(request.status.to_string())
    .bind(request.started_at)
    .bind(request.completed_at)
    .bind(domain_count)
    .bind(entity_count)
    .bind(request.truncated)
    .bind(replaces_inventory)
    .bind(Json(&request.issues))
    .execute(&mut *transaction)
    .await
    .context("insert AD collection run")?;

    if replaces_inventory {
        let domain_ids: Vec<Uuid> = request.domains.iter().map(|_| Uuid::new_v4()).collect();
        let dns_names: Vec<String> = request
            .domains
            .iter()
            .map(|domain| domain.dns_name.to_ascii_lowercase())
            .collect();
        let forest_dns_names: Vec<Option<String>> = request
            .domains
            .iter()
            .map(|domain| {
                domain
                    .forest_dns_name
                    .as_ref()
                    .map(|name| name.to_ascii_lowercase())
            })
            .collect();
        let netbios_names: Vec<Option<String>> = request
            .domains
            .iter()
            .map(|domain| domain.netbios_name.clone())
            .collect();
        let functional_levels: Vec<Option<String>> = request
            .domains
            .iter()
            .map(|domain| domain.functional_level.clone())
            .collect();
        let observed_at: Vec<DateTime<Utc>> = request
            .domains
            .iter()
            .map(|domain| domain.observed_at)
            .collect();

        let persisted_domains: Vec<(Uuid, String)> = if request.domains.is_empty() {
            Vec::new()
        } else {
            sqlx::query_as(
                r#"INSERT INTO ad_domains (
                       id, organization_id, network_id, collection_key, dns_name,
                       forest_dns_name, netbios_name, functional_level,
                       last_collection_run_id, observed_at
                   )
                   SELECT input.id, $1, $2, $3, input.dns_name,
                          input.forest_dns_name, input.netbios_name,
                          input.functional_level, $4, input.observed_at
                   FROM UNNEST(
                       $5::uuid[], $6::text[], $7::text[], $8::text[],
                       $9::text[], $10::timestamptz[]
                   ) AS input(
                       id, dns_name, forest_dns_name, netbios_name,
                       functional_level, observed_at
                   )
                   ON CONFLICT (network_id, collection_key, dns_name) DO UPDATE SET
                       organization_id = EXCLUDED.organization_id,
                       forest_dns_name = EXCLUDED.forest_dns_name,
                       netbios_name = EXCLUDED.netbios_name,
                       functional_level = EXCLUDED.functional_level,
                       last_collection_run_id = EXCLUDED.last_collection_run_id,
                       observed_at = EXCLUDED.observed_at,
                       updated_at = NOW()
                   RETURNING id, dns_name"#,
            )
            .bind(target.organization_id)
            .bind(target.network_id)
            .bind(&target.collection_key)
            .bind(run_id)
            .bind(&domain_ids)
            .bind(&dns_names)
            .bind(&forest_dns_names)
            .bind(&netbios_names)
            .bind(&functional_levels)
            .bind(&observed_at)
            .fetch_all(&mut *transaction)
            .await
            .context("batch upsert AD domains")?
        };
        let domain_id_by_name: HashMap<String, Uuid> = persisted_domains
            .into_iter()
            .map(|(id, name)| (name, id))
            .collect();

        let mut entity_ids = Vec::with_capacity(entity_count as usize);
        let mut entity_domain_ids = Vec::with_capacity(entity_count as usize);
        let mut kinds = Vec::with_capacity(entity_count as usize);
        let mut external_ids = Vec::with_capacity(entity_count as usize);
        let mut names = Vec::with_capacity(entity_count as usize);
        let mut entity_dns_names = Vec::with_capacity(entity_count as usize);
        let mut parent_ids = Vec::with_capacity(entity_count as usize);
        let mut related_ids = Vec::with_capacity(entity_count as usize);
        let mut site_names = Vec::with_capacity(entity_count as usize);
        let mut operating_systems = Vec::with_capacity(entity_count as usize);
        let mut operating_system_versions = Vec::with_capacity(entity_count as usize);
        let mut network_prefixes = Vec::with_capacity(entity_count as usize);
        let mut enabled = Vec::with_capacity(entity_count as usize);
        let mut entity_observed_at = Vec::with_capacity(entity_count as usize);

        for domain in &request.domains {
            let domain_id = *domain_id_by_name
                .get(&domain.dns_name.to_ascii_lowercase())
                .ok_or_else(|| anyhow!("upserted AD domain was not returned"))?;
            for entity in &domain.entities {
                entity_ids.push(Uuid::new_v4());
                entity_domain_ids.push(domain_id);
                kinds.push(entity.kind.to_string());
                external_ids.push(entity.external_id.clone());
                names.push(entity.name.clone());
                entity_dns_names.push(
                    entity
                        .dns_name
                        .as_ref()
                        .map(|name| name.to_ascii_lowercase()),
                );
                parent_ids.push(entity.parent_external_id.clone());
                related_ids.push(entity.related_external_id.clone());
                site_names.push(entity.site_name.clone());
                operating_systems.push(entity.operating_system.clone());
                operating_system_versions.push(entity.operating_system_version.clone());
                network_prefixes.push(
                    entity
                        .network_prefix
                        .as_deref()
                        .map(IpNetwork::from_str)
                        .transpose()
                        .context("parse validated AD network prefix")?,
                );
                enabled.push(entity.is_enabled);
                entity_observed_at.push(entity.observed_at);
            }
        }

        if !entity_ids.is_empty() {
            sqlx::query(
                r#"INSERT INTO ad_entities (
                       id, organization_id, network_id, domain_id,
                       collection_run_id, kind, external_id, name, dns_name,
                       parent_external_id, related_external_id, site_name,
                       operating_system, operating_system_version,
                       network_prefix, is_enabled, observed_at
                   )
                   SELECT input.id, $1, $2, input.domain_id, $3, input.kind,
                          input.external_id, input.name, input.dns_name,
                          input.parent_external_id, input.related_external_id,
                          input.site_name, input.operating_system,
                          input.operating_system_version, input.network_prefix,
                          input.is_enabled, input.observed_at
                   FROM UNNEST(
                       $4::uuid[], $5::uuid[], $6::text[], $7::text[],
                       $8::text[], $9::text[], $10::text[], $11::text[],
                       $12::text[], $13::text[], $14::text[], $15::cidr[],
                       $16::boolean[], $17::timestamptz[]
                   ) AS input(
                       id, domain_id, kind, external_id, name, dns_name,
                       parent_external_id, related_external_id, site_name,
                       operating_system, operating_system_version,
                       network_prefix, is_enabled, observed_at
                   )
                   ON CONFLICT (domain_id, kind, external_id) DO UPDATE SET
                       organization_id = EXCLUDED.organization_id,
                       network_id = EXCLUDED.network_id,
                       collection_run_id = EXCLUDED.collection_run_id,
                       name = EXCLUDED.name,
                       dns_name = EXCLUDED.dns_name,
                       parent_external_id = EXCLUDED.parent_external_id,
                       related_external_id = EXCLUDED.related_external_id,
                       site_name = EXCLUDED.site_name,
                       operating_system = EXCLUDED.operating_system,
                       operating_system_version = EXCLUDED.operating_system_version,
                       network_prefix = EXCLUDED.network_prefix,
                       is_enabled = EXCLUDED.is_enabled,
                       observed_at = EXCLUDED.observed_at,
                       updated_at = NOW()"#,
            )
            .bind(target.organization_id)
            .bind(target.network_id)
            .bind(run_id)
            .bind(&entity_ids)
            .bind(&entity_domain_ids)
            .bind(&kinds)
            .bind(&external_ids)
            .bind(&names)
            .bind(&entity_dns_names)
            .bind(&parent_ids)
            .bind(&related_ids)
            .bind(&site_names)
            .bind(&operating_systems)
            .bind(&operating_system_versions)
            .bind(&network_prefixes)
            .bind(&enabled)
            .bind(&entity_observed_at)
            .execute(&mut *transaction)
            .await
            .context("batch upsert AD entities")?;
        }

        sqlx::query(
            r#"DELETE FROM ad_entities entity
               USING ad_domains domain
               WHERE entity.domain_id = domain.id
                 AND domain.network_id = $1
                 AND domain.collection_key = $2
                 AND entity.collection_run_id <> $3"#,
        )
        .bind(target.network_id)
        .bind(&target.collection_key)
        .bind(run_id)
        .execute(&mut *transaction)
        .await
        .context("replace stale AD entities")?;

        sqlx::query(
            r#"DELETE FROM ad_domains
               WHERE network_id = $1 AND collection_key = $2
                 AND last_collection_run_id <> $3"#,
        )
        .bind(target.network_id)
        .bind(&target.collection_key)
        .bind(run_id)
        .execute(&mut *transaction)
        .await
        .context("replace stale AD domains")?;
    }

    // Bound provenance history per collector target. Rank the currently
    // referenced run first so it consumes one of the retention slots, then
    // retain the newest unreferenced runs. Merely excluding referenced rows
    // from deletion after applying OFFSET can leave retention + 1 rows.
    sqlx::query(
        r#"DELETE FROM ad_collection_runs AS run
           WHERE run.id IN (
               SELECT candidate.id
                FROM ad_collection_runs AS candidate
                WHERE candidate.network_id = $1
                  AND candidate.collection_key = $2
                ORDER BY (
                    EXISTS (
                        SELECT 1 FROM ad_domains AS ad_domain
                        WHERE ad_domain.last_collection_run_id = candidate.id
                    ) OR EXISTS (
                        SELECT 1 FROM ad_entities AS ad_entity
                        WHERE ad_entity.collection_run_id = candidate.id
                    )
                ) DESC, candidate.received_order DESC
               OFFSET $3
           )
           AND NOT EXISTS (
               SELECT 1 FROM ad_domains AS ad_domain
               WHERE ad_domain.last_collection_run_id = run.id
           )
           AND NOT EXISTS (
               SELECT 1 FROM ad_entities AS ad_entity
               WHERE ad_entity.collection_run_id = run.id
           )"#,
    )
    .bind(target.network_id)
    .bind(&target.collection_key)
    .bind(COLLECTION_RUN_RETENTION_PER_TARGET)
    .execute(&mut *transaction)
    .await
    .context("prune unreferenced AD collection history")?;

    transaction.commit().await?;
    get_collection_run(pool, run_id)
        .await?
        .ok_or_else(|| anyhow!("inserted AD collection run was not found"))
}

pub async fn get_collection_run(pool: &PgPool, id: Uuid) -> Result<Option<AdCollectionRun>> {
    let row = sqlx::query_as::<_, AdCollectionRunRow>(
        r#"SELECT id, organization_id, network_id, daemon_id, credential_id,
                  target_host_id, target_ip, discovery_id, session_id,
                  collection_key, collector, status, started_at, completed_at,
                  domain_count, entity_count, truncated, inventory_applied,
                  issues, created_at
           FROM ad_collection_runs WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    row.map(TryInto::try_into).transpose()
}

pub async fn list_domains(
    pool: &PgPool,
    network_ids: &[Uuid],
    query: &AdListQuery,
) -> Result<(Vec<AdDomain>, u64)> {
    let (limit, offset) = query.pagination();
    let items = sqlx::query_as::<_, AdDomain>(
        r#"SELECT id, organization_id, network_id, collection_key, dns_name,
                  forest_dns_name, netbios_name, functional_level,
                  last_collection_run_id, observed_at, created_at, updated_at
           FROM ad_domains
           WHERE network_id = ANY($1)
             AND ($2::uuid IS NULL OR network_id = $2)
           ORDER BY dns_name ASC, id ASC
           LIMIT $3 OFFSET $4"#,
    )
    .bind(network_ids)
    .bind(query.network_id)
    .bind(i64::from(limit))
    .bind(i64::from(offset))
    .fetch_all(pool)
    .await?;
    let total: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM ad_domains
           WHERE network_id = ANY($1)
             AND ($2::uuid IS NULL OR network_id = $2)"#,
    )
    .bind(network_ids)
    .bind(query.network_id)
    .fetch_one(pool)
    .await?;
    Ok((items, total.try_into()?))
}

pub async fn list_entities(
    pool: &PgPool,
    network_ids: &[Uuid],
    query: &AdEntityListQuery,
) -> Result<(Vec<AdEntity>, u64)> {
    let (limit, offset) = query.pagination();
    let kind = query.kind.map(|kind| kind.to_string());
    let rows = sqlx::query_as::<_, AdEntityRow>(
        r#"SELECT id, organization_id, network_id, domain_id,
                  collection_run_id, kind, external_id, name, dns_name,
                  parent_external_id, related_external_id, site_name,
                  operating_system, operating_system_version,
                  network_prefix::text AS network_prefix, is_enabled,
                  observed_at, created_at, updated_at
           FROM ad_entities
           WHERE network_id = ANY($1)
             AND ($2::uuid IS NULL OR network_id = $2)
             AND ($3::uuid IS NULL OR domain_id = $3)
             AND ($4::text IS NULL OR kind = $4)
           ORDER BY kind ASC, name ASC, id ASC
           LIMIT $5 OFFSET $6"#,
    )
    .bind(network_ids)
    .bind(query.network_id)
    .bind(query.domain_id)
    .bind(&kind)
    .bind(i64::from(limit))
    .bind(i64::from(offset))
    .fetch_all(pool)
    .await?;
    let total: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM ad_entities
           WHERE network_id = ANY($1)
             AND ($2::uuid IS NULL OR network_id = $2)
             AND ($3::uuid IS NULL OR domain_id = $3)
             AND ($4::text IS NULL OR kind = $4)"#,
    )
    .bind(network_ids)
    .bind(query.network_id)
    .bind(query.domain_id)
    .bind(kind)
    .fetch_one(pool)
    .await?;
    Ok((
        rows.into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>>>()?,
        total.try_into()?,
    ))
}

pub async fn list_collection_runs(
    pool: &PgPool,
    network_ids: &[Uuid],
    query: &AdListQuery,
) -> Result<(Vec<AdCollectionRun>, u64)> {
    let (limit, offset) = query.pagination();
    let rows = sqlx::query_as::<_, AdCollectionRunRow>(
        r#"SELECT id, organization_id, network_id, daemon_id, credential_id,
                  target_host_id, target_ip, discovery_id, session_id,
                  collection_key, collector, status, started_at, completed_at,
                  domain_count, entity_count, truncated, inventory_applied,
                  issues, created_at
           FROM ad_collection_runs
           WHERE network_id = ANY($1)
             AND ($2::uuid IS NULL OR network_id = $2)
           ORDER BY completed_at DESC, id DESC
           LIMIT $3 OFFSET $4"#,
    )
    .bind(network_ids)
    .bind(query.network_id)
    .bind(i64::from(limit))
    .bind(i64::from(offset))
    .fetch_all(pool)
    .await?;
    let total: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM ad_collection_runs
           WHERE network_id = ANY($1)
             AND ($2::uuid IS NULL OR network_id = $2)"#,
    )
    .bind(network_ids)
    .bind(query.network_id)
    .fetch_one(pool)
    .await?;
    Ok((
        rows.into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>>>()?,
        total.try_into()?,
    ))
}
