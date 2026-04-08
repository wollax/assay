//! CLI subcommands for agent harness configuration management.
//!
//! Provides `assay harness generate|install|update|diff` for dispatching to
//! adapter-specific config generators (claude-code, codex, opencode).

use anyhow::{Context, bail};
use clap::Subcommand;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use assay_types::{HarnessProfile, PromptLayer, PromptLayerKind, SettingsOverride};

use super::project_root;

/// Valid adapter names for error messages.
const VALID_ADAPTERS: &[&str] = &["claude-code", "codex", "opencode"];

#[derive(Subcommand)]
pub(crate) enum HarnessCommand {
    /// Generate harness configuration for an adapter and print to stdout
    #[command(after_long_help = "\
Examples:
  Generate Claude Code config:
    assay harness generate claude-code

  Generate from a spec:
    assay harness generate claude-code --spec auth-flow

  Write output to a directory:
    assay harness generate codex --output-dir /tmp/config")]
    Generate {
        /// Adapter name: claude-code, codex, or opencode
        adapter: String,
        /// Spec name to load (filename without .toml extension)
        #[arg(long)]
        spec: Option<String>,
        /// Workflow name for session context
        #[arg(long)]
        workflow: Option<String>,
        /// Write generated config to this directory instead of stdout
        #[arg(long)]
        output_dir: Option<String>,
    },
    /// Generate and install harness config into the project root
    #[command(after_long_help = "\
Examples:
  Install Claude Code config:
    assay harness install claude-code

  Install from a spec:
    assay harness install codex --spec auth-flow")]
    Install {
        /// Adapter name: claude-code, codex, or opencode
        adapter: String,
        /// Spec name to load (filename without .toml extension)
        #[arg(long)]
        spec: Option<String>,
    },
    /// Regenerate and overwrite harness config in the project root
    #[command(after_long_help = "\
Examples:
  Update Claude Code config:
    assay harness update claude-code")]
    Update {
        /// Adapter name: claude-code, codex, or opencode
        adapter: String,
        /// Spec name to load (filename without .toml extension)
        #[arg(long)]
        spec: Option<String>,
    },
    /// Compare generated config against existing files (dry-run)
    #[command(after_long_help = "\
Examples:
  Diff Claude Code config:
    assay harness diff claude-code

  Diff with a spec:
    assay harness diff opencode --spec auth-flow")]
    Diff {
        /// Adapter name: claude-code, codex, or opencode
        adapter: String,
        /// Spec name to load (filename without .toml extension)
        #[arg(long)]
        spec: Option<String>,
    },
}

/// Handle harness subcommands.
pub(crate) fn handle(command: HarnessCommand) -> anyhow::Result<i32> {
    match command {
        HarnessCommand::Generate {
            adapter,
            spec,
            workflow: _,
            output_dir,
        } => handle_generate(&adapter, spec.as_deref(), output_dir.as_deref()),
        HarnessCommand::Install { adapter, spec } => handle_install(&adapter, spec.as_deref()),
        HarnessCommand::Update { adapter, spec } => handle_install(&adapter, spec.as_deref()),
        HarnessCommand::Diff { adapter, spec } => handle_diff(&adapter, spec.as_deref()),
    }
}

/// Validate an adapter name.
fn validate_adapter(adapter: &str) -> anyhow::Result<()> {
    if VALID_ADAPTERS.contains(&adapter) {
        Ok(())
    } else {
        bail!(
            "Unknown adapter '{}'. Valid adapters: {}",
            adapter,
            VALID_ADAPTERS.join(", ")
        );
    }
}

/// Build a minimal default HarnessProfile.
fn default_profile(adapter: &str) -> HarnessProfile {
    HarnessProfile {
        name: format!("{adapter}-default"),
        prompt_layers: Vec::new(),
        settings: SettingsOverride {
            model: None,
            permissions: Vec::new(),
            tools: Vec::new(),
            max_turns: None,
        },
        hooks: Vec::new(),
        working_dir: None,
    }
}

