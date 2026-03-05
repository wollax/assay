//! MCP server implementation with spec and gate tools.
//!
//! Provides the [`AssayServer`] which exposes five tools over MCP:
//! - `spec_list` — discover available specs
//! - `spec_get` — read a full spec definition
//! - `gate_run` — evaluate quality gate criteria (auto-creates sessions for agent criteria)
//! - `gate_report` — submit agent evaluation for a criterion in an active session
//! - `gate_finalize` — finalize a session, persisting all evaluations as a GateRunRecord
//!
//! All domain errors are returned as `CallToolResult` with `isError: true`
//! so that agents can see and self-correct. Protocol errors (`McpError`)
//! are reserved for infrastructure failures.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt, handler::server::tool::ToolRouter,
    handler::server::wrapper::Parameters, model::*, tool, tool_handler, tool_router,
    transport::io::stdio,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use assay_core::spec::SpecEntry;
use assay_types::{
    AgentEvaluation, AgentSession, Confidence, Config, CriterionKind, EvaluatorRole,
};

// ── Parameter structs ────────────────────────────────────────────────

/// Parameters for the `spec_get` tool.
#[derive(Deserialize, JsonSchema)]
struct SpecGetParams {
    /// The spec to retrieve.
    #[schemars(description = "Spec name (filename without .toml extension, e.g. 'auth-flow')")]
    name: String,
}

/// Parameters for the `gate_run` tool.
#[derive(Deserialize, JsonSchema)]
struct GateRunParams {
    /// The spec whose criteria to evaluate.
    #[schemars(
        description = "Spec name to evaluate gates for (filename without .toml extension, e.g. 'auth-flow')"
    )]
    name: String,

    /// Whether to include full evidence in the response.
    #[schemars(
        description = "Include full stdout/stderr evidence per criterion (default: false). \
            When false, returns summary only with pass/fail, exit code, and duration. \
            When true, adds stdout and stderr fields to each criterion result."
    )]
    #[serde(default)]
    include_evidence: bool,
}

/// Parameters for the `gate_report` tool.
#[derive(Deserialize, JsonSchema)]
struct GateReportParams {
    /// Session ID returned by gate_run.
    #[schemars(description = "Session ID returned by gate_run")]
    session_id: String,

    /// Name of the criterion being evaluated (must match a criterion in the spec).
    #[schemars(
        description = "Name of the criterion being evaluated (must match a criterion in the spec)"
    )]
    criterion_name: String,

    /// Whether the criterion passed.
    #[schemars(description = "Whether the criterion passed")]
    passed: bool,

    /// What the agent observed (concrete facts).
    #[schemars(description = "What the agent observed (concrete facts)")]
    evidence: String,

    /// Why those facts lead to pass/fail.
    #[schemars(description = "Why those facts lead to pass/fail")]
    reasoning: String,

    /// Confidence in the evaluation (high, medium, low).
    #[schemars(description = "Confidence in the evaluation (high, medium, low)")]
    #[serde(default)]
    confidence: Option<Confidence>,

    /// Role of the evaluator (self, independent, human).
    #[schemars(description = "Role of the evaluator (self, independent, human)")]
    evaluator_role: EvaluatorRole,
}

/// Parameters for the `gate_finalize` tool.
#[derive(Deserialize, JsonSchema)]
struct GateFinalizeParams {
    /// Session ID to finalize.
    #[schemars(description = "Session ID to finalize")]
    session_id: String,
}

// ── Response structs ─────────────────────────────────────────────────

/// A single entry in the `spec_list` response.
#[derive(Serialize)]
struct SpecListEntry {
    name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    description: String,
    criteria_count: usize,
    /// "legacy" for flat .toml files, "directory" for directory-based specs.
    format: String,
    /// Whether a companion `spec.toml` (feature spec) exists.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    has_feature_spec: bool,
}

/// Aggregate gate run response.
#[derive(Serialize)]
struct GateRunResponse {
    spec_name: String,
    passed: usize,
    failed: usize,
    skipped: usize,
    required_failed: usize,
    advisory_failed: usize,
    total_duration_ms: u64,
    criteria: Vec<CriterionSummary>,
    /// Session ID when agent criteria are present (requires gate_report + gate_finalize).
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<String>,
    /// Criteria pending agent evaluation (names only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pending_criteria: Option<Vec<String>>,
}

/// Response from the `gate_report` tool.
#[derive(Serialize)]
struct GateReportResponse {
    session_id: String,
    criterion_name: String,
    accepted: bool,
    evaluations_count: usize,
    pending_criteria: Vec<String>,
}

/// Per-criterion result within a gate run response.
#[derive(Serialize)]
struct CriterionSummary {
    name: String,
    status: String,
    enforcement: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    kind_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stdout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stderr: Option<String>,
}

// ── Constants ────────────────────────────────────────────────────────

/// Session timeout in seconds (30 minutes).
const SESSION_TIMEOUT_SECS: u64 = 1800;

// ── Server struct ────────────────────────────────────────────────────

/// MCP server exposing Assay spec and gate operations as tools.
#[derive(Clone)]
pub struct AssayServer {
    tool_router: ToolRouter<Self>,
    sessions: Arc<Mutex<HashMap<String, AgentSession>>>,
}

