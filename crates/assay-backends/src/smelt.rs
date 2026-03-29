//! SmeltBackend — pushes session events to Smelt's HTTP server.
//!
//! `SmeltBackend` implements [`StateBackend`] by HTTP-POSTing every
//! `push_session_event` call to `<url>/api/v1/events?job=<job_id>`.
//! Uses `reqwest::blocking::Client` (D168 pattern, same as LinearBackend).
//!
//! # Graceful degradation (D190)
//! When the Smelt server is unreachable or returns a non-2xx status,
//! `push_session_event` and `annotate_run` emit `tracing::warn!` and
//! return `Ok(())` — the run is **never** aborted due to a Smelt failure.

use std::path::Path;

use assay_core::{AssayError, CapabilitySet, StateBackend};
use assay_types::{OrchestratorStatus, TeamCheckpoint};

/// Remote backend that pushes orchestrator events to Smelt's HTTP server.
///
/// Each `push_session_event` call POSTs the serialized `OrchestratorStatus`
/// JSON to `<url>/api/v1/events?job=<job_id>`. An optional bearer token
/// is included in the `Authorization` header when configured.
pub struct SmeltBackend {
    url: String,
    job_id: String,
    #[allow(dead_code)]
    token: Option<String>,
    client: reqwest::blocking::Client,
}

impl std::fmt::Debug for SmeltBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SmeltBackend")
            .field("url", &self.url)
            .field("job_id", &self.job_id)
            .field("has_token", &self.token.is_some())
            .finish()
    }
}

impl SmeltBackend {
    /// Construct a new `SmeltBackend`.
    ///
    /// If `token` is `Some`, all requests include an `Authorization: Bearer`
    /// header with `set_sensitive(true)` to prevent logging.
    pub fn new(url: String, job_id: String, token: Option<String>) -> Self {
        use reqwest::header;

        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );

        if let Some(ref tok) = token {
            let mut auth_value = header::HeaderValue::from_str(&format!("Bearer {tok}"))
                .unwrap_or_else(|_| {
                    // Fallback: token contains non-ASCII; this should never happen
                    // with well-formed bearer tokens.
                    header::HeaderValue::from_static("Bearer <invalid>")
                });
            auth_value.set_sensitive(true);
            headers.insert(header::AUTHORIZATION, auth_value);
        }

        let client = reqwest::blocking::Client::builder()
            .default_headers(headers)
            .build()
            .expect("reqwest::blocking::Client builder should not fail with valid headers");

        Self {
            url,
            job_id,
            token,
            client,
        }
    }

    /// Build the events endpoint URL.
    fn events_url(&self) -> String {
        format!(
            "{}/api/v1/events?job={}",
            self.url.trim_end_matches('/'),
            self.job_id
        )
    }
}

impl StateBackend for SmeltBackend {
    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet {
            supports_signals: true,
            supports_messaging: false,
            supports_gossip_manifest: false,
            supports_annotations: true,
            supports_checkpoints: false,
            supports_peer_registry: false, // register_peer is fire-and-forget; unregister not implemented
        }
    }

    fn push_session_event(
        &self,
        _run_dir: &Path,
        status: &OrchestratorStatus,
    ) -> assay_core::Result<()> {
        let json_str = serde_json::to_string(status)
            .map_err(|e| AssayError::json("serializing OrchestratorStatus", "SmeltBackend", e))?;

        let url = self.events_url();
        match self.client.post(&url).body(json_str).send() {
            Ok(resp) => {
                let status_code = resp.status();
                if status_code.is_success() {
                    tracing::debug!(
                        url = %url,
                        status = %status_code,
                        "SmeltBackend: push_session_event succeeded"
                    );
                } else {
                    tracing::warn!(
                        url = %url,
                        status = %status_code,
                        "SmeltBackend: push_session_event received non-2xx, continuing gracefully"
                    );
                }
            }
            Err(e) => {
                tracing::warn!(
                    url = %url,
                    error = %e,
                    "SmeltBackend: push_session_event failed, continuing gracefully"
                );
            }
        }

        Ok(())
    }

    fn read_run_state(&self, _run_dir: &Path) -> assay_core::Result<Option<OrchestratorStatus>> {
        Err(AssayError::io(
            "read_run_state not supported by SmeltBackend",
            "SmeltBackend",
            std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "SmeltBackend does not support read_run_state",
            ),
        ))
    }

    fn send_message(
        &self,
        _inbox_path: &Path,
        _name: &str,
        _contents: &[u8],
    ) -> assay_core::Result<()> {
        Err(AssayError::io(
            "send_message not supported by SmeltBackend",
            "SmeltBackend",
            std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "SmeltBackend does not support messaging",
            ),
        ))
    }

    fn poll_inbox(&self, _inbox_path: &Path) -> assay_core::Result<Vec<(String, Vec<u8>)>> {
        Err(AssayError::io(
            "poll_inbox not supported by SmeltBackend",
            "SmeltBackend",
            std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "SmeltBackend does not support messaging",
            ),
        ))
    }

    fn annotate_run(&self, _run_dir: &Path, manifest_path: &str) -> assay_core::Result<()> {
        let body = format!("[assay:manifest] {manifest_path}");
        let url = self.events_url();

        match self.client.post(&url).body(body).send() {
            Ok(resp) => {
                let status_code = resp.status();
                if status_code.is_success() {
                    tracing::debug!(
                        url = %url,
                        status = %status_code,
                        "SmeltBackend: annotate_run succeeded"
                    );
                } else {
                    tracing::warn!(
                        url = %url,
                        status = %status_code,
                        "SmeltBackend: annotate_run received non-2xx, continuing gracefully"
                    );
                }
            }
            Err(e) => {
                tracing::warn!(
                    url = %url,
                    error = %e,
                    "SmeltBackend: annotate_run failed, continuing gracefully"
                );
            }
        }

        Ok(())
    }

    fn save_checkpoint_summary(
        &self,
        _assay_dir: &Path,
        _checkpoint: &TeamCheckpoint,
    ) -> assay_core::Result<()> {
        Err(AssayError::io(
            "save_checkpoint_summary not supported by SmeltBackend",
            "SmeltBackend",
            std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "SmeltBackend does not support checkpoints",
            ),
        ))
    }

    fn register_peer(&self, peer: &assay_types::PeerInfo) -> assay_core::Result<()> {
        let url = format!("{}/api/v1/peers", self.url);
        // Use the header-map client which sets Content-Type: application/json by default.
        let json_bytes = serde_json::to_vec(peer)
            .map_err(|e| AssayError::json("serializing PeerInfo", "SmeltBackend", e))?;

        match self.client.post(&url).body(json_bytes).send() {
            Ok(resp) => {
                let status_code = resp.status();
                if status_code.is_success() {
                    tracing::debug!(
                        url = %url,
                        peer_id = %peer.peer_id,
                        "SmeltBackend: register_peer succeeded"
                    );
                } else {
                    tracing::warn!(
                        url = %url,
                        status = %status_code,
                        peer_id = %peer.peer_id,
                        "SmeltBackend: register_peer received non-2xx, continuing gracefully"
                    );
                }
            }
            Err(e) => {
                tracing::warn!(
                    url = %url,
                    error = %e,
                    peer_id = %peer.peer_id,
                    "SmeltBackend: register_peer failed, continuing gracefully"
                );
            }
        }

        Ok(())
    }
}