/// Try to load a spec and build a harness profile from it. Falls back to default profile.
fn resolve_profile(adapter: &str, spec_name: Option<&str>) -> anyhow::Result<HarnessProfile> {
    let Some(spec_name) = spec_name else {
        return Ok(default_profile(adapter));
    };

    let root = project_root()?;
    let config = assay_core::config::load(&root).map_err(|e| anyhow::anyhow!("{e}"))?;
    let specs_dir = root.join(".assay").join(&config.specs_dir);
    let spec_path = specs_dir.join(format!("{spec_name}.toml"));

    if !spec_path.exists() {
        bail!("Spec '{}' not found at {}", spec_name, spec_path.display());
    }

    let spec_content = std::fs::read_to_string(&spec_path).context("failed to read spec file")?;
    let _spec: assay_types::Spec =
        toml::from_str(&spec_content).context("failed to parse spec TOML")?;

    // Build a ManifestSession from the spec for profile construction.
    let session = assay_types::ManifestSession {
        spec: spec_name.to_string(),
        name: Some(format!("{adapter}-{spec_name}")),
        settings: None,
        hooks: Vec::new(),
        prompt_layers: Vec::new(),
        file_scope: Vec::new(),
        shared_files: Vec::new(),
        depends_on: Vec::new(),
        prompt: None,
    };

    Ok(assay_core::pipeline::build_harness_profile(&session))
}

/// Inject scope prompt layer if file_scope is non-empty.
fn inject_scope_layer(
    profile: &mut HarnessProfile,
    file_scope: &[String],
    shared_files: &[String],
) {
    if file_scope.is_empty() {
        return;
    }
    let prompt = assay_harness::scope::generate_scope_prompt(
        &profile.name,
        file_scope,
        shared_files,
        &[], // No other sessions in CLI context
    );
    profile.prompt_layers.push(PromptLayer {
        kind: PromptLayerKind::System,
        name: "scope-enforcement".to_string(),
        content: prompt,
        priority: -100,
    });
}

/// Enumeration of generated config files by adapter.
enum GeneratedConfig {
    Claude(assay_harness::claude::ClaudeConfig),
    Codex(assay_harness::codex::CodexConfig),
    OpenCode(assay_harness::opencode::OpenCodeConfig),
}

impl GeneratedConfig {
    /// Return (relative_path, content) pairs for all files this config would produce.
    fn files(&self) -> Vec<(String, String)> {
        match self {
            GeneratedConfig::Claude(c) => {
                let mut files = Vec::new();
                if !c.claude_md.is_empty() {
                    files.push(("CLAUDE.md".to_string(), c.claude_md.clone()));
                }
                files.push((".mcp.json".to_string(), c.mcp_json.clone()));
                files.push((".claude/settings.json".to_string(), c.settings_json.clone()));
                files
            }
            GeneratedConfig::Codex(c) => {
                let mut files = Vec::new();
                if !c.agents_md.is_empty() {
                    files.push(("AGENTS.md".to_string(), c.agents_md.clone()));
                }
                files.push((".codex/config.toml".to_string(), c.config_toml.clone()));
                files
            }
            GeneratedConfig::OpenCode(c) => {
                let mut files = Vec::new();
                if !c.agents_md.is_empty() {
                    files.push(("AGENTS.md".to_string(), c.agents_md.clone()));
                }
                files.push(("opencode.json".to_string(), c.config_json.clone()));
                files
            }
        }
    }

    /// Write config to a directory using the adapter's write_config.
    fn write(&self, dir: &Path) -> anyhow::Result<()> {
        match self {
            GeneratedConfig::Claude(c) => {
                assay_harness::claude::write_config(c, dir).context("failed to write Claude config")
            }
            GeneratedConfig::Codex(c) => {
                assay_harness::codex::write_config(c, dir).context("failed to write Codex config")
            }
            GeneratedConfig::OpenCode(c) => assay_harness::opencode::write_config(c, dir)
                .context("failed to write OpenCode config"),
        }
    }
}

/// Generate config for the given adapter.
fn generate_for_adapter(
    adapter: &str,
    profile: &HarnessProfile,
) -> anyhow::Result<GeneratedConfig> {
    match adapter {
        "claude-code" => Ok(GeneratedConfig::Claude(
            assay_harness::claude::generate_config(profile),
        )),
        "codex" => Ok(GeneratedConfig::Codex(
            assay_harness::codex::generate_config(profile),
        )),
        "opencode" => Ok(GeneratedConfig::OpenCode(
            assay_harness::opencode::generate_config(profile),
        )),
        _ => unreachable!("adapter already validated"),
    }
}