impl Default for AssayServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl AssayServer {
    /// Create a new server with the tool router initialized.
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// List all specs in the current Assay project.
    #[tool(
        description = "List all specs in the current Assay project. Returns an array of {name, description, criteria_count} objects. Use this to discover available specs before calling spec_get or gate_run."
    )]
    async fn spec_list(&self) -> Result<CallToolResult, McpError> {
        let cwd = resolve_cwd()?;
        let config = match load_config(&cwd) {
            Ok(c) => c,
            Err(err_result) => return Ok(err_result),
        };

        let specs_dir = cwd.join(".assay").join(&config.specs_dir);
        let scan_result = match assay_core::spec::scan(&specs_dir) {
            Ok(r) => r,
            Err(e) => return Ok(domain_error(&e)),
        };

        let entries: Vec<SpecListEntry> = scan_result
            .entries
            .iter()
            .map(|entry| match entry {
                SpecEntry::Legacy { slug, spec } => SpecListEntry {
                    name: slug.clone(),
                    description: spec.description.clone(),
                    criteria_count: spec.criteria.len(),
                    format: "legacy".to_string(),
                    has_feature_spec: false,
                },
                SpecEntry::Directory {
                    slug,
                    gates,
                    spec_path,
                } => SpecListEntry {
                    name: slug.clone(),
                    description: gates.description.clone(),
                    criteria_count: gates.criteria.len(),
                    format: "directory".to_string(),
                    has_feature_spec: spec_path.is_some(),
                },
            })
            .collect();

        let json = serde_json::to_string(&entries)
            .map_err(|e| McpError::internal_error(format!("serialization failed: {e}"), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Get a spec by name.
    #[tool(
        description = "Get a spec by name. Returns the full spec definition as JSON. For legacy specs: {format, name, description, criteria}. For directory specs: {format, gates, feature_spec?}. Use spec_list first to find available spec names."
    )]
    async fn spec_get(
        &self,
        params: Parameters<SpecGetParams>,
    ) -> Result<CallToolResult, McpError> {
        let cwd = resolve_cwd()?;
        let config = match load_config(&cwd) {
            Ok(c) => c,
            Err(err_result) => return Ok(err_result),
        };
        let entry = match load_spec_entry_mcp(&cwd, &config, &params.0.name) {
            Ok(e) => e,
            Err(err_result) => return Ok(err_result),
        };

        let json = match &entry {
            SpecEntry::Legacy { spec, .. } => {
                let response = serde_json::json!({
                    "format": "legacy",
                    "name": spec.name,
                    "description": spec.description,
                    "criteria": spec.criteria,
                });
                serde_json::to_string(&response)
            }
            SpecEntry::Directory {
                gates, spec_path, ..
            } => {
                let feature_spec = spec_path
                    .as_ref()
                    .and_then(|p| assay_core::spec::load_feature_spec(p).ok());
                let response = serde_json::json!({
                    "format": "directory",
                    "gates": gates,
                    "feature_spec": feature_spec,
                });
                serde_json::to_string(&response)
            }
        }
        .map_err(|e| McpError::internal_error(format!("serialization failed: {e}"), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Run quality gate checks for a spec.
    #[tool(
        description = "Run quality gate checks for a spec. Evaluates all executable criteria (shell commands) and returns pass/fail status per criterion with aggregate counts. If the spec contains agent-evaluated criteria (kind=AgentReport), a session is auto-created and a session_id is returned — use gate_report to submit evaluations, then gate_finalize to persist results. Set include_evidence=true for full stdout/stderr output per criterion."
    )]
    async fn gate_run(
        &self,
        params: Parameters<GateRunParams>,
    ) -> Result<CallToolResult, McpError> {
        let cwd = resolve_cwd()?;
        let config = match load_config(&cwd) {
            Ok(c) => c,
            Err(err_result) => return Ok(err_result),
        };
        let entry = match load_spec_entry_mcp(&cwd, &config, &params.0.name) {
            Ok(e) => e,
            Err(err_result) => return Ok(err_result),
        };
        let include_evidence = params.0.include_evidence;

        let working_dir = resolve_working_dir(&cwd, &config);
        let config_timeout = config.gates.as_ref().map(|g| g.default_timeout);

        // Extract agent criteria info before moving entry into spawn_blocking.
        let agent_info = extract_agent_criteria_info(&entry);

        let working_dir_owned = working_dir.clone();

        let summary = tokio::task::spawn_blocking(move || match entry {
            SpecEntry::Legacy { spec, .. } => {
                assay_core::gate::evaluate_all(&spec, &working_dir_owned, None, config_timeout)
            }
            SpecEntry::Directory { gates, .. } => assay_core::gate::evaluate_all_gates(
                &gates,
                &working_dir_owned,
                None,
                config_timeout,
            ),
        })
        .await
        .map_err(|e| McpError::internal_error(format!("gate evaluation panicked: {e}"), None))?;

        let mut response = format_gate_response(&summary, include_evidence);

        // If spec has agent criteria, create a session and attach to response.
        if let Some(info) = agent_info {
            let session = assay_core::gate::session::create_session(
                &summary.spec_name,
                info.agent_criteria_names,
                info.spec_enforcement,
                summary.results.clone(),
            );

            let session_id = session.session_id.clone();
            let pending: Vec<String> = session.criteria_names.iter().cloned().collect();

            response.session_id = Some(session_id.clone());
            response.pending_criteria = Some(pending);

            // Store the session.
            self.sessions
                .lock()
                .await
                .insert(session_id.clone(), session);

            // Spawn timeout task.
            let sessions = Arc::clone(&self.sessions);
            let assay_dir = cwd.join(".assay");
            let max_history = config.gates.as_ref().and_then(|g| g.max_history);
            let wd_string = working_dir.to_string_lossy().to_string();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_secs(SESSION_TIMEOUT_SECS)).await;
                let session = {
                    let mut sessions = sessions.lock().await;
                    sessions.remove(&session_id)
                };
                if let Some(session) = session {
                    tracing::warn!(
                        session_id = %session.session_id,
                        spec_name = %session.spec_name,
                        "session timed out after {}s, auto-finalizing",
                        SESSION_TIMEOUT_SECS
                    );
                    let record = assay_core::gate::session::finalize_as_timed_out(&session);
                    if let Err(e) = assay_core::history::save(&assay_dir, &record, max_history) {
                        tracing::error!(
                            session_id = %record.run_id,
                            "failed to save timed-out session: {e}"
                        );
                    } else {
                        tracing::info!(
                            session_id = %record.run_id,
                            spec_name = %record.summary.spec_name,
                            passed = record.summary.passed,
                            failed = record.summary.failed,
                            "timed-out session saved to history"
                        );
                    }
                    let _ = wd_string; // suppress unused warning (captured for future use)
                }
            });
        }

        let json = serde_json::to_string(&response)
            .map_err(|e| McpError::internal_error(format!("serialization failed: {e}"), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Submit an agent evaluation for a single criterion in an active gate session.
    #[tool(
        description = "Submit an agent evaluation for a single criterion in an active gate session. Call gate_run first to start a session, then gate_report for each agent-evaluated criterion, then gate_finalize to persist results."
    )]
    async fn gate_report(
        &self,
        params: Parameters<GateReportParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let mut sessions = self.sessions.lock().await;
        let Some(session) = sessions.get_mut(&p.session_id) else {
            return Ok(CallToolResult::error(vec![Content::text(format!(
                "session '{}' not found (expired or already finalized)",
                p.session_id
            ))]));
        };

        let eval = AgentEvaluation {
            passed: p.passed,
            evidence: p.evidence,
            reasoning: p.reasoning,
            confidence: p.confidence,
            evaluator_role: p.evaluator_role,
            timestamp: Utc::now(),
        };

        if let Err(e) =
            assay_core::gate::session::report_evaluation(session, &p.criterion_name, eval)
        {
            return Ok(domain_error(&e));
        }

        let pending: Vec<String> = session
            .criteria_names
            .iter()
            .filter(|name| !session.agent_evaluations.contains_key(*name))
            .cloned()
            .collect();
        let evaluations_count = session
            .agent_evaluations
            .get(&p.criterion_name)
            .map_or(0, |v| v.len());

        let response = GateReportResponse {
            session_id: p.session_id,
            criterion_name: p.criterion_name,
            accepted: true,
            evaluations_count,
            pending_criteria: pending,
        };

        let json = serde_json::to_string(&response)
            .map_err(|e| McpError::internal_error(format!("serialization failed: {e}"), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Finalize an active gate session, persisting all accumulated evaluations.
    #[tool(
        description = "Finalize an active gate session, persisting all accumulated evaluations as a GateRunRecord. Missing required criteria cause the gate to fail. Call this after submitting all gate_report evaluations."
    )]
    async fn gate_finalize(
        &self,
        params: Parameters<GateFinalizeParams>,
    ) -> Result<CallToolResult, McpError> {
        let session_id = params.0.session_id;
        let session = {
            let mut sessions = self.sessions.lock().await;
            sessions.remove(&session_id)
        };
        let Some(session) = session else {
            return Ok(CallToolResult::error(vec![Content::text(format!(
                "session '{}' not found (expired or already finalized)",
                session_id
            ))]));
        };

        let cwd = resolve_cwd()?;
        let config = match load_config(&cwd) {
            Ok(c) => c,
            Err(e) => return Ok(e),
        };
        let assay_dir = cwd.join(".assay");
        let working_dir = resolve_working_dir(&cwd, &config);
        let max_history = config.gates.as_ref().and_then(|g| g.max_history);

        let record = match assay_core::gate::session::finalize_session(
            &session,
            &assay_dir,
            Some(&working_dir.to_string_lossy()),
            max_history,
        ) {
            Ok(r) => r,
            Err(e) => return Ok(domain_error(&e)),
        };

        let response = serde_json::json!({
            "run_id": record.run_id,
            "spec_name": record.summary.spec_name,
            "passed": record.summary.passed,
            "failed": record.summary.failed,
            "skipped": record.summary.skipped,
            "required_failed": record.summary.enforcement.required_failed,
            "advisory_failed": record.summary.enforcement.advisory_failed,
            "persisted": true,
        });

        let json = serde_json::to_string(&response)
            .map_err(|e| McpError::internal_error(format!("serialization failed: {e}"), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }
}

#[tool_handler]
impl ServerHandler for AssayServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "Assay development kit. Manages specs (what to build) and gates \
                 (quality checks). Use spec_list to discover specs, spec_get to \
                 read one, gate_run to evaluate criteria. For specs with agent \
                 criteria: gate_run returns a session_id, then call gate_report \
                 for each criterion, and gate_finalize to persist results."
                    .to_string(),
            ),
        }
    }
}

