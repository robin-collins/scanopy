use std::{
    collections::{HashMap, VecDeque},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use sha2::{Digest, Sha256};
use tokio::{sync::mpsc, task::JoinHandle};

use crate::{
    daemon::runtime::service::DaemonRuntimeService,
    server::passive::types::{
        MAX_OBSERVATIONS_PER_BATCH, PassiveIngestRequest, PassiveIngestResponse,
        PassiveObservationInput,
    },
};

use super::capture::spawn_capture_tasks;
#[cfg(target_os = "linux")]
use super::parsers::{parse_ip_neighbor_json, parse_proc_arp};

const QUEUE_CAPACITY: usize = 512;
const DEDUPE_CAPACITY: usize = 4096;
const DEDUPE_WINDOW: Duration = Duration::from_secs(10 * 60);
const FLUSH_INTERVAL: Duration = Duration::from_secs(5);

pub struct PassiveRuntime {
    running: Arc<AtomicBool>,
    task: JoinHandle<()>,
}

impl PassiveRuntime {
    pub async fn shutdown(self) {
        self.running.store(false, Ordering::Relaxed);
        let _ = tokio::time::timeout(Duration::from_secs(10), self.task).await;
    }
}

pub async fn spawn_passive_runtime(
    runtime: Arc<DaemonRuntimeService>,
) -> anyhow::Result<PassiveRuntime> {
    let selected_interfaces = runtime.config.get_interfaces().await.unwrap_or_default();
    let running = Arc::new(AtomicBool::new(true));
    let task_running = running.clone();
    let (sender, mut receiver) = mpsc::channel::<PassiveObservationInput>(QUEUE_CAPACITY);
    spawn_capture_tasks(&selected_interfaces, sender.clone(), running.clone());

    let task = tokio::spawn(async move {
        let mut batch = Vec::with_capacity(MAX_OBSERVATIONS_PER_BATCH);
        let mut dedupe = DedupeWindow::default();
        let mut flush = tokio::time::interval(FLUSH_INTERVAL);
        let mut neighbor_poll = tokio::time::interval(Duration::from_secs(5 * 60));
        flush.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        neighbor_poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        while task_running.load(Ordering::Relaxed) {
            tokio::select! {
                item = receiver.recv() => {
                    let Some(item) = item else { break; };
                    if batch.len() >= MAX_OBSERVATIONS_PER_BATCH {
                        flush_batch(&runtime, &mut batch).await;
                    }
                    if batch.len() < MAX_OBSERVATIONS_PER_BATCH
                        && item.validate().is_ok()
                        && !dedupe.is_duplicate(&item)
                    {
                        batch.push(item);
                    }
                }
                _ = neighbor_poll.tick() => {
                    #[cfg(target_os = "linux")]
                    {
                        let observations = match tokio::process::Command::new("ip")
                            .args(["-j", "neighbor", "show"])
                            .output()
                            .await
                        {
                            Ok(output) if output.status.success() => parse_ip_neighbor_json(&output.stdout).unwrap_or_default(),
                            _ => tokio::fs::read_to_string("/proc/net/arp").await
                                .map(|contents| parse_proc_arp(&contents))
                                .unwrap_or_default(),
                        };
                        for item in observations {
                            if item.validate().is_ok()
                                && !dedupe.is_duplicate(&item)
                                && batch.len() < MAX_OBSERVATIONS_PER_BATCH
                            {
                                batch.push(item);
                            }
                        }
                    }
                }
                _ = flush.tick() => flush_batch(&runtime, &mut batch).await,
            }
        }
        flush_batch(&runtime, &mut batch).await;
        tracing::info!("Passive observation runtime stopped");
    });
    Ok(PassiveRuntime { running, task })
}

async fn flush_batch(runtime: &DaemonRuntimeService, batch: &mut Vec<PassiveObservationInput>) {
    if batch.is_empty() {
        return;
    }
    let Ok(Some(network_id)) = runtime.config.get_network_id().await else {
        // Initial provisioning may complete after daemon startup. Keep one
        // bounded batch and publish it once network identity becomes available.
        return;
    };
    let observations = std::mem::take(batch);
    let request = PassiveIngestRequest {
        network_id,
        observations,
    };
    match runtime
        .api_client
        .post::<_, PassiveIngestResponse>(
            "/api/v1/passive/observations",
            &request,
            "Failed to publish passive observations",
        )
        .await
    {
        Ok(response) => tracing::debug!(
            accepted = response.accepted,
            duplicates = response.duplicates,
            "Published passive observations"
        ),
        Err(error) => {
            tracing::warn!(error = %error, count = request.observations.len(), "Passive observation batch failed; collectors remain active");
            // Bound retry memory to one batch. New facts take precedence after
            // repeated outages so passive capture can never grow without limit.
            if batch.is_empty() {
                *batch = request.observations;
            }
        }
    }
}

#[derive(Default)]
struct DedupeWindow {
    expiries: HashMap<[u8; 32], Instant>,
    order: VecDeque<([u8; 32], Instant)>,
}

impl DedupeWindow {
    fn is_duplicate(&mut self, observation: &PassiveObservationInput) -> bool {
        let now = Instant::now();
        while self.order.front().is_some_and(|(_, expiry)| *expiry <= now) {
            if let Some((fingerprint, expiry)) = self.order.pop_front()
                && self.expiries.get(&fingerprint) == Some(&expiry)
            {
                self.expiries.remove(&fingerprint);
            }
        }
        let fact =
            serde_json::to_vec(&(&observation.source, &observation.fact)).unwrap_or_default();
        let fingerprint: [u8; 32] = Sha256::digest(fact).into();
        if self
            .expiries
            .get(&fingerprint)
            .is_some_and(|expiry| *expiry > now)
        {
            return true;
        }
        let expiry = now + DEDUPE_WINDOW;
        self.expiries.insert(fingerprint, expiry);
        self.order.push_back((fingerprint, expiry));
        while self.expiries.len() > DEDUPE_CAPACITY {
            if let Some((oldest, expiry)) = self.order.pop_front()
                && self.expiries.get(&oldest) == Some(&expiry)
            {
                self.expiries.remove(&oldest);
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::passive::types::{NeighborState, PassiveFact, PassiveSource};
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn dedupe_ignores_wire_ids_and_timestamps() {
        let make = || PassiveObservationInput {
            observation_id: Uuid::new_v4(),
            source: PassiveSource::KernelNeighbor,
            confidence: 80,
            observed_at: Utc::now(),
            expires_at: None,
            fact: PassiveFact::NeighborMapping {
                address: "192.0.2.1".parse().unwrap(),
                mac_address: None,
                interface: "eth0".into(),
                state: NeighborState::Incomplete,
            },
        };
        let mut dedupe = DedupeWindow::default();
        assert!(!dedupe.is_duplicate(&make()));
        assert!(dedupe.is_duplicate(&make()));
    }
}