/// Handle `assay harness generate`.
fn handle_generate(
    adapter: &str,
    spec: Option<&str>,
    output_dir: Option<&str>,
) -> anyhow::Result<i32> {
    validate_adapter(adapter)?;
    let mut profile = resolve_profile(adapter, spec)?;
    inject_scope_layer(&mut profile, &[], &[]);

    let config = generate_for_adapter(adapter, &profile)?;
    let files = config.files();

    // Print config summary.
    tracing::info!(file_count = files.len(), adapter = %adapter, "Generated config");
    for (path, content) in &files {
        tracing::info!(path = %path, bytes = content.len(), "Generated file");
    }

    // Print file contents to stdout for piping.
    for (path, content) in &files {
        println!("--- {path} ---");
        println!("{content}");
    }

    // If --output-dir provided, write files.
    if let Some(dir) = output_dir {
        let dir_path = PathBuf::from(dir);
        std::fs::create_dir_all(&dir_path)
            .with_context(|| format!("failed to create output dir: {}", dir_path.display()))?;
        config.write(&dir_path)?;
        tracing::info!(output_dir = %dir_path.display(), "Wrote config");
    }

    Ok(0)
}

/// Handle `assay harness install` and `assay harness update` (same behavior).
fn handle_install(adapter: &str, spec: Option<&str>) -> anyhow::Result<i32> {
    validate_adapter(adapter)?;
    let mut profile = resolve_profile(adapter, spec)?;
    inject_scope_layer(&mut profile, &[], &[]);

    let config = generate_for_adapter(adapter, &profile)?;
    let root = project_root()?;

    config.write(&root)?;

    let files = config.files();
    tracing::info!(file_count = files.len(), adapter = %adapter, root = %root.display(), "Installed config");
    for (path, _) in &files {
        tracing::info!(path = %path, "Installed file");
    }

    Ok(0)
}

/// Handle `assay harness diff`.
fn handle_diff(adapter: &str, spec: Option<&str>) -> anyhow::Result<i32> {
    validate_adapter(adapter)?;
    let mut profile = resolve_profile(adapter, spec)?;
    inject_scope_layer(&mut profile, &[], &[]);

    let config = generate_for_adapter(adapter, &profile)?;
    let root = project_root()?;
    let files = config.files();

    let mut added = Vec::new();
    let mut changed = Vec::new();
    let mut unchanged = Vec::new();

    // Collect all generated file paths.
    let generated_paths: BTreeSet<String> = files.iter().map(|(p, _)| p.clone()).collect();

    for (rel_path, new_content) in &files {
        let existing_path = root.join(rel_path);
        if existing_path.exists() {
            let existing = std::fs::read_to_string(&existing_path)
                .with_context(|| format!("failed to read {}", existing_path.display()))?;
            if existing == *new_content {
                unchanged.push(rel_path.clone());
            } else {
                changed.push(rel_path.clone());
            }
        } else {
            added.push(rel_path.clone());
        }
    }

    // Check for files that would be removed (existing adapter files not in generated set).
    let removed = find_existing_adapter_files(adapter, &root)
        .into_iter()
        .filter(|p| !generated_paths.contains(p))
        .collect::<Vec<_>>();

    let has_changes = !added.is_empty() || !changed.is_empty() || !removed.is_empty();

    if !has_changes {
        tracing::info!(adapter = %adapter, "No changes detected");
        return Ok(0);
    }

    tracing::info!(adapter = %adapter, "Diff results");
    for path in &added {
        tracing::info!(path = %path, change = "added", "Diff entry");
    }
    for path in &changed {
        tracing::info!(path = %path, change = "changed", "Diff entry");
    }
    for path in &removed {
        tracing::info!(path = %path, change = "removed", "Diff entry");
    }
    tracing::info!(
        added = added.len(),
        changed = changed.len(),
        removed = removed.len(),
        unchanged = unchanged.len(),
        "Diff summary"
    );

    Ok(1) // Exit code 1 indicates changes detected.
}