// ── Helper functions ─────────────────────────────────────────────────

/// Resolve the current working directory.
fn resolve_cwd() -> Result<PathBuf, McpError> {
    std::env::current_dir().map_err(|e| {
        McpError::internal_error(format!("cannot determine working directory: {e}"), None)
    })
}

/// Load and validate the Assay config from CWD.
fn load_config(cwd: &Path) -> Result<Config, CallToolResult> {
    assay_core::config::load(cwd).map_err(|e| domain_error(&e))
}

/// Load a spec entry by name, trying directory-based first, then legacy.
fn load_spec_entry_mcp(
    cwd: &Path,
    config: &Config,
    name: &str,
) -> Result<SpecEntry, CallToolResult> {
    let specs_dir = cwd.join(".assay").join(&config.specs_dir);
    assay_core::spec::load_spec_entry(name, &specs_dir).map_err(|e| domain_error(&e))
}

/// Resolve the gate working directory from config, matching CLI behavior.
fn resolve_working_dir(cwd: &Path, config: &Config) -> PathBuf {
    match config.gates.as_ref().and_then(|g| g.working_dir.as_deref()) {
        Some(dir) => {
            let path = Path::new(dir);
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                cwd.join(path)
            }
        }
        None => cwd.to_path_buf(),
    }
}

/// Convert an AssayError into a tool execution error that the agent can see and act on.
fn domain_error(err: &assay_core::AssayError) -> CallToolResult {
    CallToolResult::error(vec![Content::text(err.to_string())])
}

/// Collected info about agent criteria in a spec, used to create a session.
struct AgentCriteriaInfo {
    agent_criteria_names: std::collections::HashSet<String>,
    spec_enforcement: HashMap<String, assay_types::Enforcement>,
}

/// A single criterion's metadata for agent criteria extraction.
type CriterionMeta<'a> = (
    &'a str,
    Option<CriterionKind>,
    Option<assay_types::Enforcement>,
);

