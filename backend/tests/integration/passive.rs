use chrono::{Duration, Utc};
use reqwest::StatusCode;
use scanopy::server::{
    daemon_api_keys::r#impl::{
        api::DaemonApiKeyResponse,
        base::{DaemonApiKey, DaemonApiKeyBase},
    },
    passive::{
        storage,
        types::{
            MAX_STORED_OBSERVATIONS_PER_DAEMON, NeighborState, PassiveFact, PassiveIngestRequest,
            PassiveListQuery, PassiveObservationInput, PassiveSource,
        },
    },
    shared::storage::traits::Storable,
};
use sqlx::Row;
use uuid::Uuid;

use crate::infra::{BASE_URL, TestContext};

pub async fn run_storage_tests(ctx: &TestContext) -> Result<(), String> {
    let daemon_id: Uuid =
        sqlx::query("SELECT daemon_id FROM discovery WHERE network_id = $1 LIMIT 1")
            .bind(ctx.network_id)
            .fetch_one(&ctx.db_pool)
            .await
            .map_err(|error| error.to_string())?
            .get("daemon_id");

    assert_cross_network_ingest_forbidden(ctx, daemon_id).await?;

    let current = observation(Utc::now(), "192.0.2.241");
    let request = PassiveIngestRequest {
        network_id: ctx.network_id,
        observations: vec![current.clone()],
    };
    request.validate().map_err(|error| error.to_string())?;
    let first = storage::ingest(&ctx.db_pool, ctx.network_id, daemon_id, &request)
        .await
        .map_err(|error| error.to_string())?;
    assert_eq!((first.accepted, first.duplicates), (1, 0));

    let duplicate = storage::ingest(&ctx.db_pool, ctx.network_id, daemon_id, &request)
        .await
        .map_err(|error| error.to_string())?;
    assert_eq!((duplicate.accepted, duplicate.duplicates), (0, 1));

    let mut refreshed = current.clone();
    refreshed.observation_id = Uuid::new_v4();
    refreshed.observed_at += Duration::seconds(1);
    refreshed.expires_at = Some(refreshed.observed_at + Duration::minutes(30));
    let refresh = storage::ingest(
        &ctx.db_pool,
        ctx.network_id,
        daemon_id,
        &PassiveIngestRequest {
            network_id: ctx.network_id,
            observations: vec![refreshed.clone()],
        },
    )
    .await
    .map_err(|error| error.to_string())?;
    assert_eq!((refresh.accepted, refresh.duplicates), (0, 1));
    let current_fact_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM passive_observations WHERE network_id = $1 \
         AND daemon_id = $2 AND source = 'arp'",
    )
    .bind(ctx.network_id)
    .bind(daemon_id)
    .fetch_one(&ctx.db_pool)
    .await
    .map_err(|error| error.to_string())?;
    assert_eq!(current_fact_rows, 1);

    let hidden_expired_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO passive_observations \
            (id, network_id, daemon_id, source, observation_key, confidence, correlation_kind, \
             correlation_key, fact, observed_at, expires_at) \
         VALUES ($1, $2, $3, 'arp', $4, 70, 'observation', $5, \
             jsonb_build_object('kind', 'neighbor_mapping', 'address', '192.0.2.248', \
                 'mac_address', NULL, 'interface', 'expired-list-test', 'state', 'reachable'), \
             NOW() - INTERVAL '2 minutes', NOW() - INTERVAL '1 minute')",
    )
    .bind(hidden_expired_id)
    .bind(ctx.network_id)
    .bind(daemon_id)
    .bind(format!("expired-{hidden_expired_id}"))
    .bind(format!("expired-{hidden_expired_id}"))
    .execute(&ctx.db_pool)
    .await
    .map_err(|error| error.to_string())?;

    let query = PassiveListQuery {
        network_id: Some(ctx.network_id),
        source: Some("arp".into()),
        limit: Some(500),
        offset: Some(0),
    };
    let (listed, _) = storage::list(&ctx.db_pool, &[ctx.network_id], &query)
        .await
        .map_err(|error| error.to_string())?;
    assert!(listed.iter().any(|item| item.id == current.observation_id));
    assert!(!listed.iter().any(|item| item.id == hidden_expired_id));
    let (forbidden, total) = storage::list(&ctx.db_pool, &[], &query)
        .await
        .map_err(|error| error.to_string())?;
    assert!(forbidden.is_empty());
    assert_eq!(total, 0);

    let expired = observation(Utc::now() - Duration::days(31), "192.0.2.242");
    storage::ingest(
        &ctx.db_pool,
        ctx.network_id,
        daemon_id,
        &PassiveIngestRequest {
            network_id: ctx.network_id,
            observations: vec![expired.clone()],
        },
    )
    .await
    .map_err(|error| error.to_string())?;
    let expired_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM passive_observations WHERE id = $1")
            .bind(expired.observation_id)
            .fetch_one(&ctx.db_pool)
            .await
            .map_err(|error| error.to_string())?;
    assert_eq!(expired_count, 0);

    let correlation_key: String =
        sqlx::query_scalar("SELECT correlation_key FROM passive_observations WHERE id = $1")
            .bind(current.observation_id)
            .fetch_one(&ctx.db_pool)
            .await
            .map_err(|error| error.to_string())?;
    sqlx::query("DELETE FROM passive_observations WHERE id = $1")
        .bind(current.observation_id)
        .execute(&ctx.db_pool)
        .await
        .map_err(|error| error.to_string())?;
    storage::cleanup_expired(&ctx.db_pool)
        .await
        .map_err(|error| error.to_string())?;
    let correlation_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM passive_correlations WHERE network_id = $1 AND correlation_key = $2",
    )
    .bind(ctx.network_id)
    .bind(correlation_key)
    .fetch_one(&ctx.db_pool)
    .await
    .map_err(|error| error.to_string())?;
    assert_eq!(correlation_count, 0);

    sqlx::query(
        "INSERT INTO passive_observations \
            (id, network_id, daemon_id, source, observation_key, confidence, correlation_kind, \
             correlation_key, fact, observed_at, expires_at) \
         SELECT ('00000000-0000-0000-0000-' || lpad(to_hex(value), 12, '0'))::uuid, \
                $1, $2, 'arp', 'cap-seed-' || value, 70, 'seed', 'seed-' || value, \
                jsonb_build_object('kind', 'neighbor_mapping', 'address', '192.0.2.250', \
                    'mac_address', NULL, 'interface', 'cap-seed', 'state', 'reachable'), \
                NOW(), NOW() + INTERVAL '30 minutes' \
         FROM generate_series(1, 9900) value",
    )
    .bind(ctx.network_id)
    .bind(daemon_id)
    .execute(&ctx.db_pool)
    .await
    .map_err(|error| error.to_string())?;
    let left = cap_batch(ctx.network_id, "left");
    let right = cap_batch(ctx.network_id, "right");
    let (left_result, right_result) = tokio::join!(
        storage::ingest(&ctx.db_pool, ctx.network_id, daemon_id, &left),
        storage::ingest(&ctx.db_pool, ctx.network_id, daemon_id, &right),
    );
    left_result.map_err(|error| error.to_string())?;
    right_result.map_err(|error| error.to_string())?;
    let capped_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM passive_observations WHERE network_id = $1 AND daemon_id = $2",
    )
    .bind(ctx.network_id)
    .bind(daemon_id)
    .fetch_one(&ctx.db_pool)
    .await
    .map_err(|error| error.to_string())?;
    assert_eq!(capped_count, MAX_STORED_OBSERVATIONS_PER_DAEMON);
    sqlx::query("DELETE FROM passive_observations WHERE network_id = $1 AND daemon_id = $2")
        .bind(ctx.network_id)
        .bind(daemon_id)
        .execute(&ctx.db_pool)
        .await
        .map_err(|error| error.to_string())?;
    storage::cleanup_expired(&ctx.db_pool)
        .await
        .map_err(|error| error.to_string())?;
    Ok(())
}

