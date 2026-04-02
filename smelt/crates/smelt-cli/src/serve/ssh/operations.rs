//! Free functions for SSH-based manifest delivery, state sync, and remote
//! job execution.

use anyhow::anyhow;
use tracing::{debug, warn};

use crate::serve::config::WorkerConfig;
use crate::serve::types::JobId;

use super::SshClient;

/// SCP the local manifest file to `/tmp/smelt-<job_id>.toml` on the worker.
///
/// Returns the remote path string on success.
pub async fn deliver_manifest<C: SshClient>(
    client: &C,
    worker: &WorkerConfig,
    timeout_secs: u64,
    job_id: &JobId,
    local_manifest: &std::path::Path,
) -> anyhow::Result<String> {
    let remote_path = format!("/tmp/smelt-{}.toml", job_id);
    let remote_dest = format!("{}@{}:{}", worker.user, worker.host, remote_path);
    client
        .scp_to(worker, timeout_secs, local_manifest, &remote_dest)
        .await?;
    Ok(remote_path)
}

/// SCP the remote `.smelt/runs/<job_name>/` directory back to the local
/// filesystem so `smelt status` can read job state after remote execution.
///
/// `job_name` is the manifest's `[job] name` field (not the queue `JobId`) —
/// it must match what `smelt run` uses to compute its state directory.
///
/// Creates the local target directory (`local_dest_dir/.smelt/runs/<job_name>/`)
/// before calling `scp_from`.
pub async fn sync_state_back<C: SshClient>(
    client: &C,
    worker: &WorkerConfig,
    timeout_secs: u64,
    job_name: &str,
    local_dest_dir: &std::path::Path,
) -> anyhow::Result<()> {
    let remote_src = format!("/tmp/.smelt/runs/{}/", job_name);
    let local_target = local_dest_dir.join(".smelt/runs").join(job_name);

    debug!(
        host = %worker.host,
        job_name = %job_name,
        local_dest_dir = %local_dest_dir.display(),
        "sync_state_back entry"
    );

    std::fs::create_dir_all(&local_target).map_err(|e| {
        anyhow!(
            "failed to create local state dir {}: {}",
            local_target.display(),
            e
        )
    })?;

    client
        .scp_from(worker, timeout_secs, &remote_src, &local_target)
        .await
}

/// SSH exec `smelt run <remote_manifest_path>` on the worker, returning the
/// raw exit code.  Callers map 0/2/other per D050.
pub async fn run_remote_job<C: SshClient>(
    client: &C,
    worker: &WorkerConfig,
    timeout_secs: u64,
    remote_manifest_path: &str,
) -> anyhow::Result<i32> {
    let cmd = format!("smelt run {}", remote_manifest_path);
    let output = client.exec(worker, timeout_secs, &cmd).await?;
    if output.exit_code == 127 {
        warn!(
            host = %worker.host,
            cmd = %cmd,
            "exit code 127: smelt may not be on the remote PATH"
        );
    }
    Ok(output.exit_code)
}
