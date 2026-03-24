//! `smelt run` subcommand — execute a job manifest.

use std::future::Future;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use tracing::{error, info};

use smelt_core::forge::ForgeConfig;
use smelt_core::manifest::{self, CredentialStatus, JobManifest};
use smelt_core::monitor::{JobMonitor, JobPhase, compute_job_timeout};
use smelt_core::config::SmeltConfig;
use smelt_core::{ForgeClient, GitHubForge};

/// Run a job manifest.
#[derive(Debug, Args)]
pub struct RunArgs {
    /// Path to the job manifest TOML file.
    pub manifest: PathBuf,

    /// Validate and print the execution plan without running anything.
    #[arg(long)]
    pub dry_run: bool,

    /// Skip PR creation even when a `[forge]` section is present in the manifest.
    #[arg(long)]
    pub no_pr: bool,
}

// ── AnyProvider ──────────────────────────────────────────────────────────────

/// Dispatch enum that routes [`RuntimeProvider`] calls to the concrete backend
/// selected by `manifest.environment.runtime`.
///
/// A local enum avoids `Box<dyn RuntimeProvider>`, which is not object-safe
/// because `RuntimeProvider` has RPITIT `async fn` methods (see D019).
enum AnyProvider {
    Docker(smelt_core::DockerProvider),
    Compose(smelt_core::ComposeProvider),
    Kubernetes(smelt_core::KubernetesProvider),
}

impl smelt_core::provider::RuntimeProvider for AnyProvider {
    async fn provision(
        &self,
        manifest: &smelt_core::manifest::JobManifest,
    ) -> smelt_core::Result<smelt_core::provider::ContainerId> {
        match self {
            AnyProvider::Docker(p) => p.provision(manifest).await,
            AnyProvider::Compose(p) => p.provision(manifest).await,
            AnyProvider::Kubernetes(p) => p.provision(manifest).await,
        }
    }

    async fn exec(
        &self,
        container: &smelt_core::provider::ContainerId,
        command: &[String],
    ) -> smelt_core::Result<smelt_core::provider::ExecHandle> {
        match self {
            AnyProvider::Docker(p) => p.exec(container, command).await,
            AnyProvider::Compose(p) => p.exec(container, command).await,
            AnyProvider::Kubernetes(p) => p.exec(container, command).await,
        }
    }

    async fn exec_streaming<F>(
        &self,
        container: &smelt_core::provider::ContainerId,
        command: &[String],
        output_cb: F,
    ) -> smelt_core::Result<smelt_core::provider::ExecHandle>
    where
        F: FnMut(&str) + Send + 'static,
    {
        match self {
            AnyProvider::Docker(p) => p.exec_streaming(container, command, output_cb).await,
            AnyProvider::Compose(p) => p.exec_streaming(container, command, output_cb).await,
            AnyProvider::Kubernetes(p) => p.exec_streaming(container, command, output_cb).await,
        }
    }

    async fn collect(
        &self,
        container: &smelt_core::provider::ContainerId,
        manifest: &smelt_core::manifest::JobManifest,
    ) -> smelt_core::Result<smelt_core::provider::CollectResult> {
        match self {
            AnyProvider::Docker(p) => p.collect(container, manifest).await,
            AnyProvider::Compose(p) => p.collect(container, manifest).await,
            AnyProvider::Kubernetes(p) => p.collect(container, manifest).await,
        }
    }

