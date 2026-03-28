//! Dry-run mode for `smelt run --dry-run` — validates and prints the execution plan.

use anyhow::{Context, Result};
use tracing::{error, info};

use smelt_core::manifest::{CredentialStatus, JobManifest};

use super::RunArgs;

/// Load, validate, and print the execution plan for a manifest.
pub(super) fn execute_dry_run(args: &RunArgs) -> Result<i32> {
    // Phase 1: Load manifest
    info!(path = %args.manifest.display(), "loading manifest");
    let manifest = JobManifest::load(&args.manifest)
        .with_context(|| format!("failed to load manifest `{}`", args.manifest.display()))?;
    info!("manifest loaded successfully");

    // Phase 2: Validate
    info!("validating manifest");
    if let Err(e) = manifest.validate() {
        error!(%e, path = %args.manifest.display(), "Validation failed");
        error!("{e}");
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

    // ── Compose Services ──
    if !manifest.services.is_empty() {
        println!("── Compose Services ──");
        for svc in &manifest.services {
            println!("  {:<16} {}", svc.name, svc.image);
        }
        println!();
    }

    // ── Kubernetes ──
    if let Some(ref kube) = manifest.kubernetes {
        println!("── Kubernetes ──");
        println!("  Namespace:   {}", kube.namespace);
        println!(
            "  Context:     {}",
            kube.context.as_deref().unwrap_or("ambient")
        );
        if let Some(ref v) = kube.cpu_request {
            println!("  CPU req:     {v}");
        }
        if let Some(ref v) = kube.memory_request {
            println!("  Mem req:     {v}");
        }
        if let Some(ref v) = kube.cpu_limit {
            println!("  CPU limit:   {v}");
        }
        if let Some(ref v) = kube.memory_limit {
            println!("  Mem limit:   {v}");
        }
        println!();
    }

    // ── Forge ──
    if let Some(ref forge) = manifest.forge {
        println!("── Forge ──");
        println!("  Provider:    {}", forge.provider);
        println!("  Repo:        {}", forge.repo);
        println!("  Token env:   {}", forge.token_env);
        println!("  (use --no-pr to skip PR creation)");
        println!();
    }

    println!("═══ End Plan ═══");
}

/// Truncate a spec string for display: newlines are replaced with spaces,
/// and strings longer than `max_len` are truncated with "…".
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
