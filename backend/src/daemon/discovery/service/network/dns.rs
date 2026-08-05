use std::net::IpAddr;
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use anyhow::Error;
use tokio::sync::Semaphore;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

use super::NetworkScan;

const REVERSE_DNS_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_REVERSE_DNS_CONCURRENCY: usize = 16;
const MAX_HOSTNAME_LENGTH: usize = 253;
const MAX_DISPLAY_NAME_LENGTH: usize = 100;

// A timed-out spawn_blocking task keeps running until the platform resolver
// returns. Keeping the permit inside that task prevents slow DNS from growing
// an unbounded queue of detached resolver calls.
static REVERSE_DNS_SEMAPHORE: LazyLock<Arc<Semaphore>> =
    LazyLock::new(|| Arc::new(Semaphore::new(MAX_REVERSE_DNS_CONCURRENCY)));

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ResolvedHostname {
    pub hostname: String,
    pub display_name: String,
}

impl NetworkScan {
    pub(super) async fn get_hostname_for_ip(
        &self,
        ip: IpAddr,
        cancel: &CancellationToken,
    ) -> Result<Option<ResolvedHostname>, Error> {
        Ok(resolve_hostname_with(
            ip,
            cancel,
            REVERSE_DNS_TIMEOUT,
            REVERSE_DNS_SEMAPHORE.clone(),
            move |ip| dns_lookup::lookup_addr(&ip),
        )
        .await)
    }
}

async fn resolve_hostname_with<F, E>(
    ip: IpAddr,
    cancel: &CancellationToken,
    lookup_timeout: Duration,
    semaphore: Arc<Semaphore>,
    lookup: F,
) -> Option<ResolvedHostname>
where
    F: FnOnce(IpAddr) -> Result<String, E> + Send + 'static,
    E: Send + 'static,
{
    if ip.is_unspecified() || ip.is_multicast() {
        return None;
    }

    let lookup_future = async move {
        let permit = semaphore.acquire_owned().await.ok()?;
        tokio::task::spawn_blocking(move || {
            // The permit deliberately outlives the async JoinHandle if timeout
            // or cancellation detaches this blocking resolver operation.
            let _permit = permit;
            lookup(ip).ok().and_then(|value| normalize_hostname(&value))
        })
        .await
        .ok()
        .flatten()
    };

    tokio::select! {
        biased;
        _ = cancel.cancelled() => None,
        result = timeout(lookup_timeout, lookup_future) => result.ok().flatten(),
    }
}

fn normalize_hostname(value: &str) -> Option<ResolvedHostname> {
    let hostname = value.trim().trim_end_matches('.');
    if hostname.is_empty()
        || hostname.len() > MAX_HOSTNAME_LENGTH
        || !hostname.is_ascii()
        || !hostname.split('.').all(valid_dns_label)
    {
        return None;
    }

    let display_name = if hostname.len() <= MAX_DISPLAY_NAME_LENGTH {
        hostname.to_string()
    } else {
        // A DNS label is at most 63 bytes, so the short hostname is always
        // valid for HostBase.name's 100-byte display-name limit.
        hostname.split('.').next()?.to_string()
    };

    Some(ResolvedHostname {
        hostname: hostname.to_string(),
        display_name,
    })
}

