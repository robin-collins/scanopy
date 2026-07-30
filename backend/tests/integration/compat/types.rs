//! Shared types for compatibility testing.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// A captured HTTP request/response exchange.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedExchange {
    pub method: String,
    pub path: String,
    pub request_body: serde_json::Value,
    pub response_status: u16,
    pub response_body: serde_json::Value,
}

/// A manifest of captured exchanges for a specific version.
#[derive(Debug, Serialize, Deserialize)]
pub struct FixtureManifest {
    pub version: String,
    pub exchanges: Vec<CapturedExchange>,
}

const FIXTURES_DIR: &str = "tests/integration/compat/fixtures";

/// Load all fixture versions that have the specified manifest file.
pub fn get_fixture_versions(manifest_name: &str) -> Vec<String> {
    let dir = Path::new(FIXTURES_DIR);

    if !dir.exists() {
        return Vec::new();
    }

    fs::read_dir(dir)
        .expect("Failed to read fixtures directory")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name().into_string().ok()?;

            if name.starts_with('v') && entry.path().is_dir() {
                let manifest_path = entry.path().join(manifest_name);
                if manifest_path.exists() {
                    Some(name.trim_start_matches('v').to_string())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect()
}

/// Load a fixture manifest for a specific version.
pub fn load_manifest(version: &str, manifest_name: &str) -> Option<FixtureManifest> {
    let path = Path::new(FIXTURES_DIR)
        .join(format!("v{}", version))
        .join(manifest_name);

    if !path.exists() {
        return None;
    }

    let content = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Load the OpenAPI spec for a specific version.
pub fn load_openapi_spec(version: &str) -> Option<serde_json::Value> {
    let path = Path::new(FIXTURES_DIR)
        .join(format!("v{}", version))
        .join("openapi.json");

    if !path.exists() {
        return None;
    }

    let content = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use scanopy::{
        daemon::runtime::state::DaemonStatus,
        server::daemons::r#impl::api::{DaemonRegistrationRequest, DaemonRegistrationResponse},
    };

    #[test]
    fn v0_18_wire_fixtures_default_missing_feature_flags() {
        let daemon_to_server = load_manifest("0.18.0", "daemon_to_server.json")
            .expect("v0.18.0 daemon-to-server fixture must remain available");

        let registration = daemon_to_server
            .exchanges
            .iter()
            .find(|exchange| exchange.path == "/api/daemons/register")
            .expect("v0.18.0 registration exchange");
        assert!(registration.request_body.get("feature_flags").is_none());
        let request: DaemonRegistrationRequest =
            serde_json::from_value(registration.request_body.clone())
                .expect("current server must accept a v0.18 registration");
        assert!(request.feature_flags.is_empty());

        let response: DaemonRegistrationResponse =
            serde_json::from_value(registration.response_body["data"].clone())
                .expect("current daemon must accept a v0.18 registration response");
        assert!(response.daemon.base.feature_flags.is_empty());

        let request_work = daemon_to_server
            .exchanges
            .iter()
            .find(|exchange| exchange.path.ends_with("/request-work"))
            .expect("v0.18.0 request-work exchange");
        assert!(request_work.request_body.get("feature_flags").is_none());
        let status: DaemonStatus = serde_json::from_value(request_work.request_body.clone())
            .expect("current server must accept a v0.18 daemon status");
        assert!(status.feature_flags.is_empty());

        let server_to_daemon = load_manifest("0.18.0", "server_to_daemon.json")
            .expect("v0.18.0 server-to-daemon fixture must remain available");
        let first_contact = server_to_daemon
            .exchanges
            .iter()
            .find(|exchange| exchange.path == "/api/first-contact")
            .expect("v0.18.0 first-contact exchange");
        assert!(
            first_contact.response_body["data"]
                .get("feature_flags")
                .is_none()
        );
        let status: DaemonStatus =
            serde_json::from_value(first_contact.response_body["data"].clone())
                .expect("current daemon must accept a v0.18 server status");
        assert!(status.feature_flags.is_empty());
    }
}
