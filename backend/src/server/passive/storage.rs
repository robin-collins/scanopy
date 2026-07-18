use anyhow::Context;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use super::types::{
    DEFAULT_RETENTION_DAYS, MAX_STORED_OBSERVATIONS_PER_DAEMON, PassiveFact, PassiveIngestRequest,
    PassiveIngestResponse, PassiveListQuery, PassiveObservation,
};

pub async fn ingest(
    pool: &PgPool,
    network_id: Uuid,
    daemon_id: Uuid,
    request: &PassiveIngestRequest,
) -> anyhow::Result<PassiveIngestResponse> {
    let mut transaction = pool.begin().await?;
    // Serialize each daemon/network writer so the hard row ceiling remains a
    // true ceiling even when retry and live batches overlap.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!("scanopy-passive:{network_id}:{daemon_id}"))
        .execute(&mut *transaction)
        .await
        .context("failed to lock passive observation scope")?;
    let mut accepted = 0u32;
    let mut duplicates = 0u32;

    for observation in &request.observations {
        let (correlation_kind, correlation_key) =
            correlation(observation.observation_id, &observation.fact);
        let observation_key = observation_key(&observation.source, &observation.fact)?;
        let fact = serde_json::to_value(&observation.fact)?;
        let inserted = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO passive_observations
                (id, network_id, daemon_id, source, observation_key, confidence,
                 correlation_kind, correlation_key, fact, observed_at, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT DO NOTHING
            RETURNING id
            "#,
        )
        .bind(observation.observation_id)
        .bind(network_id)
        .bind(daemon_id)
        .bind(observation.source.to_string())
        .bind(&observation_key)
        .bind(i64::from(observation.confidence))
        .bind(correlation_kind)
        .bind(&correlation_key)
        .bind(fact)
        .bind(observation.observed_at)
        .bind(observation.expires_at)
        .fetch_optional(&mut *transaction)
        .await
        .context("failed to persist passive observation")?;

        if inserted.is_none() {
            sqlx::query(
                r#"
                UPDATE passive_observations SET
                    confidence = $1,
                    fact = $2,
                    observed_at = GREATEST(observed_at, $3),
                    expires_at = CASE WHEN $3 >= observed_at THEN $4 ELSE expires_at END
                WHERE network_id = $5 AND daemon_id = $6 AND source = $7
                  AND observation_key = $8
                "#,
            )
            .bind(i64::from(observation.confidence))
            .bind(serde_json::to_value(&observation.fact)?)
            .bind(observation.observed_at)
            .bind(observation.expires_at)
            .bind(network_id)
            .bind(daemon_id)
            .bind(observation.source.to_string())
            .bind(&observation_key)
            .execute(&mut *transaction)
            .await
            .context("failed to refresh passive observation")?;
            sqlx::query(
                "UPDATE passive_correlations SET last_seen_at = GREATEST(last_seen_at, $1) \
                 WHERE network_id = $2 AND correlation_kind = $3 AND correlation_key = $4",
            )
            .bind(observation.observed_at)
            .bind(network_id)
            .bind(correlation_kind)
            .bind(&correlation_key)
            .execute(&mut *transaction)
            .await
            .context("failed to refresh passive correlation")?;
            duplicates += 1;
            continue;
        }
        accepted += 1;
        sqlx::query(
            r#"
            INSERT INTO passive_correlations
                (network_id, correlation_kind, correlation_key, first_seen_at,
                 last_seen_at, observation_count, sources)
            VALUES ($1, $2, $3, $4, $4, 1, ARRAY[$5]::TEXT[])
            ON CONFLICT (network_id, correlation_kind, correlation_key) DO UPDATE SET
                first_seen_at = LEAST(passive_correlations.first_seen_at, EXCLUDED.first_seen_at),
                last_seen_at = GREATEST(passive_correlations.last_seen_at, EXCLUDED.last_seen_at),
                observation_count = passive_correlations.observation_count + 1,
                sources = CASE
                    WHEN $5 = ANY(passive_correlations.sources) THEN passive_correlations.sources
                    ELSE array_append(passive_correlations.sources, $5)
                END
            "#,
        )
        .bind(network_id)
        .bind(correlation_kind)
        .bind(correlation_key)
        .bind(observation.observed_at)
        .bind(observation.source.to_string())
        .execute(&mut *transaction)
        .await
        .context("failed to update passive correlation")?;
    }

    // Opportunistic bounded-retention cleanup keeps this independent of the
    // discovery scheduler. Stable Scanopy inventory is intentionally untouched.
    sqlx::query(
        "DELETE FROM passive_observations WHERE network_id = $1 AND \
         (observed_at < NOW() - make_interval(days => $2) OR expires_at < NOW())",
    )
    .bind(network_id)
    .bind(i32::try_from(DEFAULT_RETENTION_DAYS).unwrap_or(30))
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "DELETE FROM passive_correlations WHERE network_id = $1 AND \
         (last_seen_at < NOW() - make_interval(days => $2) OR NOT EXISTS (\
             SELECT 1 FROM passive_observations observation \
             WHERE observation.network_id = passive_correlations.network_id \
               AND observation.correlation_kind = passive_correlations.correlation_kind \
               AND observation.correlation_key = passive_correlations.correlation_key \
               AND (observation.expires_at IS NULL OR observation.expires_at >= NOW())\
         ))",
    )
    .bind(network_id)
    .bind(i32::try_from(DEFAULT_RETENTION_DAYS).unwrap_or(30))
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        "DELETE FROM passive_observations WHERE id IN (\
            SELECT id FROM passive_observations \
            WHERE network_id = $1 AND daemon_id = $2 \
            ORDER BY observed_at DESC, id DESC OFFSET $3\
        )",
    )
    .bind(network_id)
    .bind(daemon_id)
    .bind(MAX_STORED_OBSERVATIONS_PER_DAEMON)
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "DELETE FROM passive_correlations correlation WHERE network_id = $1 AND NOT EXISTS (\
            SELECT 1 FROM passive_observations observation \
            WHERE observation.network_id = correlation.network_id \
              AND observation.correlation_kind = correlation.correlation_kind \
              AND observation.correlation_key = correlation.correlation_key \
              AND (observation.expires_at IS NULL OR observation.expires_at >= NOW())\
        )",
    )
    .bind(network_id)
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;
    Ok(PassiveIngestResponse {
        accepted,
        duplicates,
    })
}