/// Extract agent criteria information from a spec entry.
///
/// Returns `Some(info)` if the spec contains any `AgentReport` criteria,
/// `None` if all criteria are command-based or descriptive.
fn extract_agent_criteria_info(entry: &SpecEntry) -> Option<AgentCriteriaInfo> {
    let (criteria_iter, gate_section): (
        Box<dyn Iterator<Item = CriterionMeta<'_>>>,
        Option<&assay_types::GateSection>,
    ) = match entry {
        SpecEntry::Legacy { spec, .. } => (
            Box::new(
                spec.criteria
                    .iter()
                    .map(|c| (c.name.as_str(), c.kind, c.enforcement)),
            ),
            spec.gate.as_ref(),
        ),
        SpecEntry::Directory { gates, .. } => (
            Box::new(
                gates
                    .criteria
                    .iter()
                    .map(|c| (c.name.as_str(), c.kind, c.enforcement)),
            ),
            gates.gate.as_ref(),
        ),
    };

    let mut agent_criteria_names = std::collections::HashSet::new();
    let mut spec_enforcement = HashMap::new();

    for (name, kind, enforcement) in criteria_iter {
        if kind == Some(CriterionKind::AgentReport) {
            agent_criteria_names.insert(name.to_string());
        }
        let resolved = assay_core::gate::resolve_enforcement(enforcement, gate_section);
        spec_enforcement.insert(name.to_string(), resolved);
    }

    if agent_criteria_names.is_empty() {
        None
    } else {
        Some(AgentCriteriaInfo {
            agent_criteria_names,
            spec_enforcement,
        })
    }
}

/// Derive a human-readable kind label from a `GateKind`.
fn kind_label(kind: &assay_types::GateKind) -> Option<String> {
    match kind {
        assay_types::GateKind::Command { .. } => Some("cmd".to_string()),
        assay_types::GateKind::FileExists { .. } => Some("file".to_string()),
        assay_types::GateKind::AgentReport => Some("agent".to_string()),
        assay_types::GateKind::AlwaysPass => None,
    }
}

/// Map a `GateRunSummary` to the bounded `GateRunResponse` struct.
fn format_gate_response(
    summary: &assay_types::GateRunSummary,
    include_evidence: bool,
) -> GateRunResponse {
    let enforcement_label = |e: assay_types::Enforcement| -> String {
        match e {
            assay_types::Enforcement::Required => "required".to_string(),
            assay_types::Enforcement::Advisory => "advisory".to_string(),
        }
    };

    let criteria = summary
        .results
        .iter()
        .map(|cr| match &cr.result {
            None => CriterionSummary {
                name: cr.criterion_name.clone(),
                status: "skipped".to_string(),
                enforcement: enforcement_label(cr.enforcement),
                kind_label: None,
                exit_code: None,
                duration_ms: None,
                reason: None,
                stdout: None,
                stderr: None,
            },
            Some(gate_result) if gate_result.passed => CriterionSummary {
                name: cr.criterion_name.clone(),
                status: "passed".to_string(),
                enforcement: enforcement_label(cr.enforcement),
                kind_label: kind_label(&gate_result.kind),
                exit_code: gate_result.exit_code,
                duration_ms: Some(gate_result.duration_ms),
                reason: None,
                stdout: if include_evidence {
                    Some(gate_result.stdout.clone())
                } else {
                    None
                },
                stderr: if include_evidence {
                    Some(gate_result.stderr.clone())
                } else {
                    None
                },
            },
            Some(gate_result) => {
                let reason = first_nonempty_line(&gate_result.stderr)
                    .unwrap_or("unknown")
                    .to_string();
                CriterionSummary {
                    name: cr.criterion_name.clone(),
                    status: "failed".to_string(),
                    enforcement: enforcement_label(cr.enforcement),
                    kind_label: kind_label(&gate_result.kind),
                    exit_code: gate_result.exit_code,
                    duration_ms: Some(gate_result.duration_ms),
                    reason: Some(reason),
                    stdout: if include_evidence {
                        Some(gate_result.stdout.clone())
                    } else {
                        None
                    },
                    stderr: if include_evidence {
                        Some(gate_result.stderr.clone())
                    } else {
                        None
                    },
                }
            }
        })
        .collect();

    GateRunResponse {
        spec_name: summary.spec_name.clone(),
        passed: summary.passed,
        failed: summary.failed,
        skipped: summary.skipped,
        required_failed: summary.enforcement.required_failed,
        advisory_failed: summary.enforcement.advisory_failed,
        total_duration_ms: summary.total_duration_ms,
        criteria,
        session_id: None,
        pending_criteria: None,
    }
}

/// Extract the first non-empty line from a string, or `None` if all lines are empty.
fn first_nonempty_line(s: &str) -> Option<&str> {
    s.lines().find(|line| !line.trim().is_empty())
}