    async fn teardown(
        &self,
        container: &smelt_core::provider::ContainerId,
    ) -> smelt_core::Result<()> {
        match self {
            AnyProvider::Docker(p) => p.teardown(container).await,
            AnyProvider::Compose(p) => p.teardown(container).await,
            AnyProvider::Kubernetes(p) => p.teardown(container).await,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────

/// Returns true when Phase 9 should attempt PR creation.
pub(crate) fn should_create_pr(no_pr: bool, no_changes: bool, forge: Option<&ForgeConfig>) -> bool {
    !no_pr && !no_changes && forge.is_some()
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

    // Phase 3.5: Ensure .assay/ is in .gitignore for the repo
    if let Ok(repo_path) = manifest::resolve_repo_path(&manifest.job.repo)
        && let Err(e) = ensure_gitignore_assay(&repo_path) {
            eprintln!("[WARN] could not update .gitignore: {e:#}");
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
    monitor.write().map_err(|e| anyhow::anyhow!("{e}"))?;

    // Phase 3 + 4: Connect to runtime provider
    eprintln!("Provisioning container...");
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
            eprintln!("Error: unsupported runtime `{other}`. Supported: docker, compose, kubernetes.");
            return Ok(1);
        }
    };

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

    // Phase 5.5: Assay setup — config, specs dir, per-session spec files

    // Write assay config into container (idempotent — skips if already present)
    eprintln!("Writing assay config...");
    let config_cmd = smelt_core::AssayInvoker::build_write_assay_config_command(&manifest.job.name);
    match provider.exec(&container, &config_cmd).await {
        Err(e) => {
            let _ = monitor.set_phase(JobPhase::Failed);
            eprintln!("Tearing down container...");
            let _ = monitor.set_phase(JobPhase::TearingDown);
            let _ = provider.teardown(&container).await;
            eprintln!("Container removed.");
            let _ = monitor.cleanup();
            return Err(e).with_context(|| "failed to exec assay config write");
        }
        Ok(handle) if handle.exit_code != 0 => {
            let _ = monitor.set_phase(JobPhase::Failed);
            eprintln!("Tearing down container...");
            let _ = monitor.set_phase(JobPhase::TearingDown);
            let _ = provider.teardown(&container).await;
            eprintln!("Container removed.");
            let _ = monitor.cleanup();
            return Err(anyhow::anyhow!(
                "assay config write exited with code {} in container {container}: stderr={}",
                handle.exit_code,
                handle.stderr.trim()
            ));
        }
        Ok(_) => {}
    }

    // Ensure the specs directory exists inside the container
    eprintln!("Writing specs dir...");
    let specs_dir_cmd = smelt_core::AssayInvoker::build_ensure_specs_dir_command();
    match provider.exec(&container, &specs_dir_cmd).await {
        Err(e) => {
            let _ = monitor.set_phase(JobPhase::Failed);
            eprintln!("Tearing down container...");
            let _ = monitor.set_phase(JobPhase::TearingDown);
            let _ = provider.teardown(&container).await;
            eprintln!("Container removed.");
            let _ = monitor.cleanup();
            return Err(e).with_context(|| "failed to exec specs dir creation");
        }
        Ok(handle) if handle.exit_code != 0 => {
            let _ = monitor.set_phase(JobPhase::Failed);
            eprintln!("Tearing down container...");
            let _ = monitor.set_phase(JobPhase::TearingDown);
            let _ = provider.teardown(&container).await;
            eprintln!("Container removed.");
            let _ = monitor.cleanup();
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
        eprintln!("Writing spec: {spec_name}...");
        if let Err(e) = smelt_core::AssayInvoker::write_spec_file_to_container(
            &provider,
            &container,
            &spec_name,
            &spec_toml,
        )
        .await
        {
            let _ = monitor.set_phase(JobPhase::Failed);
            eprintln!("Tearing down container...");
            let _ = monitor.set_phase(JobPhase::TearingDown);
            let _ = provider.teardown(&container).await;
            eprintln!("Container removed.");
            let _ = monitor.cleanup();
            return Err(e)
                .with_context(|| format!("failed to write spec '{spec_name}' to container"));
        }
    }

    // Phase 6: Write assay manifest into container
    monitor
        .set_phase(JobPhase::WritingManifest)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    eprintln!("Writing manifest...");
    let toml_content = smelt_core::AssayInvoker::build_run_manifest_toml(&manifest);
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
            .exec_streaming(&container, &cmd, |chunk| eprint!("{chunk}"))
            .await
            .with_context(|| "failed to execute assay run")?;
        let assay_exit = handle.exit_code;
        if assay_exit == 2 {
            eprintln!("Assay complete — gate failures (exit 2)");
        } else {
            eprintln!("Assay complete — exit code: {assay_exit}");
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
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        eprintln!("Collecting results...");
        let repo_path = manifest::resolve_repo_path(&manifest.job.repo)
            .with_context(|| "failed to resolve repo path for collection")?;
        let git_binary = which::which("git")
            .with_context(|| "git not found on PATH during collection")?;
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
            eprintln!("No new commits from Assay — target branch not created");
        } else {
            eprintln!(
                "Collected: {} commits on branch '{}', {} files changed",
                collect_result.commit_count,
                collect_result.branch,
                collect_result.files_changed.len(),
            );
        }

        // Phase 9: Create GitHub PR if forge is configured
        if should_create_pr(args.no_pr, collect_result.no_changes, manifest.forge.as_ref()) {
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
            let body = format!(
                "Automated results from smelt job '{job_name}'.\n\nBase: `{base}`"
            );
            eprintln!("Creating PR: {} → {}...", head, base);
            let pr = github
                .create_pr(&forge_cfg.repo, head, base, &title, &body)
                .await
                .with_context(|| "Phase 9: failed to create GitHub PR")?;
            monitor.state.pr_url = Some(pr.url.clone());
            monitor.state.pr_number = Some(pr.number);
            monitor.state.forge_repo = Some(forge_cfg.repo.clone());
            monitor.state.forge_token_env = Some(forge_cfg.token_env.clone());
            monitor.write().map_err(|e| anyhow::anyhow!("{e}"))?;
            eprintln!("PR created: {}", pr.url);
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

/// Ensure `.assay/` appears in the repo's `.gitignore`.
///
/// - If `.gitignore` does not exist: creates it with `.assay/\n`.
/// - If `.gitignore` exists and already contains `.assay/`: no-op (idempotent).
/// - If `.gitignore` exists but lacks `.assay/`: appends, preserving a trailing
///   newline boundary so the new entry always starts on its own line.
fn ensure_gitignore_assay(repo_path: &std::path::Path) -> anyhow::Result<()> {
    let gitignore_path = repo_path.join(".gitignore");

    if gitignore_path.exists() {
        let content = std::fs::read_to_string(&gitignore_path)?;
        // Idempotency check: already present — nothing to do
        if content.contains(".assay/") {
            return Ok(());
        }
        // Append, ensuring the entry begins on a new line
        let append = if content.ends_with('\n') {
            ".assay/\n".to_string()
        } else {
            "\n.assay/\n".to_string()
        };
        use std::io::Write as _;
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&gitignore_path)?;
        file.write_all(append.as_bytes())?;
    } else {
        std::fs::write(&gitignore_path, ".assay/\n")?;
    }

    Ok(())
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
        println!("  Context:     {}", kube.context.as_deref().unwrap_or("ambient"));
        if let Some(ref v) = kube.cpu_request    { println!("  CPU req:     {v}"); }
        if let Some(ref v) = kube.memory_request  { println!("  Mem req:     {v}"); }
        if let Some(ref v) = kube.cpu_limit       { println!("  CPU limit:   {v}"); }
        if let Some(ref v) = kube.memory_limit    { println!("  Mem limit:   {v}"); }
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
    use tempfile::TempDir;

    // ── ensure_gitignore_assay tests ────────────────────────────────────────

    #[test]
    fn test_ensure_gitignore_creates() {
        let tmp = TempDir::new().unwrap();
        // No .gitignore exists — should create it
        ensure_gitignore_assay(tmp.path()).unwrap();
        let content = std::fs::read_to_string(tmp.path().join(".gitignore")).unwrap();
        assert!(content.contains(".assay/"), "created .gitignore should contain .assay/");
    }

    #[test]
    fn test_ensure_gitignore_appends() {
        let tmp = TempDir::new().unwrap();
        // Existing .gitignore with trailing newline
        std::fs::write(tmp.path().join(".gitignore"), "target/\n").unwrap();
        ensure_gitignore_assay(tmp.path()).unwrap();
        let content = std::fs::read_to_string(tmp.path().join(".gitignore")).unwrap();
        assert!(content.contains("target/"), "original entry preserved");
        assert!(content.contains(".assay/"), "new entry added");
    }

    #[test]
    fn test_ensure_gitignore_trailing_newline() {
        let tmp = TempDir::new().unwrap();
        // Existing .gitignore WITHOUT trailing newline
        std::fs::write(tmp.path().join(".gitignore"), "target/").unwrap();
        ensure_gitignore_assay(tmp.path()).unwrap();
        let content = std::fs::read_to_string(tmp.path().join(".gitignore")).unwrap();
        // Must NOT produce "target/.assay/" on the same line
        assert!(
            !content.contains("target/.assay/"),
            "entries must be on separate lines, got: {content:?}"
        );
        assert!(content.contains(".assay/"), ".assay/ must appear in file");
    }

    #[test]
    fn test_ensure_gitignore_idempotent() {
        let tmp = TempDir::new().unwrap();
        // Already contains .assay/
        std::fs::write(tmp.path().join(".gitignore"), ".assay/\n").unwrap();
        // Call twice
        ensure_gitignore_assay(tmp.path()).unwrap();
        ensure_gitignore_assay(tmp.path()).unwrap();
        let content = std::fs::read_to_string(tmp.path().join(".gitignore")).unwrap();
        let count = content.matches(".assay/").count();
        assert_eq!(count, 1, ".assay/ should appear exactly once, got {count}");
    }

    // ── existing tests ──────────────────────────────────────────────────────

    fn forge_cfg() -> ForgeConfig {
        ForgeConfig {
            provider: "github".to_string(),
            repo: "owner/repo".to_string(),
            token_env: "GITHUB_TOKEN".to_string(),
        }
    }

    #[test]
    fn test_should_create_pr_guard() {
        let cfg = forge_cfg();

        // forge=None → always false regardless of other flags
        assert!(!should_create_pr(false, false, None), "forge=None should be false");
        assert!(!should_create_pr(false, true, None), "forge=None+no_changes should be false");
        assert!(!should_create_pr(true, false, None), "forge=None+no_pr should be false");
        assert!(!should_create_pr(true, true, None), "forge=None+both flags should be false");

        // no_pr=true → always false
        assert!(!should_create_pr(true, false, Some(&cfg)), "no_pr=true should be false");
        assert!(!should_create_pr(true, true, Some(&cfg)), "no_pr=true+no_changes should be false");

        // no_changes=true → false
        assert!(!should_create_pr(false, true, Some(&cfg)), "no_changes=true should be false");

        // all three conditions clear → true
        assert!(should_create_pr(false, false, Some(&cfg)), "all clear should be true");
    }

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
