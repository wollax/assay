//! `smelt run` subcommand — execute a job manifest.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use tracing::{error, info};

use smelt_core::manifest::{CredentialStatus, JobManifest};

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

/// Load, validate, and run the full Docker lifecycle for a manifest.
async fn execute_run(args: &RunArgs) -> Result<i32> {
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

    // From here, teardown must run regardless of what happens.
    let result = async {
        // Phase 6: Execute health-check command
        eprintln!("Executing health check...");
        let cmd = vec!["echo".to_string(), "smelt: container ready".to_string()];
        let handle = provider
            .exec(&container, &cmd)
            .await
            .with_context(|| "failed to execute health check")?;
        eprintln!(
            "Health check complete — exit code: {}",
            handle.exit_code
        );

        if handle.exit_code != 0 {
            anyhow::bail!(
                "health check exited with code {} — stderr: {}",
                handle.exit_code,
                handle.stderr.trim()
            );
        }

        Ok::<i32, anyhow::Error>(0)
    }
    .await;

    // Phase 7: Teardown — always runs
    eprintln!("Tearing down container...");
    if let Err(e) = provider.teardown(&container).await {
        eprintln!("Warning: teardown failed: {e:#}");
        // If the inner result was ok, promote teardown error
        if result.is_ok() {
            return Err(e.into());
        }
    }
    eprintln!("Container removed.");

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
