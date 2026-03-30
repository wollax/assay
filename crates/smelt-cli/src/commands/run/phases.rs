//! Execution phases for `smelt run` — the full container lifecycle.

use std::future::Future;

use anyhow::{Context, Result};
use tracing::{error, info, warn};

use smelt_core::config::SmeltConfig;
use smelt_core::manifest::{self, JobManifest};
use smelt_core::monitor::{JobMonitor, JobPhase, compute_job_timeout};
use smelt_core::{ForgeClient, GitHubForge};

use super::helpers::{ensure_gitignore_assay, should_create_pr};
use super::{AnyProvider, RunArgs};

/// Outcome of the `select!`-wrapped execution phase.
enum ExecOutcome {
    /// Exec completed normally (may still be an error).
    Completed(Result<i32>),
    /// Job timeout expired before exec finished.
    Timeout,
    /// Cancellation signal received.
    Cancelled,
}

/// Run the teardown sequence, logging warnings instead of silently discarding errors.
///
/// Always prints a "Tearing down container…" progress message to stderr.
/// Sets phase to `TearingDown`, calls `provider.teardown()`, then `monitor.cleanup()`.
/// Each step logs a warning on failure; prints a success message when
/// teardown succeeds.
async fn warn_teardown(
    monitor: &mut JobMonitor,
    provider: &AnyProvider,
    container: &smelt_core::provider::ContainerId,
) {
    use smelt_core::provider::RuntimeProvider;
    if let Err(e) = monitor.set_phase(JobPhase::TearingDown) {
        warn!(container = %container, error = %e, "failed to set TearingDown phase");
    }
    info!("Tearing down container...");
    if let Err(e) = provider.teardown(container).await {
        warn!(container = %container, error = %e, "teardown failed");
    } else {
        info!("Container removed.");
    }
    if let Err(e) = monitor.cleanup() {
        warn!(container = %container, error = %e, "monitor cleanup failed");
    }
}

