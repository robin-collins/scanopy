//! Generic credential resolution and summarization for discovery integrations.

use std::collections::HashMap;
use std::net::IpAddr;
use uuid::Uuid;

use crate::server::credentials::r#impl::mapping::{
    CredentialMapping, CredentialQueryPayload, CredentialQueryPayloadDiscriminants,
};
use crate::server::hosts::r#impl::base::Host;

/// Resolve applicable credentials for a target IP from credential mappings.
///
/// Returns credentials in specificity order: IP-specific override first,
/// then network default as fallback. Each entry includes the credential
/// and its optional server-side ID (for auto-assignment tracking).
pub fn resolve_credentials_for_ip(
    mapping: &CredentialMapping<CredentialQueryPayload>,
    ip: IpAddr,
) -> Vec<(&CredentialQueryPayload, Option<Uuid>)> {
    let mut creds = Vec::new();

    // IP override first (most specific)
    if let Some(o) = mapping.ip_overrides.iter().find(|o| o.ip == ip) {
        let cred_id = if o.credential_id != Uuid::nil() {
            Some(o.credential_id)
        } else {
            None
        };
        creds.push((&o.credential, cred_id));
    }

    // Network default as fallback (only if no override matched)
    if creds.is_empty()
        && let Some(default) = &mapping.default_credential
    {
        creds.push((default, None));
    }

    creds
}

/// Summarize credential assignments across discovered hosts, grouped by credential type.
///
/// Builds a credential_id → type lookup from the credential mappings, then groups
/// each host's assignments by type label. Returns type_label → list of "cred_id → ip".
pub fn summarize_credential_assignments(
    hosts: &[(IpAddr, Host)],
    credential_mappings: &[CredentialMapping<CredentialQueryPayload>],
) -> HashMap<String, Vec<String>> {
    // Build credential_id → type label lookup from mappings
    let mut cred_type_lookup: HashMap<Uuid, String> = HashMap::new();
    for mapping in credential_mappings {
        let type_label: Option<String> = mapping
            .default_credential
            .as_ref()
            .map(|c| {
                let d: CredentialQueryPayloadDiscriminants = c.into();
                d.to_string()
            })
            .or_else(|| {
                mapping.ip_overrides.first().map(|o| {
                    let d: CredentialQueryPayloadDiscriminants = (&o.credential).into();
                    d.to_string()
                })
            });

        if let Some(label) = type_label {
            for o in &mapping.ip_overrides {
                if o.credential_id != Uuid::nil() {
                    cred_type_lookup.insert(o.credential_id, label.clone());
                }
            }
        }
    }

    // Group assignments by type
    let mut by_type: HashMap<String, Vec<String>> = HashMap::new();
    for (ip, host) in hosts {
        for assignment in &host.base.credential_assignments {
            let label = cred_type_lookup
                .get(&assignment.credential_id)
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string());
            by_type
                .entry(label)
                .or_default()
                .push(format!("{} → {}", assignment.credential_id, ip));
        }
    }

    by_type
}