pub async fn list(
    pool: &PgPool,
    network_ids: &[Uuid],
    query: &PassiveListQuery,
) -> anyhow::Result<(Vec<PassiveObservation>, u64)> {
    let allowed: Vec<Uuid> = match query.network_id {
        Some(id) if network_ids.contains(&id) => vec![id],
        Some(_) => vec![],
        None => network_ids.to_vec(),
    };
    if allowed.is_empty() {
        return Ok((vec![], 0));
    }

    let mut count = QueryBuilder::<Postgres>::new(
        "SELECT COUNT(*) FROM passive_observations WHERE network_id = ANY(",
    );
    count
        .push_bind(&allowed)
        .push(") AND (expires_at IS NULL OR expires_at >= NOW())");
    if let Some(source) = &query.source {
        count.push(" AND source = ").push_bind(source);
    }
    let total: i64 = count.build_query_scalar().fetch_one(pool).await?;

    let (limit, offset) = query.pagination();
    let mut data = QueryBuilder::<Postgres>::new(
        "SELECT id, network_id, daemon_id, source, confidence, correlation_kind, \
         correlation_key, fact, observed_at, expires_at, created_at FROM passive_observations WHERE network_id = ANY(",
    );
    data.push_bind(&allowed)
        .push(") AND (expires_at IS NULL OR expires_at >= NOW())");
    if let Some(source) = &query.source {
        data.push(" AND source = ").push_bind(source);
    }
    data.push(" ORDER BY observed_at DESC, id DESC LIMIT ")
        .push_bind(i64::from(limit))
        .push(" OFFSET ")
        .push_bind(i64::from(offset));
    let items = data
        .build_query_as::<PassiveObservation>()
        .fetch_all(pool)
        .await?;
    Ok((items, u64::try_from(total).unwrap_or(0)))
}