/// Core run logic, parameterized on a cancellation future for testability.
///
/// In production, `cancel` is `tokio::signal::ctrl_c()`.
/// In tests, it can be a `tokio::sync::oneshot::Receiver` or similar.
pub async fn run_with_cancellation<F>(args: &RunArgs, cancel: F) -> Result<i32>
where
    F: Future<Output = std::io::Result<()>> + Send,
{
    use smelt_core::docker::DockerProvider;
    use smelt_core::provider::RuntimeProvider;

    // Phase 1: Load manifest
    info!(path = %args.manifest.display(), "loading manifest");
    let mut manifest = JobManifest::load(&args.manifest)
        .with_context(|| format!("failed to load manifest `{}`", args.manifest.display()))?;
    info!("manifest loaded successfully");

    // Merge runtime_env from dispatch context (e.g. SMELT_EVENT_URL, SMELT_JOB_ID)
    if !args.runtime_env.is_empty() {
        info!(
            count = args.runtime_env.len(),
            keys = ?args.runtime_env.keys().collect::<Vec<_>>(),
            "injecting runtime env vars from dispatch context"
        );
        manifest.runtime_env.extend(args.runtime_env.clone());
    }

    // Phase 2: Validate
    info!("validating manifest");
    if let Err(e) = manifest.validate() {
        error!(%e, path = %args.manifest.display(), "Validation failed");
        error!("{e}");
        return Ok(1);
    }
    info!("manifest validation passed");

    // Phase 3.5: Ensure .assay/ is in .gitignore for the repo
    if let Ok(repo_path) = manifest::resolve_repo_path(&manifest.job.repo)
        && let Err(e) = ensure_gitignore_assay(&repo_path)
    {
        warn!(error = %e, "could not update .gitignore");
    }

    // Compute state dir and timeout
    let state_dir = args
        .manifest
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join(".smelt")
        .join("runs")
        .join(&manifest.job.name);
    let timeout_duration = compute_job_timeout(&manifest, SmeltConfig::default().default_timeout);
    let session_names: Vec<String> = manifest.session.iter().map(|s| s.name.clone()).collect();

    // Create monitor
    let mut monitor = JobMonitor::new(&manifest.job.name, session_names, &state_dir);
    monitor
        .write()
        .context("failed to write initial monitor state")?;

    // Phase 3 + 4: Connect to runtime provider
    info!("Provisioning container...");
    let provider: AnyProvider = match manifest.environment.runtime.as_str() {
        "docker" => AnyProvider::Docker(
            DockerProvider::new().with_context(|| "failed to connect to Docker daemon")?,
        ),
        "compose" => AnyProvider::Compose(
            smelt_core::ComposeProvider::new()
                .with_context(|| "failed to connect to Docker daemon")?,
        ),
        "kubernetes" => AnyProvider::Kubernetes(
            smelt_core::KubernetesProvider::new(&manifest)
                .await
                .with_context(|| "failed to connect to Kubernetes cluster")?,
        ),
        other => {
            error!("Error: unsupported runtime `{other}`. Supported: docker, compose, kubernetes.");
            return Ok(1);
        }
    };

    // Phase 5: Provision container
    let provision_result = provider
        .provision(&manifest)
        .await
        .with_context(|| "failed to provision container")?;
    let container = provision_result.container_id;
    if let Some(ref ip) = provision_result.container_ip {
        info!(container_ip = %ip, "container IP discovered for signal delivery");
        // Report container IP back to dispatch for signal_urls caching.
        // Use .ok() on the Mutex lock — a poisoned mutex here is non-fatal;
        // signal URL caching will simply not happen for this job.
        if let Ok(mut guard) = args.container_ip_tx.lock()
            && let Some(tx) = guard.take()
        {
            let _ = tx.send(ip.clone());
        }
    }
    info!("Container provisioned: {container}");
    monitor.set_container(container.as_str());

    // From here, teardown must run regardless of what happens.
    // Pin the cancel future so it can be polled in select!
    tokio::pin!(cancel);

    // Phase 5.5: Assay setup — config, specs dir, per-session spec files

    // Write assay config into container (idempotent — skips if already present)
    info!("Writing assay config...");
    let config_cmd = smelt_core::AssayInvoker::build_write_assay_config_command(&manifest.job.name);
    match provider.exec(&container, &config_cmd).await {
        Err(e) => {
            let _ = monitor.set_phase(JobPhase::Failed);
            warn_teardown(&mut monitor, &provider, &container).await;
            return Err(e).with_context(|| "failed to exec assay config write");
        }
        Ok(handle) if handle.exit_code != 0 => {
            let _ = monitor.set_phase(JobPhase::Failed);
            warn_teardown(&mut monitor, &provider, &container).await;
            return Err(anyhow::anyhow!(
                "assay config write exited with code {} in container {container}: stderr={}",
                handle.exit_code,
                handle.stderr.trim()
            ));
        }
        Ok(_) => {}
    }

    // Ensure the specs directory exists inside the container
    info!("Writing specs dir...");
    let specs_dir_cmd = smelt_core::AssayInvoker::build_ensure_specs_dir_command();
    match provider.exec(&container, &specs_dir_cmd).await {
        Err(e) => {
            let _ = monitor.set_phase(JobPhase::Failed);
            warn_teardown(&mut monitor, &provider, &container).await;
            return Err(e).with_context(|| "failed to exec specs dir creation");
        }
        Ok(handle) if handle.exit_code != 0 => {
            let _ = monitor.set_phase(JobPhase::Failed);
            warn_teardown(&mut monitor, &provider, &container).await;
            return Err(anyhow::anyhow!(
                "specs dir creation exited with code {} in container {container}: stderr={}",
                handle.exit_code,
                handle.stderr.trim()
            ));
        }
        Ok(_) => {}
    }

    // Write per-session spec TOML files into the container
    for s in manifest.session.iter() {
        let spec_name = smelt_core::AssayInvoker::sanitize_session_name(&s.name);
        let spec_toml = smelt_core::AssayInvoker::build_spec_toml(s);
        info!("Writing spec: {spec_name}...");
        if let Err(e) = smelt_core::AssayInvoker::write_spec_file_to_container(
            &provider, &container, &spec_name, &spec_toml,
        )
        .await
        {
            let _ = monitor.set_phase(JobPhase::Failed);
            warn_teardown(&mut monitor, &provider, &container).await;
            return Err(e)
                .with_context(|| format!("failed to write spec '{spec_name}' to container"));
        }
    }

    // Phase 6: Write assay manifest into container
    monitor
        .set_phase(JobPhase::WritingManifest)
        .context("failed to set WritingManifest phase")?;
    info!("Writing manifest...");
    let toml_content = smelt_core::AssayInvoker::build_run_manifest_toml(&manifest);
    let write_result =
        smelt_core::AssayInvoker::write_manifest_to_container(&provider, &container, &toml_content)
            .await;

    if let Err(e) = write_result {
        let _ = monitor.set_phase(JobPhase::Failed);
        warn_teardown(&mut monitor, &provider, &container).await;
        return Err(e).with_context(|| "failed to write assay manifest to container");
    }
    info!("Manifest written.");

    // Phase 7: Execute assay run with timeout + cancellation
    monitor
        .set_phase(JobPhase::Executing)
        .context("failed to set Executing phase")?;
    info!("Executing assay run...");

    let cmd = smelt_core::AssayInvoker::build_run_command(&manifest);
    let exec_future = async {
        let handle = provider
            .exec_streaming(&container, &cmd, |chunk| eprint!("{chunk}"))
            .await
            .with_context(|| "failed to execute assay run")?;
        let assay_exit = handle.exit_code;
        if assay_exit == 2 {
            info!("Assay complete — gate failures (exit 2)");
        } else {
            info!("Assay complete — exit code: {assay_exit}");
        }

        if assay_exit != 0 && assay_exit != 2 {
            anyhow::bail!(
                "assay run exited with code {assay_exit} — stderr: {}",
                handle.stderr.trim()
            );
        }

        // Collect results
        monitor
            .set_phase(JobPhase::Collecting)
            .context("failed to set Collecting phase")?;
        info!("Collecting results...");
        let repo_path = manifest::resolve_repo_path(&manifest.job.repo)
            .with_context(|| "failed to resolve repo path for collection")?;
        let git_binary =
            which::which("git").with_context(|| "git not found on PATH during collection")?;
        let git = smelt_core::GitCli::new(git_binary, repo_path.clone());

        if manifest.environment.runtime == "kubernetes" {
            tracing::info!(branch = %manifest.merge.target, "fetching result branch from remote");
            use smelt_core::GitOps as _;
            git.fetch_ref("origin", &format!("+{t}:{t}", t = manifest.merge.target))
                .await
                .with_context(|| "Phase 8: failed to fetch result branch from remote")?;
        }

        let collector = smelt_core::ResultCollector::new(git, repo_path);
        let collect_result = collector
            .collect(&manifest.job.base_ref, &manifest.merge.target)
            .await
            .with_context(|| "failed to collect results")?;

        if collect_result.no_changes {
            info!("No new commits from Assay — target branch not created");
        } else {
            info!(
                "Collected: {} commits on branch '{}', {} files changed",
                collect_result.commit_count,
                collect_result.branch,
                collect_result.files_changed.len(),
            );
        }

        // Phase 9: Create GitHub PR if forge is configured
        if should_create_pr(
            args.no_pr,
            collect_result.no_changes,
            manifest.forge.as_ref(),
        ) {
            let forge_cfg = manifest.forge.as_ref().unwrap();
            let token = std::env::var(&forge_cfg.token_env).map_err(|_| {
                anyhow::anyhow!(
                    "env var {} not set — required for PR creation (forge.token_env)",
                    forge_cfg.token_env
                )
            })?;
            let github = GitHubForge::new(token)
                .with_context(|| "Phase 9: failed to initialise GitHub forge client")?;
            let job_name = &manifest.job.name;
            let head = &collect_result.branch;
            let base = &manifest.job.base_ref;
            let title = format!("[smelt] {} — {} → {}", job_name, head, base);
            let body = format!("Automated results from smelt job '{job_name}'.\n\nBase: `{base}`");
            info!("Creating PR: {} → {}...", head, base);
            let pr = github
                .create_pr(&forge_cfg.repo, head, base, &title, &body)
                .await
                .with_context(|| "Phase 9: failed to create GitHub PR")?;
            monitor.state.pr_url = Some(pr.url.clone());
            monitor.state.pr_number = Some(pr.number);
            monitor.state.forge_repo = Some(forge_cfg.repo.clone());
            monitor.state.forge_token_env = Some(forge_cfg.token_env.clone());
            monitor
                .write()
                .context("failed to write monitor state after PR creation")?;
            info!("PR created: {}", pr.url);
        }

        Ok::<i32, anyhow::Error>(assay_exit)
    };

    let outcome = tokio::select! {
        result = exec_future => ExecOutcome::Completed(result),
        _ = tokio::time::sleep(timeout_duration) => ExecOutcome::Timeout,
        _ = &mut cancel => ExecOutcome::Cancelled,
    };

    // Map outcome to result + update monitor phase
    let result = match outcome {
        ExecOutcome::Completed(Ok(2)) => {
            let _ = monitor.set_phase(JobPhase::GatesFailed);
            Ok(2)
        }
        ExecOutcome::Completed(Ok(code)) => {
            let _ = monitor.set_phase(JobPhase::Complete);
            Ok(code)
        }
        ExecOutcome::Completed(Err(e)) => {
            let _ = monitor.set_phase(JobPhase::Failed);
            Err(e)
        }
        ExecOutcome::Timeout => {
            let _ = monitor.set_phase(JobPhase::Timeout);
            warn!("Timeout — tearing down...");
            Err(anyhow::anyhow!(
                "job timed out after {}s",
                timeout_duration.as_secs()
            ))
        }
        ExecOutcome::Cancelled => {
            let _ = monitor.set_phase(JobPhase::Cancelled);
            warn!("Cancelled — tearing down...");
            Err(anyhow::anyhow!("job cancelled by signal"))
        }
    };

    // Teardown — always runs
    if let Err(e) = monitor.set_phase(JobPhase::TearingDown) {
        warn!(container = %container, error = %e, "failed to set TearingDown phase");
    }
    info!("Tearing down container...");
    if let Err(e) = provider.teardown(&container).await {
        warn!(container = %container, error = %e, "teardown failed");
        if result.is_ok() {
            if let Err(e) = monitor.cleanup() {
                warn!(container = %container, error = %e, "monitor cleanup failed");
            }
            return Err(e.into());
        }
    } else {
        info!("Container removed.");
    }
    if let Err(e) = monitor.cleanup() {
        warn!(container = %container, error = %e, "monitor cleanup failed");
    }

    result
}
