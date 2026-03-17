//! `smelt run` subcommand — execute a job manifest.

use std::future::Future;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use tracing::{error, info};

use smelt_core::manifest::{self, CredentialStatus, JobManifest};
use smelt_core::monitor::{JobMonitor, JobPhase, compute_job_timeout};
use smelt_core::config::SmeltConfig;

/// Run a job manifest.
#[derive(Debug, Args)]
pub struct RunArgs {
    /// Path to the job manifest TOML file.
    pub manifest: PathBuf,

    /// Validate and print the execution plan without running anything.
    #[arg(long)]
    pub dry_run: bool,
}

/// Execute the `run` subcommand.
pub async fn execute(args: &RunArgs) -> Result<i32> {
    if args.dry_run {
        execute_dry_run(args)
    } else {
        execute_run(args).await
    }
}

/// Outcome of the `select!`-wrapped execution phase.
enum ExecOutcome {
    /// Exec completed normally (may still be an error).
    Completed(Result<i32>),
    /// Job timeout expired before exec finished.
    Timeout,
    /// Cancellation signal received.
    Cancelled,
}

/// Load, validate, and run the full Docker lifecycle for a manifest.
async fn execute_run(args: &RunArgs) -> Result<i32> {
    run_with_cancellation(args, tokio::signal::ctrl_c()).await
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
    let manifest = JobManifest::load(&args.manifest)
        .with_context(|| format!("failed to load manifest `{}`", args.manifest.display()))?;
    info!("manifest loaded successfully");

    // Phase 2: Validate
    info!("validating manifest");
    if let Err(e) = manifest.validate() {
        error!(%e, "manifest validation failed");
        eprintln!("Validation failed for `{}`:\n", args.manifest.display());
        eprintln!("{e}");
        return Ok(1);
    }
    info!("manifest validation passed");

    // Phase 3: Runtime type check
    if manifest.environment.runtime != "docker" {
        eprintln!(
            "Error: unsupported runtime `{}`. Only `docker` is currently supported.",
            manifest.environment.runtime
        );
        return Ok(1);
    }

    // Compute state dir and timeout
    let state_dir = args
        .manifest
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join(".smelt");
    let timeout_duration = compute_job_timeout(&manifest, SmeltConfig::default().default_timeout);
    let session_names: Vec<String> = manifest.session.iter().map(|s| s.name.clone()).collect();

    // Create monitor
    let mut monitor = JobMonitor::new(&manifest.job.name, session_names, &state_dir);
    monitor.write().map_err(|e| anyhow::anyhow!("{e}"))?;

    // Phase 4: Connect to Docker
    eprintln!("Provisioning container...");
    let provider = DockerProvider::new()
        .with_context(|| "failed to connect to Docker daemon")?;

    // Phase 5: Provision container
    let container = provider
        .provision(&manifest)
        .await
        .with_context(|| "failed to provision container")?;
    eprintln!("Container provisioned: {container}");
    monitor.set_container(container.as_str());

    // From here, teardown must run regardless of what happens.
    // Pin the cancel future so it can be polled in select!
    tokio::pin!(cancel);

    // Phase 6: Write assay manifest into container
    monitor
        .set_phase(JobPhase::WritingManifest)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    eprintln!("Writing manifest...");
    let toml_content = smelt_core::AssayInvoker::build_manifest_toml(&manifest);
    let write_result = smelt_core::AssayInvoker::write_manifest_to_container(
        &provider,
        &container,
        &toml_content,
    )
    .await;

    if let Err(e) = write_result {
        let _ = monitor.set_phase(JobPhase::Failed);
        eprintln!("Tearing down container...");
        let _ = monitor.set_phase(JobPhase::TearingDown);
        let _ = provider.teardown(&container).await;
        eprintln!("Container removed.");
        let _ = monitor.cleanup();
        return Err(e).with_context(|| "failed to write assay manifest to container");
    }
    eprintln!("Manifest written.");

    // Phase 7: Execute assay run with timeout + cancellation
    monitor
        .set_phase(JobPhase::Executing)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    eprintln!("Executing assay run...");

    let cmd = smelt_core::AssayInvoker::build_run_command(&manifest);
    let exec_future = async {
        let handle = provider
            .exec(&container, &cmd)
            .await
            .with_context(|| "failed to execute assay run")?;
        eprintln!("Assay complete — exit code: {}", handle.exit_code);

        if !handle.stdout.is_empty() {
            eprint!("{}", handle.stdout);
        }
        if !handle.stderr.is_empty() {
            eprint!("{}", handle.stderr);
        }

        if handle.exit_code != 0 {
            anyhow::bail!(
                "assay run exited with code {} — stderr: {}",
                handle.exit_code,
                handle.stderr.trim()
            );
        }

        // Collect results
        monitor
            .set_phase(JobPhase::Collecting)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        eprintln!("Collecting results...");
        let repo_path = manifest::resolve_repo_path(&manifest.job.repo)
            .with_context(|| "failed to resolve repo path for collection")?;
        let git_binary = which::which("git")
            .with_context(|| "git not found on PATH during collection")?;
        let git = smelt_core::GitCli::new(git_binary, repo_path.clone());
        let collector = smelt_core::ResultCollector::new(git, repo_path);
        let collect_result = collector
            .collect(&manifest.job.base_ref, &manifest.merge.target)
            .await
            .with_context(|| "failed to collect results")?;

        if collect_result.no_changes {
            eprintln!("No new commits from Assay — target branch not created");
        } else {
            eprintln!(
                "Collected: {} commits on branch '{}', {} files changed",
                collect_result.commit_count,
                collect_result.branch,
                collect_result.files_changed.len(),
            );
        }

        Ok::<i32, anyhow::Error>(0)
    };

    let outcome = tokio::select! {
        result = exec_future => ExecOutcome::Completed(result),
        _ = tokio::time::sleep(timeout_duration) => ExecOutcome::Timeout,
        _ = &mut cancel => ExecOutcome::Cancelled,
    };

    // Map outcome to result + update monitor phase
    let result = match outcome {
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
            eprintln!("Timeout — tearing down...");
            Err(anyhow::anyhow!(
                "job timed out after {}s",
                timeout_duration.as_secs()
            ))
        }
        ExecOutcome::Cancelled => {
            let _ = monitor.set_phase(JobPhase::Cancelled);
            eprintln!("Cancelled — tearing down...");
            Err(anyhow::anyhow!("job cancelled by signal"))
        }
    };

    // Teardown — always runs
    let _ = monitor.set_phase(JobPhase::TearingDown);
    eprintln!("Tearing down container...");
    if let Err(e) = provider.teardown(&container).await {
        eprintln!("Warning: teardown failed: {e:#}");
        if result.is_ok() {
            let _ = monitor.cleanup();
            return Err(e.into());
        }
    }
    eprintln!("Container removed.");
    let _ = monitor.cleanup();

    result
}