async fn assert_cross_network_ingest_forbidden(
    ctx: &TestContext,
    daemon_id: Uuid,
) -> Result<(), String> {
    let api_key = DaemonApiKey::new(DaemonApiKeyBase {
        key: String::new(),
        name: "Passive cross-network authorization test".to_string(),
        last_used: None,
        expires_at: None,
        network_id: ctx.network_id,
        is_enabled: true,
        tags: Vec::new(),
        plaintext: None,
    });
    let created: DaemonApiKeyResponse = ctx.client.post("/api/v1/auth/daemon", &api_key).await?;

    let foreign_network_id = Uuid::new_v4();
    let rejected_observation = observation(Utc::now(), "192.0.2.247");
    let response = reqwest::Client::new()
        .post(format!("{BASE_URL}/api/v1/passive/observations"))
        .header("Authorization", format!("Bearer {}", created.key))
        .header("X-Daemon-ID", daemon_id.to_string())
        .json(&PassiveIngestRequest {
            network_id: foreign_network_id,
            observations: vec![rejected_observation.clone()],
        })
        .send()
        .await
        .map_err(|error| format!("passive cross-network request failed: {error}"))?;

    let status = response.status();
    let response_body = response.text().await.unwrap_or_default();
    if status != StatusCode::FORBIDDEN {
        return Err(format!(
            "expected passive cross-network ingestion to return 403, got {status}: {response_body}"
        ));
    }

    let persisted: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM passive_observations WHERE id = $1")
            .bind(rejected_observation.observation_id)
            .fetch_one(&ctx.db_pool)
            .await
            .map_err(|error| error.to_string())?;
    if persisted != 0 {
        return Err("cross-network passive observation was persisted".to_string());
    }

    ctx.client
        .delete_no_content(&format!("/api/v1/auth/daemon/{}", created.api_key.id))
        .await?;

    Ok(())
}

fn cap_batch(network_id: Uuid, prefix: &str) -> PassiveIngestRequest {
    let observed_at = Utc::now();
    PassiveIngestRequest {
        network_id,
        observations: (0..200)
            .map(|index| PassiveObservationInput {
                observation_id: Uuid::new_v4(),
                source: PassiveSource::Arp,
                confidence: 70,
                observed_at,
                expires_at: Some(observed_at + Duration::minutes(30)),
                fact: PassiveFact::NeighborMapping {
                    address: "192.0.2.249".parse().unwrap(),
                    mac_address: Some("02:00:00:00:00:42".parse().unwrap()),
                    interface: format!("cap-{prefix}-{index}"),
                    state: NeighborState::Reachable,
                },
            })
            .collect(),
    }
}

fn observation(observed_at: chrono::DateTime<Utc>, address: &str) -> PassiveObservationInput {
    PassiveObservationInput {
        observation_id: Uuid::new_v4(),
        source: PassiveSource::Arp,
        confidence: 70,
        observed_at,
        expires_at: Some(observed_at + Duration::minutes(30)),
        fact: PassiveFact::NeighborMapping {
            address: address.parse().unwrap(),
            mac_address: Some("02:00:00:00:00:41".parse().unwrap()),
            interface: "integration-test".into(),
            state: NeighborState::Reachable,
        },
    }
}