/// Find existing adapter-specific config files in the project root.
fn find_existing_adapter_files(adapter: &str, root: &Path) -> Vec<String> {
    let candidates: Vec<&str> = match adapter {
        "claude-code" => vec!["CLAUDE.md", ".mcp.json", ".claude/settings.json"],
        "codex" => vec!["AGENTS.md", ".codex/config.toml"],
        "opencode" => vec!["AGENTS.md", "opencode.json"],
        _ => vec![],
    };

    candidates
        .into_iter()
        .filter(|p| root.join(p).exists())
        .map(|p| p.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_adapter_accepts_valid_names() {
        assert!(validate_adapter("claude-code").is_ok());
        assert!(validate_adapter("codex").is_ok());
        assert!(validate_adapter("opencode").is_ok());
    }

    #[test]
    fn validate_adapter_rejects_unknown() {
        let err = validate_adapter("foo").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Unknown adapter 'foo'"));
        assert!(msg.contains("claude-code"));
        assert!(msg.contains("codex"));
        assert!(msg.contains("opencode"));
    }

    #[test]
    fn generate_claude_code_produces_non_empty_config() {
        let profile = default_profile("claude-code");
        let config = generate_for_adapter("claude-code", &profile).unwrap();
        let files = config.files();
        assert!(!files.is_empty(), "claude-code should produce files");
        // Should have at least .mcp.json and .claude/settings.json
        let paths: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
        assert!(paths.contains(&".mcp.json"));
        assert!(paths.contains(&".claude/settings.json"));
    }

    #[test]
    fn generate_codex_produces_non_empty_config() {
        let profile = default_profile("codex");
        let config = generate_for_adapter("codex", &profile).unwrap();
        let files = config.files();
        assert!(!files.is_empty(), "codex should produce files");
        let paths: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
        assert!(paths.contains(&".codex/config.toml"));
    }

    #[test]
    fn generate_opencode_produces_non_empty_config() {
        let profile = default_profile("opencode");
        let config = generate_for_adapter("opencode", &profile).unwrap();
        let files = config.files();
        assert!(!files.is_empty(), "opencode should produce files");
        let paths: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
        assert!(paths.contains(&"opencode.json"));
    }

    #[test]
    fn diff_with_no_existing_files_shows_all_added() {
        let profile = default_profile("claude-code");
        let config = generate_for_adapter("claude-code", &profile).unwrap();
        let files = config.files();

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // No existing files — all should be "added".
        let mut added = Vec::new();
        for (rel_path, _) in &files {
            let existing_path = root.join(rel_path);
            if !existing_path.exists() {
                added.push(rel_path.clone());
            }
        }
        assert_eq!(added.len(), files.len(), "all files should be added");
    }

    #[test]
    fn diff_detects_changed_files() {
        let profile = default_profile("claude-code");
        let config = generate_for_adapter("claude-code", &profile).unwrap();
        let files = config.files();

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Write files with different content.
        for (rel_path, _) in &files {
            let full_path = root.join(rel_path);
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(&full_path, "old content").unwrap();
        }

        // Now check — all should be "changed".
        let mut changed = Vec::new();
        for (rel_path, new_content) in &files {
            let existing = std::fs::read_to_string(root.join(rel_path)).unwrap();
            if existing != *new_content {
                changed.push(rel_path.clone());
            }
        }
        assert_eq!(changed.len(), files.len(), "all files should be changed");
    }

    #[test]
    fn scope_prompt_injected_when_file_scope_present() {
        let mut profile = default_profile("claude-code");
        let file_scope = vec!["src/**/*.rs".to_string()];
        let shared = vec!["shared/config.rs".to_string()];
        inject_scope_layer(&mut profile, &file_scope, &shared);

        assert_eq!(profile.prompt_layers.len(), 1);
        let layer = &profile.prompt_layers[0];
        assert_eq!(layer.kind, PromptLayerKind::System);
        assert_eq!(layer.name, "scope-enforcement");
        assert_eq!(layer.priority, -100);
        assert!(layer.content.contains("src/**/*.rs"));
        assert!(layer.content.contains("shared/config.rs"));
    }

    #[test]
    fn scope_prompt_not_injected_when_file_scope_empty() {
        let mut profile = default_profile("claude-code");
        inject_scope_layer(&mut profile, &[], &[]);
        assert!(profile.prompt_layers.is_empty());
    }

    #[test]
    fn install_writes_config_to_dir() {
        let profile = default_profile("codex");
        let config = generate_for_adapter("codex", &profile).unwrap();
        let dir = tempfile::tempdir().unwrap();
        config.write(dir.path()).unwrap();

        assert!(dir.path().join(".codex/config.toml").exists());
    }

    #[test]
    fn find_existing_adapter_files_returns_present_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Create some claude-code files.
        std::fs::write(root.join(".mcp.json"), "{}").unwrap();
        let claude_dir = root.join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(claude_dir.join("settings.json"), "{}").unwrap();

        let existing = find_existing_adapter_files("claude-code", root);
        assert!(existing.contains(&".mcp.json".to_string()));
        assert!(existing.contains(&".claude/settings.json".to_string()));
        // CLAUDE.md not present, so not included.
        assert!(!existing.contains(&"CLAUDE.md".to_string()));
    }
}
