//! Generic credential resolution for discovery integrations.
//!
//! Provides a single function to resolve applicable credentials for a target IP
//! from credential mappings, in specificity order (IP override → network default).

use std::net::IpAddr;
use uuid::Uuid;

use crate::server::credentials::r#impl::mapping::{
    CredentialMapping, CredentialQueryPayload,
};

/// Resolve applicable credentials for a target IP from credential mappings.
///
/// Returns credentials in specificity order: IP-specific override first,
/// then network default as fallback. Each entry includes the credential
/// and its optional server-side ID (for auto-assignment tracking).
pub fn resolve_credentials_for_ip<'a>(
    mapping: &'a CredentialMapping<CredentialQueryPayload>,
    ip: IpAddr,
) -> Vec<(&'a CredentialQueryPayload, Option<Uuid>)> {
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
    if creds.is_empty() {
        if let Some(default) = &mapping.default_credential {
            creds.push((default, None));
        }
    }

    creds
}
