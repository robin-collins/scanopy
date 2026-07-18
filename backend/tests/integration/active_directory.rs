use std::net::IpAddr;

use chrono::{Duration, Utc};
use scanopy::server::active_directory::{
    storage::{VerifiedAdTarget, ingest_collection},
    types::{
        AdCollectedDomain, AdCollectedEntity, AdCollectionRequest, AdCollectionStatus, AdEntityKind,
    },
};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::infra::TestContext;

pub async fn run_storage_tests(ctx: &TestContext) -> Result<(), String> {
    let identity = fixture_identity(&ctx.db_pool, ctx.network_id, ctx.organization_id).await?;
    let credential_id = identity.credential_id;

    let first = request(
        &identity,
        "first",
        -20,
        AdCollectionStatus::Succeeded,
        false,
    );
    let first_run = ingest_collection(&ctx.db_pool, &identity, &first)
        .await
        .map_err(|error| error.to_string())?;
    assert!(first_run.inventory_applied);
    assert_eq!(
        current_entity_name(&ctx.db_pool, &identity.collection_key).await?,
        "first"
    );

    for (status, truncated) in [
        (AdCollectionStatus::Partial, false),
        (AdCollectionStatus::Failed, false),
        (AdCollectionStatus::Succeeded, true),
    ] {
        let candidate = request(&identity, "must-not-apply", -15, status, truncated);
        let run = ingest_collection(&ctx.db_pool, &identity, &candidate)
            .await
            .map_err(|error| error.to_string())?;
        assert!(!run.inventory_applied);
        assert_eq!(
            current_entity_name(&ctx.db_pool, &identity.collection_key).await?,
            "first"
        );
        let references: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM ad_entities WHERE collection_run_id = $1")
                .bind(run.id)
                .fetch_one(&ctx.db_pool)
                .await
                .map_err(|error| error.to_string())?;
        assert_eq!(references, 0);
    }

    let older = request(
        &identity,
        "older",
        -10,
        AdCollectionStatus::Succeeded,
        false,
    );
    let newer = request(&identity, "newer", -5, AdCollectionStatus::Succeeded, false);
    let (older_result, newer_result) = tokio::join!(
        ingest_collection(&ctx.db_pool, &identity, &older),
        ingest_collection(&ctx.db_pool, &identity, &newer)
    );
    older_result.map_err(|error| error.to_string())?;
    newer_result.map_err(|error| error.to_string())?;
    assert_eq!(
        current_entity_name(&ctx.db_pool, &identity.collection_key).await?,
        "newer"
    );

    let mut equal_timestamp_replay = request(
        &identity,
        "must-not-replace-equal-timestamp",
        -5,
        AdCollectionStatus::Succeeded,
        false,
    );
    equal_timestamp_replay.started_at = newer.started_at;
    equal_timestamp_replay.completed_at = newer.completed_at;
    equal_timestamp_replay.domains[0].observed_at = newer.domains[0].observed_at;
    equal_timestamp_replay.domains[0].entities[0].observed_at =
        newer.domains[0].entities[0].observed_at;
    let replay_run = ingest_collection(&ctx.db_pool, &identity, &equal_timestamp_replay)
        .await
        .map_err(|error| error.to_string())?;
    assert!(!replay_run.inventory_applied);
    assert_eq!(
        current_entity_name(&ctx.db_pool, &identity.collection_key).await?,
        "newer"
    );

    for index in 0..105 {
        let failed = request(
            &identity,
            &format!("failed-{index}"),
            -4,
            AdCollectionStatus::Failed,
            false,
        );
        ingest_collection(&ctx.db_pool, &identity, &failed)
            .await
            .map_err(|error| error.to_string())?;
    }
    let run_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM ad_collection_runs WHERE collection_key = $1")
            .bind(&identity.collection_key)
            .fetch_one(&ctx.db_pool)
            .await
            .map_err(|error| error.to_string())?;
    assert_eq!(run_count, 100);

    sqlx::query("DELETE FROM credentials WHERE id = $1")
        .bind(credential_id)
        .execute(&ctx.db_pool)
        .await
        .map_err(|error| error.to_string())?;
    let credential_refs: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ad_collection_runs WHERE collection_key = $1 AND credential_id IS NOT NULL",
    )
    .bind(&identity.collection_key)
    .fetch_one(&ctx.db_pool)
    .await
    .map_err(|error| error.to_string())?;
    assert_eq!(credential_refs, 0);

    sqlx::query("DELETE FROM ad_domains WHERE collection_key = $1")
        .bind(&identity.collection_key)
        .execute(&ctx.db_pool)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query("DELETE FROM ad_collection_runs WHERE collection_key = $1")
        .bind(&identity.collection_key)
        .execute(&ctx.db_pool)
        .await
        .map_err(|error| error.to_string())?;
    Ok(())
}