fn valid_dns_label(label: &str) -> bool {
    !label.is_empty()
        && label.len() <= 63
        && !label.starts_with('-')
        && !label.ends_with('-')
        && label
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    fn test_semaphore(permits: usize) -> Arc<Semaphore> {
        Arc::new(Semaphore::new(permits))
    }

    #[tokio::test]
    async fn returns_normalized_local_resolver_hostname() {
        let result = resolve_hostname_with(
            "192.0.2.10".parse().unwrap(),
            &CancellationToken::new(),
            Duration::from_secs(1),
            test_semaphore(1),
            |_| Ok::<_, io::Error>("host-01.example.test.\n".to_string()),
        )
        .await
        .unwrap();

        assert_eq!(result.hostname, "host-01.example.test");
        assert_eq!(result.display_name, "host-01.example.test");
    }

    #[tokio::test]
    async fn resolver_errors_and_unusable_addresses_are_ignored() {
        let calls = Arc::new(AtomicUsize::new(0));
        let lookup_calls = calls.clone();
        let error = resolve_hostname_with(
            "192.0.2.10".parse().unwrap(),
            &CancellationToken::new(),
            Duration::from_secs(1),
            test_semaphore(1),
            move |_| {
                lookup_calls.fetch_add(1, Ordering::SeqCst);
                Err::<String, _>(io::Error::new(io::ErrorKind::NotFound, "no PTR"))
            },
        )
        .await;
        assert!(error.is_none());
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let unusable_calls = calls.clone();
        let unusable = resolve_hostname_with(
            "224.0.0.251".parse().unwrap(),
            &CancellationToken::new(),
            Duration::from_secs(1),
            test_semaphore(1),
            move |_| {
                unusable_calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, io::Error>("mdns.example.test".to_string())
            },
        )
        .await;
        assert!(unusable.is_none());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn rejects_invalid_or_unbounded_ptr_names_and_bounds_display_name() {
        assert!(normalize_hostname("").is_none());
        assert!(normalize_hostname("host name.example.test").is_none());
        assert!(normalize_hostname("-host.example.test").is_none());
        assert!(normalize_hostname("host_.example.test").is_none());
        assert!(normalize_hostname("høst.example.test").is_none());
        assert!(normalize_hostname(&"a".repeat(MAX_HOSTNAME_LENGTH + 1)).is_none());

        let long_name = format!(
            "{}.{}.{}.example.test",
            "a".repeat(63),
            "b".repeat(63),
            "c".repeat(63)
        );
        let normalized = normalize_hostname(&long_name).unwrap();
        assert_eq!(normalized.hostname, long_name);
        assert_eq!(normalized.display_name, "a".repeat(63));
        assert!(normalized.display_name.len() <= MAX_DISPLAY_NAME_LENGTH);
    }

    #[tokio::test]
    async fn cancellation_prevents_the_blocking_lookup() {
        let cancel = CancellationToken::new();
        cancel.cancel();
        let calls = Arc::new(AtomicUsize::new(0));
        let lookup_calls = calls.clone();

        let result = resolve_hostname_with(
            "192.0.2.10".parse().unwrap(),
            &cancel,
            Duration::from_secs(1),
            test_semaphore(1),
            move |_| {
                lookup_calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, io::Error>("host.example.test".to_string())
            },
        )
        .await;

        assert!(result.is_none());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn timed_out_blocking_lookup_retains_its_concurrency_permit() {
        let semaphore = test_semaphore(1);
        let calls = Arc::new(AtomicUsize::new(0));
        let first_calls = calls.clone();
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let first_semaphore = semaphore.clone();
        let first = tokio::spawn(async move {
            let cancel = CancellationToken::new();
            resolve_hostname_with(
                "192.0.2.10".parse().unwrap(),
                &cancel,
                Duration::from_millis(30),
                first_semaphore,
                move |_| {
                    first_calls.fetch_add(1, Ordering::SeqCst);
                    let _ = started_tx.send(());
                    std::thread::sleep(Duration::from_millis(100));
                    Ok::<_, io::Error>("slow.example.test".to_string())
                },
            )
            .await
        });
        started_rx.await.unwrap();
        let first = first.await.unwrap();
        assert!(first.is_none());

        let second_calls = calls.clone();
        let second = resolve_hostname_with(
            "192.0.2.11".parse().unwrap(),
            &CancellationToken::new(),
            Duration::from_millis(20),
            semaphore,
            move |_| {
                second_calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, io::Error>("second.example.test".to_string())
            },
        )
        .await;

        assert!(second.is_none());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
