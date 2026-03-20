//! Event subscriber for credential seed_ips cleanup.
//!
//! Subscribes to terminal discovery events (Complete, Failed, Cancelled)
//! and clears seed_ips on credentials that were used in the session.
//! seed_ips are ephemeral bootstrap data — once a discovery session has
//! used them (regardless of outcome), they've served their purpose.

use async_trait::async_trait;

use crate::daemon::discovery::types::base::DiscoveryPhase;
use crate::server::credentials::service::CredentialService;
use crate::server::shared::events::bus::{EventFilter, EventSubscriber};
use crate::server::shared::events::types::Event;
use crate::server::shared::services::traits::CrudService;

#[async_trait]
impl EventSubscriber for CredentialService {
    fn event_filter(&self) -> EventFilter {
        EventFilter::discovery_only(Some(vec![
            DiscoveryPhase::Complete,
            DiscoveryPhase::Failed,
            DiscoveryPhase::Cancelled,
        ]))
    }

    async fn handle_events(&self, events: Vec<Event>) -> Result<(), anyhow::Error> {
        for event in events {
            if let Event::Discovery(discovery_event) = event {
                if discovery_event.credential_ids.is_empty() {
                    continue;
                }

                for cred_id in &discovery_event.credential_ids {
                    if let Ok(Some(cred)) = self.get_by_id(cred_id).await
                        && cred.base.seed_ips.is_some()
                    {
                        if let Err(e) = self.clear_seed_ips(cred_id).await {
                            tracing::warn!(
                                credential_id = %cred_id,
                                session_id = %discovery_event.session_id,
                                phase = %discovery_event.phase,
                                error = ?e,
                                "Failed to clear seed_ips after discovery"
                            );
                        } else {
                            tracing::info!(
                                credential_id = %cred_id,
                                session_id = %discovery_event.session_id,
                                phase = %discovery_event.phase,
                                "Cleared seed_ips on credential after discovery"
                            );
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "credential-seed-ips-cleanup"
    }
}