async fn fixture_identity(
    pool: &PgPool,
    network_id: Uuid,
    organization_id: Uuid,
) -> Result<VerifiedAdTarget, String> {
    let discovery = sqlx::query(
        "SELECT id, daemon_id FROM discovery WHERE network_id = $1 ORDER BY created_at LIMIT 1",
    )
    .bind(network_id)
    .fetch_one(pool)
    .await
    .map_err(|error| error.to_string())?;
    let target_host_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM hosts WHERE network_id = $1 ORDER BY created_at LIMIT 1",
    )
    .bind(network_id)
    .fetch_one(pool)
    .await
    .map_err(|error| error.to_string())?;
    let credential_id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO credentials (
               id, organization_id, name, credential_type, created_at, updated_at
           ) VALUES (
               $1, $2, 'AD persistence integration test',
               '{"type":"ActiveDirectoryLdaps"}'::jsonb, NOW(), NOW()
           )"#,
    )
    .bind(credential_id)
    .bind(organization_id)
    .execute(pool)
    .await
    .map_err(|error| error.to_string())?;
    let target_ip: IpAddr = "192.0.2.200".parse().unwrap();
    Ok(VerifiedAdTarget {
        organization_id,
        network_id,
        daemon_id: discovery.get("daemon_id"),
        credential_id,
        target_host_id,
        target_ip,
        discovery_id: discovery.get("id"),
        session_id: Uuid::new_v4(),
        collection_key: format!("{credential_id}@{target_host_id}@{target_ip}"),
        collector: scanopy::server::active_directory::types::AdCollector::Ldaps,
    })
}

fn request(
    identity: &VerifiedAdTarget,
    name: &str,
    completed_offset_seconds: i64,
    status: AdCollectionStatus,
    truncated: bool,
) -> AdCollectionRequest {
    let completed_at = Utc::now() + Duration::seconds(completed_offset_seconds);
    let observed_at = completed_at - Duration::seconds(1);
    AdCollectionRequest {
        network_id: identity.network_id,
        credential_id: identity.credential_id,
        target_host_id: identity.target_host_id,
        target_ip: identity.target_ip,
        discovery_id: identity.discovery_id,
        session_id: identity.session_id,
        status,
        started_at: completed_at - Duration::seconds(2),
        completed_at,
        truncated,
        issues: vec![],
        domains: vec![AdCollectedDomain {
            dns_name: "integration.example.test".into(),
            forest_dns_name: Some("integration.example.test".into()),
            netbios_name: Some("INTEGRATION".into()),
            functional_level: Some("Windows2016Domain".into()),
            observed_at,
            entities: vec![AdCollectedEntity {
                kind: AdEntityKind::Computer,
                external_id: Uuid::from_u128(1).to_string(),
                name: name.into(),
                dns_name: Some("computer.integration.example.test".into()),
                parent_external_id: None,
                related_external_id: None,
                site_name: Some("Integration".into()),
                operating_system: Some("Windows".into()),
                operating_system_version: Some("1".into()),
                network_prefix: None,
                is_enabled: Some(true),
                observed_at,
            }],
        }],
    }
}

async fn current_entity_name(pool: &PgPool, collection_key: &str) -> Result<String, String> {
    sqlx::query_scalar(
        r#"SELECT entity.name
           FROM ad_entities entity
           JOIN ad_domains domain ON domain.id = entity.domain_id
           WHERE domain.collection_key = $1"#,
    )
    .bind(collection_key)
    .fetch_one(pool)
    .await
    .map_err(|error| error.to_string())
}