/// Server-owned retention sweep. This runs independently of daemon activity so
/// disabled/offline collectors cannot leave expired data behind indefinitely.
pub async fn cleanup_expired(pool: &PgPool) -> anyhow::Result<()> {
    let mut transaction = pool.begin().await?;
    sqlx::query(
        "DELETE FROM passive_observations WHERE \
         observed_at < NOW() - make_interval(days => $1) OR expires_at < NOW()",
    )
    .bind(i32::try_from(DEFAULT_RETENTION_DAYS).unwrap_or(30))
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "DELETE FROM passive_observations WHERE id IN (\
            SELECT id FROM (\
                SELECT id, ROW_NUMBER() OVER (\
                    PARTITION BY network_id, daemon_id \
                    ORDER BY observed_at DESC, id DESC\
                ) AS row_number FROM passive_observations\
            ) ranked WHERE row_number > $1\
        )",
    )
    .bind(MAX_STORED_OBSERVATIONS_PER_DAEMON)
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "DELETE FROM passive_correlations correlation WHERE \
         correlation.last_seen_at < NOW() - make_interval(days => $1) OR NOT EXISTS (\
            SELECT 1 FROM passive_observations observation \
            WHERE observation.network_id = correlation.network_id \
              AND observation.correlation_kind = correlation.correlation_kind \
              AND observation.correlation_key = correlation.correlation_key \
              AND (observation.expires_at IS NULL OR observation.expires_at >= NOW())\
         )",
    )
    .bind(i32::try_from(DEFAULT_RETENTION_DAYS).unwrap_or(30))
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(())
}

fn correlation(observation_id: Uuid, fact: &PassiveFact) -> (&'static str, String) {
    let (kind, material) = match fact {
        PassiveFact::MdnsService {
            service_type,
            instance,
            hostname,
            ..
        } => (
            "service_instance",
            format!(
                "{}|{}|{}",
                service_type.to_ascii_lowercase(),
                instance.to_ascii_lowercase(),
                hostname.as_deref().unwrap_or("").to_ascii_lowercase()
            ),
        ),
        PassiveFact::DhcpLease {
            client_mac: Some(mac),
            ..
        } => ("mac_endpoint", mac.to_string()),
        PassiveFact::NeighborMapping {
            mac_address: Some(mac),
            ..
        } => ("mac_endpoint", mac.to_string()),
        // Hostname and leased/reused IP alone are never stable device identity.
        _ => ("observation", observation_id.to_string()),
    };
    let digest = Sha256::digest(format!("{kind}\0{material}").as_bytes());
    (kind, hex::encode(digest))
}

fn observation_key(
    source: &super::types::PassiveSource,
    fact: &PassiveFact,
) -> anyhow::Result<String> {
    let material = serde_json::to_vec(&(source, fact))?;
    Ok(hex::encode(Sha256::digest(material)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::passive::types::{DhcpMessageType, NeighborState};
    use std::str::FromStr;

    #[test]
    fn correlation_uses_mac_but_not_reused_ip_or_hostname() {
        let mac = mac_address::MacAddress::from_str("02:00:00:00:00:01").unwrap();
        let first = PassiveFact::DhcpLease {
            message_type: DhcpMessageType::Ack,
            transaction_id: "12345678".into(),
            client_mac: Some(mac),
            assigned_address: Some("192.0.2.2".parse().unwrap()),
            requested_address: None,
            server_address: None,
            lease_seconds: None,
            hostname: Some("reused".into()),
            vendor_class: None,
            routers: vec![],
            dns_servers: vec![],
            domain_name: None,
        };
        let neighbor = PassiveFact::NeighborMapping {
            address: "192.0.2.99".parse().unwrap(),
            mac_address: Some(mac),
            interface: "eth0".into(),
            state: NeighborState::Reachable,
        };
        assert_eq!(
            correlation(Uuid::new_v4(), &first),
            correlation(Uuid::new_v4(), &neighbor)
        );

        let no_mac = PassiveFact::NeighborMapping {
            address: "192.0.2.2".parse().unwrap(),
            mac_address: None,
            interface: "eth0".into(),
            state: NeighborState::Incomplete,
        };
        assert_ne!(
            correlation(Uuid::new_v4(), &no_mac),
            correlation(Uuid::new_v4(), &no_mac)
        );
    }
}