/// Start the MCP server on stdio transport.
///
/// Creates an [`AssayServer`] and serves JSON-RPC on stdin/stdout until
/// the transport closes. Caller must initialize tracing before calling.
pub async fn serve() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Starting assay MCP server");

    let service = AssayServer::new().serve(stdio()).await?;

    service.waiting().await?;
    Ok(())
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use assay_types::{
        CriterionResult, Enforcement, EnforcementSummary, GateKind, GateResult, GateRunSummary,
    };
    use chrono::Utc;
    use std::io::Write as _;

    fn sample_summary() -> GateRunSummary {
        GateRunSummary {
            spec_name: "test-spec".to_string(),
            results: vec![
                CriterionResult {
                    criterion_name: "unit-tests".to_string(),
                    result: Some(GateResult {
                        passed: true,
                        kind: GateKind::Command {
                            cmd: "cargo test".to_string(),
                        },
                        stdout: "running 5 tests\ntest ok\n".to_string(),
                        stderr: String::new(),
                        exit_code: Some(0),
                        duration_ms: 1200,
                        timestamp: Utc::now(),
                        truncated: false,
                        original_bytes: None,
                        evidence: None,
                        reasoning: None,
                        confidence: None,
                        evaluator_role: None,
                    }),
                    enforcement: Enforcement::Required,
                },
                CriterionResult {
                    criterion_name: "lint".to_string(),
                    result: Some(GateResult {
                        passed: false,
                        kind: GateKind::Command {
                            cmd: "cargo clippy".to_string(),
                        },
                        stdout: String::new(),
                        stderr: "error: unused variable\n  --> src/main.rs:5:9\n".to_string(),
                        exit_code: Some(1),
                        duration_ms: 800,
                        timestamp: Utc::now(),
                        truncated: false,
                        original_bytes: None,
                        evidence: None,
                        reasoning: None,
                        confidence: None,
                        evaluator_role: None,
                    }),
                    enforcement: Enforcement::Required,
                },
                CriterionResult {
                    criterion_name: "review-checklist".to_string(),
                    result: None,
                    enforcement: Enforcement::Required,
                },
            ],
            passed: 1,
            failed: 1,
            skipped: 1,
            total_duration_ms: 2000,
            enforcement: EnforcementSummary::default(),
        }
    }

    #[test]
    fn test_format_gate_response_summary_mode() {
        let summary = sample_summary();
        let response = format_gate_response(&summary, false);

        assert_eq!(response.spec_name, "test-spec");
        assert_eq!(response.passed, 1);
        assert_eq!(response.failed, 1);
        assert_eq!(response.skipped, 1);
        assert_eq!(response.total_duration_ms, 2000);
        assert_eq!(response.criteria.len(), 3);
        assert!(
            response.session_id.is_none(),
            "no session for cmd-only spec"
        );
        assert!(response.pending_criteria.is_none());

        // Passed criterion
        let passed = &response.criteria[0];
        assert_eq!(passed.name, "unit-tests");
        assert_eq!(passed.status, "passed");
        assert_eq!(passed.kind_label.as_deref(), Some("cmd"));
        assert_eq!(passed.exit_code, Some(0));
        assert_eq!(passed.duration_ms, Some(1200));
        assert!(
            passed.reason.is_none(),
            "passed criteria should not have reason"
        );
        assert!(
            passed.stdout.is_none(),
            "summary mode should not have stdout"
        );
        assert!(
            passed.stderr.is_none(),
            "summary mode should not have stderr"
        );

        // Failed criterion
        let failed = &response.criteria[1];
        assert_eq!(failed.name, "lint");
        assert_eq!(failed.status, "failed");
        assert_eq!(failed.kind_label.as_deref(), Some("cmd"));
        assert_eq!(failed.exit_code, Some(1));
        assert_eq!(failed.duration_ms, Some(800));
        assert!(
            failed
                .reason
                .as_deref()
                .unwrap()
                .contains("unused variable"),
            "failed reason should contain first stderr line, got: {:?}",
            failed.reason
        );
        assert!(
            failed.stdout.is_none(),
            "summary mode should not have stdout"
        );
        assert!(
            failed.stderr.is_none(),
            "summary mode should not have stderr"
        );

        // Skipped criterion
        let skipped = &response.criteria[2];
        assert_eq!(skipped.name, "review-checklist");
        assert_eq!(skipped.status, "skipped");
        assert!(skipped.kind_label.is_none(), "skipped has no kind_label");
        assert!(skipped.exit_code.is_none());
        assert!(skipped.duration_ms.is_none());
        assert!(skipped.reason.is_none());
        assert!(skipped.stdout.is_none());
        assert!(skipped.stderr.is_none());
    }

    #[test]
    fn test_format_gate_response_evidence_mode() {
        let summary = sample_summary();
        let response = format_gate_response(&summary, true);

        // Passed criterion should include stdout/stderr
        let passed = &response.criteria[0];
        assert!(
            passed.stdout.is_some(),
            "evidence mode should include stdout for passed"
        );
        assert!(
            passed.stderr.is_some(),
            "evidence mode should include stderr for passed"
        );
        assert!(
            passed
                .stdout
                .as_deref()
                .unwrap()
                .contains("running 5 tests"),
            "stdout should contain actual output"
        );

        // Failed criterion should include stdout/stderr
        let failed = &response.criteria[1];
        assert!(
            failed.stdout.is_some(),
            "evidence mode should include stdout for failed"
        );
        assert!(
            failed.stderr.is_some(),
            "evidence mode should include stderr for failed"
        );
        assert!(
            failed
                .stderr
                .as_deref()
                .unwrap()
                .contains("unused variable"),
            "stderr should contain actual output"
        );

        // Skipped criterion still has no evidence
        let skipped = &response.criteria[2];
        assert!(skipped.stdout.is_none());
        assert!(skipped.stderr.is_none());
    }

    #[test]
    fn test_domain_error_produces_error_result() {
        let err = assay_core::AssayError::SpecNotFound {
            name: "auth-flow".to_string(),
            specs_dir: std::path::PathBuf::from(".assay/specs/"),
        };

        let result = domain_error(&err);

        // CallToolResult should be an error (is_error: true)
        assert!(
            result.is_error.unwrap_or(false),
            "domain_error should produce isError: true"
        );

        // The content should contain the error message
        let text = result
            .content
            .iter()
            .filter_map(|c| match &c.raw {
                RawContent::Text(t) => Some(t.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");
        assert!(
            text.contains("auth-flow"),
            "error text should contain spec name, got: {text}"
        );
        assert!(
            text.contains(".assay/specs/"),
            "error text should contain specs dir, got: {text}"
        );
    }

    #[test]
    fn test_first_nonempty_line() {
        assert_eq!(first_nonempty_line("hello\nworld"), Some("hello"));
        assert_eq!(first_nonempty_line("\n\nhello"), Some("hello"));
        assert_eq!(first_nonempty_line("  \n  \n"), None);
        assert_eq!(first_nonempty_line(""), None);
        assert_eq!(
            first_nonempty_line("error: unused variable\n  --> src/main.rs"),
            Some("error: unused variable")
        );
    }

    #[test]
    fn test_format_gate_response_failed_with_empty_stderr() {
        let summary = GateRunSummary {
            spec_name: "test".to_string(),
            results: vec![CriterionResult {
                criterion_name: "silent-fail".to_string(),
                result: Some(GateResult {
                    passed: false,
                    kind: GateKind::Command {
                        cmd: "false".to_string(),
                    },
                    stdout: String::new(),
                    stderr: String::new(),
                    exit_code: Some(1),
                    duration_ms: 50,
                    timestamp: Utc::now(),
                    truncated: false,
                    original_bytes: None,
                    evidence: None,
                    reasoning: None,
                    confidence: None,
                    evaluator_role: None,
                }),
                enforcement: Enforcement::Required,
            }],
            passed: 0,
            failed: 1,
            skipped: 0,
            total_duration_ms: 50,
            enforcement: EnforcementSummary::default(),
        };

        let response = format_gate_response(&summary, false);
        let failed = &response.criteria[0];
        assert_eq!(
            failed.reason.as_deref(),
            Some("unknown"),
            "failed with empty stderr should have 'unknown' reason"
        );
    }

    // ── Helper function integration tests ────────────────────────────

    /// Create a tempdir with a valid `.assay/config.toml`.
    fn create_project(config_toml: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let assay_dir = dir.path().join(".assay");
        std::fs::create_dir_all(&assay_dir).unwrap();
        let mut f = std::fs::File::create(assay_dir.join("config.toml")).unwrap();
        f.write_all(config_toml.as_bytes()).unwrap();
        dir
    }

    /// Create a spec file inside a project's specs directory.
    fn create_spec(project_dir: &Path, specs_dir_name: &str, filename: &str, content: &str) {
        let specs_path = project_dir.join(".assay").join(specs_dir_name);
        std::fs::create_dir_all(&specs_path).unwrap();
        let mut f = std::fs::File::create(specs_path.join(filename)).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn test_load_config_valid_project() {
        let dir = create_project(r#"project_name = "test-project""#);
        let config = load_config(dir.path());

        assert!(
            config.is_ok(),
            "load_config should succeed for valid project, got: {:?}",
            config.err()
        );
        let config = config.unwrap();
        assert_eq!(config.project_name, "test-project");
    }

    #[test]
    fn test_load_config_missing_project() {
        let dir = tempfile::tempdir().unwrap();
        let result = load_config(dir.path());

        assert!(
            result.is_err(),
            "load_config should fail for missing .assay/"
        );
        let err_result = result.unwrap_err();
        assert!(
            err_result.is_error.unwrap_or(false),
            "should produce isError: true"
        );

        // Check that error text mentions the path
        let text: String = err_result
            .content
            .iter()
            .filter_map(|c| match &c.raw {
                RawContent::Text(t) => Some(t.text.as_str()),
                _ => None,
            })
            .collect();
        assert!(
            text.contains("config"),
            "error should mention config, got: {text}"
        );
    }

    #[test]
    fn test_load_spec_entry_not_found() {
        let dir = create_project(r#"project_name = "test""#);
        // Create the specs directory but no spec files
        std::fs::create_dir_all(dir.path().join(".assay").join("specs")).unwrap();

        let config = load_config(dir.path()).unwrap();
        let result = load_spec_entry_mcp(dir.path(), &config, "nonexistent");

        assert!(
            result.is_err(),
            "load_spec_entry_mcp should fail for nonexistent spec"
        );
        let err_result = result.unwrap_err();
        assert!(
            err_result.is_error.unwrap_or(false),
            "should produce isError: true"
        );

        let text: String = err_result
            .content
            .iter()
            .filter_map(|c| match &c.raw {
                RawContent::Text(t) => Some(t.text.as_str()),
                _ => None,
            })
            .collect();
        assert!(
            text.contains("nonexistent"),
            "error should mention the spec name, got: {text}"
        );
    }

    #[test]
    fn test_load_spec_entry_legacy() {
        let dir = create_project(r#"project_name = "test""#);
        create_spec(
            dir.path(),
            "specs",
            "auth-flow.toml",
            r#"
name = "auth-flow"
description = "Auth spec"

[[criteria]]
name = "unit-tests"
description = "Tests pass"
cmd = "echo ok"
"#,
        );

        let config = load_config(dir.path()).unwrap();
        let result = load_spec_entry_mcp(dir.path(), &config, "auth-flow");

        assert!(
            result.is_ok(),
            "load_spec_entry_mcp should succeed for valid spec, got: {:?}",
            result.err()
        );
        let entry = result.unwrap();
        assert!(matches!(entry, SpecEntry::Legacy { .. }));
        assert_eq!(entry.name(), "auth-flow");
    }

    #[test]
    fn test_load_spec_entry_directory() {
        let dir = create_project(r#"project_name = "test""#);
        let specs_dir = dir.path().join(".assay").join("specs").join("auth-dir");
        std::fs::create_dir_all(&specs_dir).unwrap();
        std::fs::write(
            specs_dir.join("gates.toml"),
            r#"
name = "auth-dir"

[[criteria]]
name = "compiles"
description = "Code compiles"
cmd = "echo ok"
"#,
        )
        .unwrap();

        let config = load_config(dir.path()).unwrap();
        let result = load_spec_entry_mcp(dir.path(), &config, "auth-dir");

        assert!(result.is_ok());
        let entry = result.unwrap();
        assert!(matches!(entry, SpecEntry::Directory { .. }));
        assert_eq!(entry.name(), "auth-dir");
    }

    #[test]
    fn test_resolve_working_dir_default() {
        let cwd = PathBuf::from("/some/project");
        let config = assay_types::Config {
            project_name: "test".to_string(),
            specs_dir: "specs/".to_string(),
            gates: None,
        };

        let result = resolve_working_dir(&cwd, &config);
        assert_eq!(result, cwd, "default should return cwd");
    }

    #[test]
    fn test_resolve_working_dir_relative() {
        let cwd = PathBuf::from("/some/project");
        let config = assay_types::Config {
            project_name: "test".to_string(),
            specs_dir: "specs/".to_string(),
            gates: Some(assay_types::GatesConfig {
                default_timeout: 300,
                working_dir: Some("subdir".to_string()),
                max_history: None,
            }),
        };

        let result = resolve_working_dir(&cwd, &config);
        assert_eq!(
            result,
            PathBuf::from("/some/project/subdir"),
            "relative dir should be joined to cwd"
        );
    }

    #[test]
    fn test_resolve_working_dir_absolute() {
        let cwd = PathBuf::from("/some/project");
        let config = assay_types::Config {
            project_name: "test".to_string(),
            specs_dir: "specs/".to_string(),
            gates: Some(assay_types::GatesConfig {
                default_timeout: 300,
                working_dir: Some("/tmp/custom".to_string()),
                max_history: None,
            }),
        };

        let result = resolve_working_dir(&cwd, &config);
        assert_eq!(
            result,
            PathBuf::from("/tmp/custom"),
            "absolute dir should be used as-is"
        );
    }

    // ── Response serialization tests ─────────────────────────────────

    #[test]
    fn test_spec_list_entry_serialization() {
        let entry = SpecListEntry {
            name: "auth-flow".to_string(),
            description: "Authentication flow".to_string(),
            criteria_count: 3,
            format: "legacy".to_string(),
            has_feature_spec: false,
        };

        let json = serde_json::to_value(&entry).unwrap();
        assert_eq!(json["name"], "auth-flow");
        assert_eq!(json["description"], "Authentication flow");
        assert_eq!(json["criteria_count"], 3);
        assert_eq!(json["format"], "legacy");
        assert!(
            json.get("has_feature_spec").is_none(),
            "false has_feature_spec should be omitted"
        );
    }

    #[test]
    fn test_spec_list_entry_omits_empty_description() {
        let entry = SpecListEntry {
            name: "bare-spec".to_string(),
            description: String::new(),
            criteria_count: 1,
            format: "legacy".to_string(),
            has_feature_spec: false,
        };

        let json = serde_json::to_value(&entry).unwrap();
        assert_eq!(json["name"], "bare-spec");
        assert!(
            json.get("description").is_none(),
            "empty description should be omitted, got: {json}"
        );
        assert_eq!(json["criteria_count"], 1);
    }

    #[test]
    fn test_spec_list_entry_directory_format() {
        let entry = SpecListEntry {
            name: "auth-dir".to_string(),
            description: String::new(),
            criteria_count: 2,
            format: "directory".to_string(),
            has_feature_spec: true,
        };

        let json = serde_json::to_value(&entry).unwrap();
        assert_eq!(json["format"], "directory");
        assert_eq!(json["has_feature_spec"], true);
    }

    #[test]
    fn test_gate_run_response_serialization() {
        let response = GateRunResponse {
            spec_name: "auth-flow".to_string(),
            passed: 2,
            failed: 1,
            skipped: 1,
            required_failed: 1,
            advisory_failed: 0,
            total_duration_ms: 1500,
            criteria: vec![
                CriterionSummary {
                    name: "unit-tests".to_string(),
                    status: "passed".to_string(),
                    enforcement: "required".to_string(),
                    kind_label: Some("cmd".to_string()),
                    exit_code: Some(0),
                    duration_ms: Some(800),
                    reason: None,
                    stdout: None,
                    stderr: None,
                },
                CriterionSummary {
                    name: "lint".to_string(),
                    status: "failed".to_string(),
                    enforcement: "required".to_string(),
                    kind_label: Some("cmd".to_string()),
                    exit_code: Some(1),
                    duration_ms: Some(700),
                    reason: Some("error: unused variable".to_string()),
                    stdout: None,
                    stderr: None,
                },
                CriterionSummary {
                    name: "review".to_string(),
                    status: "skipped".to_string(),
                    enforcement: "required".to_string(),
                    kind_label: None,
                    exit_code: None,
                    duration_ms: None,
                    reason: None,
                    stdout: None,
                    stderr: None,
                },
            ],
            session_id: None,
            pending_criteria: None,
        };

        let json = serde_json::to_value(&response).unwrap();

        // Top-level fields
        assert_eq!(json["spec_name"], "auth-flow");
        assert_eq!(json["passed"], 2);
        assert_eq!(json["failed"], 1);
        assert_eq!(json["skipped"], 1);
        assert_eq!(json["total_duration_ms"], 1500);

        // No session fields when no agent criteria
        assert!(
            json.get("session_id").is_none(),
            "session_id should be omitted: {json}"
        );
        assert!(
            json.get("pending_criteria").is_none(),
            "pending_criteria should be omitted: {json}"
        );

        // Passed criterion: has kind_label, no reason, no stdout, no stderr in JSON
        let passed = &json["criteria"][0];
        assert_eq!(passed["name"], "unit-tests");
        assert_eq!(passed["status"], "passed");
        assert_eq!(passed["kind_label"], "cmd");
        assert_eq!(passed["exit_code"], 0);
        assert!(
            passed.get("reason").is_none(),
            "passed should not have reason: {passed}"
        );
        assert!(
            passed.get("stdout").is_none(),
            "summary mode should not have stdout: {passed}"
        );
        assert!(
            passed.get("stderr").is_none(),
            "summary mode should not have stderr: {passed}"
        );

        // Failed criterion: has reason, no stdout/stderr
        let failed = &json["criteria"][1];
        assert_eq!(failed["name"], "lint");
        assert_eq!(failed["reason"], "error: unused variable");
        assert!(
            failed.get("stdout").is_none(),
            "summary mode should not have stdout: {failed}"
        );

        // Skipped criterion: minimal fields only, no kind_label
        let skipped = &json["criteria"][2];
        assert_eq!(skipped["name"], "review");
        assert_eq!(skipped["status"], "skipped");
        assert!(
            skipped.get("kind_label").is_none(),
            "skipped should not have kind_label: {skipped}"
        );
        assert!(
            skipped.get("exit_code").is_none(),
            "skipped should not have exit_code: {skipped}"
        );
        assert!(
            skipped.get("duration_ms").is_none(),
            "skipped should not have duration_ms: {skipped}"
        );
        assert!(
            skipped.get("reason").is_none(),
            "skipped should not have reason: {skipped}"
        );
    }

    #[test]
    fn test_gate_run_response_with_evidence() {
        let response = GateRunResponse {
            spec_name: "test".to_string(),
            passed: 1,
            failed: 0,
            skipped: 0,
            required_failed: 0,
            advisory_failed: 0,
            total_duration_ms: 500,
            criteria: vec![CriterionSummary {
                name: "check".to_string(),
                status: "passed".to_string(),
                enforcement: "required".to_string(),
                kind_label: Some("cmd".to_string()),
                exit_code: Some(0),
                duration_ms: Some(500),
                reason: None,
                stdout: Some("all tests passed".to_string()),
                stderr: Some(String::new()),
            }],
            session_id: None,
            pending_criteria: None,
        };

        let json = serde_json::to_value(&response).unwrap();
        let criterion = &json["criteria"][0];

        assert_eq!(criterion["stdout"], "all tests passed");
        assert!(
            criterion.get("stderr").is_some(),
            "evidence mode should include stderr even when empty"
        );
    }

    // ── New tool response serialization tests ───────────────────────

    #[test]
    fn test_gate_report_response_serialization() {
        let response = GateReportResponse {
            session_id: "20260305T220000Z-abc123".to_string(),
            criterion_name: "code-review".to_string(),
            accepted: true,
            evaluations_count: 1,
            pending_criteria: vec!["arch-review".to_string()],
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["session_id"], "20260305T220000Z-abc123");
        assert_eq!(json["criterion_name"], "code-review");
        assert_eq!(json["accepted"], true);
        assert_eq!(json["evaluations_count"], 1);
        assert_eq!(json["pending_criteria"][0], "arch-review");
    }

    #[test]
    fn test_gate_run_response_with_session() {
        let response = GateRunResponse {
            spec_name: "mixed-spec".to_string(),
            passed: 1,
            failed: 0,
            skipped: 1,
            required_failed: 0,
            advisory_failed: 0,
            total_duration_ms: 500,
            criteria: vec![],
            session_id: Some("20260305T220000Z-abc123".to_string()),
            pending_criteria: Some(vec!["code-review".to_string()]),
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["session_id"], "20260305T220000Z-abc123");
        assert_eq!(json["pending_criteria"][0], "code-review");
    }

    #[test]
    fn test_kind_label_command() {
        assert_eq!(
            kind_label(&GateKind::Command {
                cmd: "echo ok".to_string()
            }),
            Some("cmd".to_string())
        );
    }

    #[test]
    fn test_kind_label_file_exists() {
        assert_eq!(
            kind_label(&GateKind::FileExists {
                path: "README.md".to_string()
            }),
            Some("file".to_string())
        );
    }

    #[test]
    fn test_kind_label_agent_report() {
        assert_eq!(
            kind_label(&GateKind::AgentReport),
            Some("agent".to_string())
        );
    }

    #[test]
    fn test_kind_label_always_pass() {
        assert_eq!(kind_label(&GateKind::AlwaysPass), None);
    }

    #[test]
    fn test_extract_agent_criteria_none_for_cmd_only() {
        let entry = SpecEntry::Legacy {
            slug: "test".to_string(),
            spec: assay_types::Spec {
                name: "test".to_string(),
                description: String::new(),
                gate: None,
                criteria: vec![assay_types::Criterion {
                    name: "builds".to_string(),
                    description: "builds".to_string(),
                    cmd: Some("cargo build".to_string()),
                    path: None,
                    timeout: None,
                    enforcement: None,
                    kind: None,
                    prompt: None,
                }],
            },
        };

        assert!(
            extract_agent_criteria_info(&entry).is_none(),
            "cmd-only spec should not have agent criteria"
        );
    }

    #[test]
    fn test_extract_agent_criteria_some_for_mixed_spec() {
        let entry = SpecEntry::Legacy {
            slug: "mixed".to_string(),
            spec: assay_types::Spec {
                name: "mixed".to_string(),
                description: String::new(),
                gate: None,
                criteria: vec![
                    assay_types::Criterion {
                        name: "builds".to_string(),
                        description: "builds".to_string(),
                        cmd: Some("cargo build".to_string()),
                        path: None,
                        timeout: None,
                        enforcement: None,
                        kind: None,
                        prompt: None,
                    },
                    assay_types::Criterion {
                        name: "code-review".to_string(),
                        description: "Agent reviews code".to_string(),
                        cmd: None,
                        path: None,
                        timeout: None,
                        enforcement: None,
                        kind: Some(CriterionKind::AgentReport),
                        prompt: Some("Review for issues".to_string()),
                    },
                ],
            },
        };

        let info = extract_agent_criteria_info(&entry);
        assert!(info.is_some(), "mixed spec should have agent criteria");
        let info = info.unwrap();
        assert!(info.agent_criteria_names.contains("code-review"));
        assert!(!info.agent_criteria_names.contains("builds"));
        assert_eq!(info.spec_enforcement.len(), 2);
    }

    #[test]
    fn test_format_gate_response_agent_result_kind_label() {
        let summary = GateRunSummary {
            spec_name: "agent-test".to_string(),
            results: vec![CriterionResult {
                criterion_name: "review".to_string(),
                result: Some(GateResult {
                    passed: true,
                    kind: GateKind::AgentReport,
                    stdout: String::new(),
                    stderr: String::new(),
                    exit_code: None,
                    duration_ms: 0,
                    timestamp: Utc::now(),
                    truncated: false,
                    original_bytes: None,
                    evidence: Some("found auth module".to_string()),
                    reasoning: Some("auth module uses JWT".to_string()),
                    confidence: Some(assay_types::Confidence::High),
                    evaluator_role: Some(EvaluatorRole::SelfEval),
                }),
                enforcement: Enforcement::Advisory,
            }],
            passed: 1,
            failed: 0,
            skipped: 0,
            total_duration_ms: 0,
            enforcement: EnforcementSummary::default(),
        };

        let response = format_gate_response(&summary, false);
        let cr = &response.criteria[0];
        assert_eq!(cr.kind_label.as_deref(), Some("agent"));
        assert_eq!(cr.status, "passed");
        assert_eq!(cr.enforcement, "advisory");
    }
}
