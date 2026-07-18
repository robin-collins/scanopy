use crate::server::{networks::service::NetworkService, shared::services::traits::CrudService};
use backon::{ExponentialBuilder, Retryable};
use serde_json::{Map, Value, json};
use std::{sync::Arc, time::Duration};
use uuid::Uuid;

pub struct PosthogService {
    client: reqwest::Client,
    api_key: String,
    capture_url: String,
    network_service: Arc<NetworkService>,
}

impl PosthogService {
    pub async fn new(
        api_key: String,
        api_host: String,
        network_service: Arc<NetworkService>,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("PostHog HTTP client configuration is valid");
        Self {
            client,
            api_key,
            capture_url: format!("{}/i/v0/e/", api_host.trim_end_matches('/')),
            network_service,
        }
    }

    pub async fn capture(
        &self,
        event_name: &str,
        distinct_id: &str,
        properties: serde_json::Value,
    ) {
        let event_name_owned = event_name.to_string();
        let distinct_id_owned = distinct_id.to_string();
        let payload = build_posthog_payload(
            &self.api_key,
            &event_name_owned,
            &distinct_id_owned,
            properties,
        );

        if let Err(e) = (|| self.send_payload(&payload))
            .retry(
                ExponentialBuilder::default()
                    .with_min_delay(Duration::from_millis(100))
                    .with_max_delay(Duration::from_millis(500))
                    .with_max_times(2),
            )
            .await
        {
            tracing::warn!(event = %event_name_owned, error = %e, "Failed to send event to PostHog");
        }
    }

    /// Send a $identify event to set person properties in PostHog.
    pub async fn identify(&self, distinct_id: &str, properties: serde_json::Value) {
        self.capture("$identify", distinct_id, json!({ "$set": properties }))
            .await;
    }

    /// Send a $groupidentify event to set group properties in PostHog.
    pub async fn group_identify(
        &self,
        group_type: &str,
        group_key: &str,
        properties: serde_json::Value,
    ) {
        self.capture(
            "$groupidentify",
            &format!("group:{group_key}"),
            json!({
                "$group_type": group_type,
                "$group_key": group_key,
                "$group_set": properties
            }),
        )
        .await;
    }

    async fn send_payload(&self, payload: &Value) -> reqwest::Result<()> {
        self.client
            .post(&self.capture_url)
            .json(payload)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn get_org_id_from_network(&self, network_id: &Uuid) -> Option<Uuid> {
        if let Ok(Some(network)) = self.network_service.get_by_id(network_id).await {
            Some(network.base.organization_id)
        } else {
            None
        }
    }
}

fn build_posthog_payload(
    api_key: &str,
    event_name: &str,
    distinct_id: &str,
    properties: Value,
) -> Value {
    let mut properties = properties.as_object().cloned().unwrap_or_default();
    properties
        .entry("$lib".to_string())
        .or_insert_with(|| Value::String("scanopy-server".to_string()));
    properties
        .entry("$lib_version".to_string())
        .or_insert_with(|| Value::String(env!("CARGO_PKG_VERSION").to_string()));
    if properties.contains_key("$groups") {
        properties.insert("$process_person_profile".to_string(), Value::Bool(true));
    }

    let mut payload = Map::new();
    payload.insert("api_key".to_string(), Value::String(api_key.to_string()));
    payload.insert(
        "uuid".to_string(),
        Value::String(Uuid::new_v4().to_string()),
    );
    payload.insert("event".to_string(), Value::String(event_name.to_string()));
    payload.insert(
        "$distinct_id".to_string(),
        Value::String(distinct_id.to_string()),
    );
    payload.insert("properties".to_string(), Value::Object(properties));
    payload.insert("timestamp".to_string(), Value::Null);
    Value::Object(payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_preserves_groups_and_uses_the_capture_contract() {
        let payload = build_posthog_payload(
            "project-key",
            "discovery_completed",
            "user-id",
            json!({
                "$groups": { "organization": "org-id" },
                "network_count": 2
            }),
        );

        assert_eq!(payload["api_key"], "project-key");
        assert_eq!(payload["event"], "discovery_completed");
        assert_eq!(payload["$distinct_id"], "user-id");
        assert_eq!(payload["properties"]["network_count"], 2);
        assert_eq!(payload["properties"]["$lib"], "scanopy-server");
        assert_eq!(payload["properties"]["$process_person_profile"], true);
        assert!(Uuid::parse_str(payload["uuid"].as_str().unwrap()).is_ok());
        assert!(payload["timestamp"].is_null());
    }
}