/// Load, validate, and print the execution plan for a manifest.
fn execute_dry_run(args: &RunArgs) -> Result<i32> {
    // Phase 1: Load manifest
    info!(path = %args.manifest.display(), "loading manifest");
    let manifest = JobManifest::load(&args.manifest)
        .with_context(|| format!("failed to load manifest `{}`", args.manifest.display()))?;
    info!("manifest loaded successfully");

    // Phase 2: Validate
    info!("validating manifest");
    if let Err(e) = manifest.validate() {
        error!(%e, "manifest validation failed");
        eprintln!("Validation failed for `{}`:\n", args.manifest.display());
        eprintln!("{e}");
        return Ok(1);
    }
    info!("manifest validation passed");

    // Phase 3: Resolve credentials
    info!("resolving credentials");
    let credentials = manifest.resolve_credentials();

    // Phase 4: Print execution plan
    info!("printing execution plan");
    print_execution_plan(&manifest, &credentials);

    Ok(0)
}

/// Print a structured, human-readable execution plan.
fn print_execution_plan(
    manifest: &JobManifest,
    credentials: &std::collections::HashMap<String, CredentialStatus>,
) {
    println!("═══ Execution Plan ═══");
    println!();

    // ── Job ──
    println!("Job:          {}", manifest.job.name);
    println!("Repository:   {}", manifest.job.repo);
    println!("Base ref:     {}", manifest.job.base_ref);
    println!();

    // ── Environment ──
    println!("── Environment ──");
    println!("  Runtime:    {}", manifest.environment.runtime);
    println!("  Image:      {}", manifest.environment.image);
    if !manifest.environment.resources.is_empty() {
        println!("  Resources:");
        let mut resources: Vec<_> = manifest.environment.resources.iter().collect();
        resources.sort_by_key(|(k, _)| (*k).clone());
        for (key, value) in &resources {
            println!("    {key:<10} {value}");
        }
    }
    println!();

    // ── Credentials ──
    println!("── Credentials ──");
    println!("  Provider:   {}", manifest.credentials.provider);
    println!("  Model:      {}", manifest.credentials.model);
    if !credentials.is_empty() {
        let mut creds: Vec<_> = credentials.iter().collect();
        creds.sort_by_key(|(k, _)| (*k).clone());
        for (name, status) in &creds {
            println!("  {name:<12} {status}");
        }
    }
    println!();

    // ── Sessions ──
    println!("── Sessions ({}) ──", manifest.session.len());
    for (i, sess) in manifest.session.iter().enumerate() {
        println!("  [{}] {}", i + 1, sess.name);
        println!("      Spec:       {}", truncate_spec(&sess.spec, 72));
        println!("      Harness:    {}", sess.harness);
        println!("      Timeout:    {}s", sess.timeout);
        if !sess.depends_on.is_empty() {
            println!("      Depends on: {}", sess.depends_on.join(", "));
        }
    }
    println!();

    // ── Merge ──
    println!("── Merge ──");
    println!("  Strategy:      {}", manifest.merge.strategy);
    println!("  Target:        {}", manifest.merge.target);
    println!(
        "  AI resolution: {}",
        if manifest.merge.ai_resolution {
            "enabled"
        } else {
            "disabled"
        }
    );
    if !manifest.merge.order.is_empty() {
        println!("  Order:         {}", manifest.merge.order.join(" → "));
    }
    println!();
    println!("═══ End Plan ═══");
}

/// Truncate a spec string for display, appending "…" if shortened.
fn truncate_spec(s: &str, max_len: usize) -> String {
    let s = s.replace('\n', " ");
    if s.len() <= max_len {
        s
    } else {
        format!("{}…", &s[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_spec_short() {
        assert_eq!(truncate_spec("hello", 10), "hello");
    }

    #[test]
    fn truncate_spec_long() {
        let result = truncate_spec("a]b".repeat(50).as_str(), 10);
        assert!(result.len() <= 14); // 10 chars + "…" (3 bytes)
        assert!(result.ends_with('…'));
    }

    #[test]
    fn truncate_spec_newlines() {
        assert_eq!(truncate_spec("line1\nline2", 20), "line1 line2");
    }
}
