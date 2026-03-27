//! SshSyncBackend — persists orchestrator state to a remote host via `scp`/`ssh`.
//!
//! `ScpRunner` wraps `std::process::Command` for synchronous `scp` and `ssh`
//! invocations using `Command::arg()` chaining (no shell string interpolation
//! for user-supplied paths — see D163).
//!
//! `SshSyncBackend` implements [`StateBackend`] with `CapabilitySet::all()`,
//! mapping each method to one or more `scp`/`ssh` calls against the configured
//! remote host.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use assay_core::{AssayError, CapabilitySet, StateBackend};
use assay_types::{OrchestratorStatus, TeamCheckpoint};

// ---------------------------------------------------------------------------
// shell_quote — for arguments passed inside remote ssh command strings
// ---------------------------------------------------------------------------

/// Wrap a value in single quotes for safe embedding in a remote shell command.
///
/// Embedded single quotes are escaped using the `'\''` pattern:
/// close the current single-quote, emit an escaped single-quote, re-open.
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

// ---------------------------------------------------------------------------
// ScpRunner — low-level scp/ssh wrapper
// ---------------------------------------------------------------------------

/// Low-level wrapper around `scp` and `ssh` CLI commands.
///
/// All commands use `Command::arg()` chaining — no `sh -c` with user-supplied
/// path values. The `ssh_run` method accepts a pre-composed remote command
/// string that does pass through the remote shell; callers must use
/// [`shell_quote`] for any user-supplied path values embedded in that string.
struct ScpRunner {
    host: String,
    user: Option<String>,
    port: Option<u16>,
}

impl ScpRunner {
    /// Build the `user@host` or `host` spec.
    fn host_spec(&self) -> String {
        match &self.user {
            Some(user) => format!("{user}@{}", self.host),
            None => self.host.clone(),
        }
    }

    /// Build a `host_spec:remote_path` string for scp.
    fn remote_spec(&self, remote_path: &str) -> String {
        format!("{}:{}", self.host_spec(), remote_path)
    }

    /// Create a base `scp` command with optional port flag (`-P`, uppercase).
    fn build_scp_base(&self) -> Command {
        let mut cmd = Command::new("scp");
        if let Some(port) = self.port {
            cmd.arg("-P").arg(port.to_string());
        }
        cmd
    }

    /// Push a local file to a remote path via scp.
    fn scp_push(&self, local: &Path, remote_path: &str) -> assay_core::Result<()> {
        tracing::debug!(op = "scp_push", local = %local.display(), "scp push");

        let mut cmd = self.build_scp_base();
        cmd.arg(local).arg(self.remote_spec(remote_path));
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let output = cmd
            .output()
            .map_err(|e| AssayError::io("scp push", local, e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AssayError::io(
                format!("scp push failed: {}", stderr.trim()),
                local,
                std::io::Error::other(stderr.trim().to_string()),
            ));
        }

        Ok(())
    }

    /// Pull a remote file to a local path via scp.
    fn scp_pull(&self, remote_path: &str, local: &Path) -> assay_core::Result<()> {
        tracing::debug!(op = "scp_pull", local = %local.display(), "scp pull");

        let mut cmd = self.build_scp_base();
        cmd.arg(self.remote_spec(remote_path)).arg(local);
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let output = cmd
            .output()
            .map_err(|e| AssayError::io("scp pull", local, e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AssayError::io(
                format!("scp pull failed: {}", stderr.trim()),
                local,
                std::io::Error::other(stderr.trim().to_string()),
            ));
        }

        Ok(())
    }

