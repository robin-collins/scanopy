use crate::daemon::shared::config::ConfigStore;
use crate::server::shared::types::api::{ApiErrorResponse, ApiResponse};
use anyhow::{Error, bail};
use reqwest::{Client, Method, RequestBuilder};
use serde::{Serialize, de::DeserializeOwned};
use std::error::Error as StdError;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::OnceCell;

pub struct DaemonApiClient {
    config_store: Arc<ConfigStore>,
    client: OnceCell<Client>,
}

impl DaemonApiClient {
    pub fn new(config_store: Arc<ConfigStore>) -> Self {
        Self {
            config_store,
            client: OnceCell::new(),
        }
    }

    /// Get or lazily initialize the HTTP client
    async fn get_client(&self) -> Result<&Client, Error> {
        self.client
            .get_or_try_init(|| async {
                let allow_self_signed_certs =
                    self.config_store.get_allow_self_signed_certs().await?;

                Client::builder()
                    .danger_accept_invalid_certs(allow_self_signed_certs)
                    .connect_timeout(Duration::from_secs(10))
                    .timeout(Duration::from_secs(30))
                    .build()
                    .map_err(|e| anyhow::anyhow!("Failed to build HTTP client: {}", e))
            })
            .await
    }

    /// Build a request with standard daemon auth headers
    async fn build_request(&self, method: Method, path: &str) -> Result<RequestBuilder, Error> {
        let client = self.get_client().await?;
        let server_target = self.config_store.get_server_url().await?;
        let daemon_id = self.config_store.get_id().await?;
        let api_key = self
            .config_store
            .get_api_key()
            .await?
            .ok_or_else(|| anyhow::anyhow!("API key not set"))?;

        let url = format!("{}{}", server_target, path);

        Ok(client
            .request(method, &url)
            .header("X-Daemon-ID", daemon_id.to_string())
            .header("X-Daemon-Version", env!("CARGO_PKG_VERSION"))
            .header("Authorization", format!("Bearer {}", api_key)))
    }

    /// Check response status and handle API errors.
    /// On error responses, attempts to parse as ApiErrorResponse to preserve error codes.
    async fn check_response(
        &self,
        response: reqwest::Response,
        context: &str,
    ) -> Result<ApiResponse<serde_json::Value>, Error> {
        let status = response.status();

        if !status.is_success() {
            // Try to parse as ApiErrorResponse to get error codes
            let body = response.text().await.unwrap_or_else(|_| String::new());
            if let Ok(error_response) = serde_json::from_str::<ApiErrorResponse>(&body) {
                return Err(error_response.into());
            }
            // Non-API response (HTML, plain text, etc.) — server URL is probably wrong
            let server_url = self.config_store.get_server_url().await.unwrap_or_default();
            bail!(
                "{}: HTTP {} — The server at {} did not respond as a Scanopy server. Verify the server URL is correct.",
                context,
                status,
                server_url
            );
        }

        let api_response: ApiResponse<serde_json::Value> = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("{}: Failed to parse response: {}", context, e))?;

        if !api_response.success {
            let error_msg = api_response
                .error
                .unwrap_or_else(|| format!("HTTP {}", status));

            bail!("{}: {}", context, error_msg);
        }

        Ok(api_response)
    }

    /// Classify a reqwest send error into a user-friendly message with diagnostics.
    /// Uses typed error inspection via reqwest predicates and io::ErrorKind enums.
    fn classify_connection_error(err: &reqwest::Error, url: &str) -> String {
        if err.is_timeout() {
            return format!(
                "Connection to {} timed out. A firewall may be blocking outbound traffic, or the server is unreachable.",
                url
            );
        }

        if err.is_connect() {
            // Walk the error source chain to find io::Error for specific classification
            let mut source: Option<&(dyn StdError + 'static)> = err.source();
            while let Some(inner) = source {
                if let Some(io_err) = inner.downcast_ref::<std::io::Error>() {
                    return match io_err.kind() {
                        std::io::ErrorKind::ConnectionRefused => format!(
                            "Connection refused by {}. The server may not be running, or the URL/port is wrong.",
                            url
                        ),
                        std::io::ErrorKind::ConnectionReset
                        | std::io::ErrorKind::ConnectionAborted => format!(
                            "Connection to {} was reset. The server may have closed the connection unexpectedly.",
                            url
                        ),
                        std::io::ErrorKind::AddrNotAvailable => {
                            format!("Address not available for {}. Check the server URL.", url)
                        }
                        _ => format!("Connection to {} failed: {}", url, io_err),
                    };
                }
                source = inner.source();
            }
        }

        format!("Failed to connect to {}: {}", url, err)
    }

    /// Execute request and parse ApiResponse, extracting data
    async fn execute<T: DeserializeOwned>(
        &self,
        request: RequestBuilder,
        context: &str,
    ) -> Result<T, Error> {
        let server_url = self.config_store.get_server_url().await.unwrap_or_default();
        let response = request.send().await.map_err(|e| {
            let msg = Self::classify_connection_error(&e, &server_url);
            anyhow::anyhow!("{}: {}", context, msg)
        })?;
        let api_response = self.check_response(response, context).await?;

        let data = api_response
            .data
            .ok_or_else(|| anyhow::anyhow!("{}: No data in response", context))?;

        serde_json::from_value(data)
            .map_err(|e| anyhow::anyhow!("{}: Failed to parse response data: {}", context, e))
    }

    /// Execute request, check for errors, but ignore response data
    async fn execute_no_data(&self, request: RequestBuilder, context: &str) -> Result<(), Error> {
        let server_url = self.config_store.get_server_url().await.unwrap_or_default();
        let response = request.send().await.map_err(|e| {
            let msg = Self::classify_connection_error(&e, &server_url);
            anyhow::anyhow!("{}: {}", context, msg)
        })?;
        self.check_response(response, context).await?;
        Ok(())
    }

    /// POST request expecting no response data
    pub async fn post_no_data<B: Serialize>(
        &self,
        path: &str,
        body: &B,
        context: &str,
    ) -> Result<(), Error> {
        let request = self.build_request(Method::POST, path).await?.json(body);
        self.execute_no_data(request, context).await
    }

    /// GET request
    pub async fn get<T: DeserializeOwned>(&self, path: &str, context: &str) -> Result<T, Error> {
        let request = self.build_request(Method::GET, path).await?;
        self.execute(request, context).await
    }

    /// POST request with JSON body
    pub async fn post<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
        context: &str,
    ) -> Result<T, Error> {
        let request = self.build_request(Method::POST, path).await?.json(body);
        self.execute(request, context).await
    }

    /// Access config store for cases that need custom handling
    pub fn config(&self) -> &Arc<ConfigStore> {
        &self.config_store
    }
}