    /// Run a command on the remote host via ssh and capture stdout.
    ///
    /// `remote_cmd` is a shell command string executed on the remote host.
    /// Since it passes through the remote shell, any user-supplied path values
    /// embedded in it **must** be wrapped with [`shell_quote`] to prevent
    /// injection and word-splitting.
    fn ssh_run(&self, remote_cmd: &str) -> assay_core::Result<String> {
        tracing::debug!(op = "ssh_run", "ssh run");

        let mut cmd = Command::new("ssh");
        if let Some(port) = self.port {
            cmd.arg("-p").arg(port.to_string());
        }
        cmd.arg(self.host_spec()).arg(remote_cmd);
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let output = cmd
            .output()
            .map_err(|e| AssayError::io("ssh run", remote_cmd, e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AssayError::io(
                format!("ssh run failed: {}", stderr.trim()),
                remote_cmd,
                std::io::Error::other(stderr.trim().to_string()),
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

// ---------------------------------------------------------------------------
// SshSyncBackend
// ---------------------------------------------------------------------------

/// Remote backend that persists orchestrator state via `scp`/`ssh` commands.
///
/// Each `StateBackend` method maps to one or more `scp push`/`pull` or `ssh`
/// invocations against the configured remote host. All paths with spaces are
/// safe: local paths use `Command::arg()` (no shell), remote paths inside ssh
/// commands use [`shell_quote`].
pub struct SshSyncBackend {
    runner: ScpRunner,
    remote_assay_dir: String,
    local_assay_dir: PathBuf,
}

impl std::fmt::Debug for SshSyncBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SshSyncBackend")
            .field("host", &self.runner.host)
            .field("remote_assay_dir", &self.remote_assay_dir)
            .finish()
    }
}

impl SshSyncBackend {
    /// Construct a new `SshSyncBackend`.
    pub fn new(
        host: String,
        remote_assay_dir: String,
        user: Option<String>,
        port: Option<u16>,
        local_assay_dir: PathBuf,
    ) -> Self {
        Self {
            runner: ScpRunner { host, user, port },
            remote_assay_dir,
            local_assay_dir,
        }
    }

    /// Map a local path to its corresponding remote path.
    ///
    /// Strips `local_assay_dir` prefix and joins the remainder with
    /// `remote_assay_dir`. Falls back to using the file name if the
    /// local path is outside `local_assay_dir`.
    fn to_remote_path(&self, local: &Path) -> String {
        if let Ok(relative) = local.strip_prefix(&self.local_assay_dir) {
            format!("{}/{}", self.remote_assay_dir, relative.display())
        } else {
            // Fallback: use file name only.
            let name = local
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string());
            format!("{}/{}", self.remote_assay_dir, name)
        }
    }

    /// Ensure a remote directory exists via `ssh mkdir -p`.
    fn ensure_remote_dir(&self, remote_dir: &str) -> assay_core::Result<()> {
        self.runner
            .ssh_run(&format!("mkdir -p {}", shell_quote(remote_dir)))
            .map(|_| ())
    }
}

impl StateBackend for SshSyncBackend {
    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet::all()
    }

    fn push_session_event(
        &self,
        run_dir: &Path,
        status: &OrchestratorStatus,
    ) -> assay_core::Result<()> {
        let json = serde_json::to_string_pretty(status)
            .map_err(|e| AssayError::json("serializing OrchestratorStatus", run_dir, e))?;

        let mut tmp = tempfile::NamedTempFile::new()
            .map_err(|e| AssayError::io("creating temp file for push_session_event", run_dir, e))?;
        tmp.write_all(json.as_bytes())
            .map_err(|e| AssayError::io("writing temp file for push_session_event", run_dir, e))?;

        let remote_run_dir = self.to_remote_path(run_dir);
        self.ensure_remote_dir(&remote_run_dir)?;

        let remote_state = format!("{}/state.json", remote_run_dir);
        self.runner.scp_push(tmp.path(), &remote_state)?;

        Ok(())
    }

    fn read_run_state(&self, run_dir: &Path) -> assay_core::Result<Option<OrchestratorStatus>> {
        let remote_state = format!("{}/state.json", self.to_remote_path(run_dir));

        let tmp = tempfile::NamedTempFile::new()
            .map_err(|e| AssayError::io("creating temp file for read_run_state", run_dir, e))?;

        // scp pull failure means file doesn't exist → Ok(None)
        if self.runner.scp_pull(&remote_state, tmp.path()).is_err() {
            return Ok(None);
        }

        let contents = std::fs::read_to_string(tmp.path())
            .map_err(|e| AssayError::io("reading pulled state.json", run_dir, e))?;

        let status: OrchestratorStatus = serde_json::from_str(&contents)
            .map_err(|e| AssayError::json("deserializing pulled state.json", run_dir, e))?;

        Ok(Some(status))
    }

    fn send_message(
        &self,
        inbox_path: &Path,
        name: &str,
        contents: &[u8],
    ) -> assay_core::Result<()> {
        let mut tmp = tempfile::NamedTempFile::new()
            .map_err(|e| AssayError::io("creating temp file for send_message", inbox_path, e))?;
        tmp.write_all(contents)
            .map_err(|e| AssayError::io("writing temp file for send_message", inbox_path, e))?;

        let remote_inbox = self.to_remote_path(inbox_path);
        self.ensure_remote_dir(&remote_inbox)?;

        let remote_file = format!("{}/{}", remote_inbox, name);
        self.runner.scp_push(tmp.path(), &remote_file)?;

        Ok(())
    }

    fn poll_inbox(&self, inbox_path: &Path) -> assay_core::Result<Vec<(String, Vec<u8>)>> {
        let remote_inbox = self.to_remote_path(inbox_path);

        // ls the remote inbox — if it doesn't exist, return empty.
        let listing = match self
            .runner
            .ssh_run(&format!("ls {}", shell_quote(&remote_inbox)))
        {
            Ok(output) => output,
            Err(_) => return Ok(vec![]),
        };

        let filenames: Vec<&str> = listing.lines().filter(|l| !l.is_empty()).collect();
        if filenames.is_empty() {
            return Ok(vec![]);
        }

        let mut messages = Vec::with_capacity(filenames.len());

        for filename in &filenames {
            let tmp = tempfile::NamedTempFile::new()
                .map_err(|e| AssayError::io("creating temp file for poll_inbox", inbox_path, e))?;

            let remote_file = format!("{}/{}", remote_inbox, filename);
            self.runner.scp_pull(&remote_file, tmp.path())?;

            let contents = std::fs::read(tmp.path())
                .map_err(|e| AssayError::io("reading pulled inbox message", inbox_path, e))?;

            // Remove the remote file — warn on failure, don't fail.
            let rm_cmd = format!("rm {}", shell_quote(&remote_file));
            if let Err(e) = self.runner.ssh_run(&rm_cmd) {
                tracing::warn!(
                    filename,
                    error = %e,
                    "failed to remove remote inbox message after pull — may be delivered twice"
                );
            }

            messages.push((filename.to_string(), contents));
        }

        Ok(messages)
    }

    fn annotate_run(&self, run_dir: &Path, manifest_path: &str) -> assay_core::Result<()> {
        let mut tmp = tempfile::NamedTempFile::new()
            .map_err(|e| AssayError::io("creating temp file for annotate_run", run_dir, e))?;
        tmp.write_all(manifest_path.as_bytes())
            .map_err(|e| AssayError::io("writing temp file for annotate_run", run_dir, e))?;

        let remote_run_dir = self.to_remote_path(run_dir);
        self.ensure_remote_dir(&remote_run_dir)?;

        let remote_annotation = format!("{}/annotation.txt", remote_run_dir);
        self.runner.scp_push(tmp.path(), &remote_annotation)?;

        Ok(())
    }

    fn save_checkpoint_summary(
        &self,
        assay_dir: &Path,
        checkpoint: &TeamCheckpoint,
    ) -> assay_core::Result<()> {
        let json = serde_json::to_string_pretty(checkpoint)
            .map_err(|e| AssayError::json("serializing TeamCheckpoint", assay_dir, e))?;

        let mut tmp = tempfile::NamedTempFile::new().map_err(|e| {
            AssayError::io(
                "creating temp file for save_checkpoint_summary",
                assay_dir,
                e,
            )
        })?;
        tmp.write_all(json.as_bytes()).map_err(|e| {
            AssayError::io(
                "writing temp file for save_checkpoint_summary",
                assay_dir,
                e,
            )
        })?;

        let remote_checkpoints = format!("{}/checkpoints", self.remote_assay_dir);
        self.ensure_remote_dir(&remote_checkpoints)?;

        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let remote_file = format!("{}/{}.json", remote_checkpoints, ts);
        self.runner.scp_push(tmp.path(), &remote_file)?;

        Ok(())
    }
}
