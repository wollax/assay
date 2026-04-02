//! Core parsing, validation, credential resolution, and repo-path tests.

use super::*;

/// Build a minimal valid docker-runtime manifest TOML string.
/// `job`, `env`, `session`, and `merge` are overridable by passing a non-empty
/// replacement string; pass `""` to use the built-in default for that section.
/// The `[credentials]` section is always the built-in default and is not overridable.
fn minimal_toml(job: &str, env: &str, session: &str, merge: &str) -> String {
    let job = if job.is_empty() {
        "[job]\nname = \"test\"\nrepo = \"repo\"\nbase_ref = \"main\""
    } else {
        job
    };
    let env = if env.is_empty() {
        "[environment]\nruntime = \"docker\"\nimage = \"img\""
    } else {
        env
    };
    let creds = "[credentials]\nprovider = \"anthropic\"\nmodel = \"m\"";
    let session = if session.is_empty() {
        "[[session]]\nname = \"s1\"\nspec = \"s\"\nharness = \"h\"\ntimeout = 60"
    } else {
        session
    };
    let merge = if merge.is_empty() {
        "[merge]\nstrategy = \"sequential\"\ntarget = \"main\""
    } else {
        merge
    };
    format!("{job}\n\n{env}\n\n{creds}\n\n{session}\n\n{merge}")
}

#[test]
fn parse_valid_manifest() {
    let manifest = load_from_str(VALID_MANIFEST).expect("should parse");
    assert_eq!(manifest.job.name, "test-job");
    assert_eq!(manifest.job.repo, "https://github.com/example/repo");
    assert_eq!(manifest.job.base_ref, "main");
    assert_eq!(manifest.environment.runtime, "docker");
    assert_eq!(manifest.environment.image, "ubuntu:22.04");
    assert_eq!(manifest.environment.resources.get("cpu").unwrap(), "2");
    assert_eq!(manifest.credentials.provider, "anthropic");
    assert_eq!(manifest.credentials.model, "claude-sonnet-4-20250514");
    assert_eq!(manifest.session.len(), 2);
    assert_eq!(manifest.session[0].name, "frontend");
    assert_eq!(manifest.session[0].timeout, 300);
    assert_eq!(manifest.session[1].depends_on, vec!["frontend"]);
    assert_eq!(manifest.merge.strategy, "sequential");
    assert!(manifest.merge.ai_resolution);
    assert_eq!(manifest.merge.target, "main");
}

#[test]
fn validate_valid_manifest() {
    let manifest = load_from_str(VALID_MANIFEST).unwrap();
    manifest
        .validate()
        .expect("valid manifest should pass validation");
}

#[test]
fn reject_unknown_fields() {
    let toml = minimal_toml(
        "[job]\nname = \"test\"\nrepo = \"repo\"\nbase_ref = \"main\"\nbogus_field = \"oops\"",
        "",
        "",
        "",
    );
    let err = load_from_str(&toml).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("bogus_field") || msg.contains("unknown field"),
        "error should mention unknown field: {msg}"
    );
}

#[test]
fn reject_unknown_fields_in_session() {
    let toml = minimal_toml(
        "",
        "",
        "[[session]]\nname = \"s1\"\nspec = \"s\"\nharness = \"h\"\ntimeout = 60\nextra_thing = true",
        "",
    );
    let err = load_from_str(&toml).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("extra_thing") || msg.contains("unknown field"),
        "error should mention unknown field: {msg}"
    );
}

#[test]
fn validate_duplicate_session_names() {
    let toml = minimal_toml(
        "",
        "",
        "[[session]]\nname = \"dupe\"\nspec = \"s\"\nharness = \"h\"\ntimeout = 60\n\n\
         [[session]]\nname = \"dupe\"\nspec = \"s2\"\nharness = \"h2\"\ntimeout = 120",
        "",
    );
    let manifest = load_from_str(&toml).unwrap();
    let err = manifest.validate().unwrap_err();
    assert!(err.to_string().contains("duplicate session name"));
}

#[test]
fn validate_zero_timeout() {
    let toml = minimal_toml(
        "",
        "",
        "[[session]]\nname = \"s1\"\nspec = \"s\"\nharness = \"h\"\ntimeout = 0",
        "",
    );
    let manifest = load_from_str(&toml).unwrap();
    let err = manifest.validate().unwrap_err();
    assert!(err.to_string().contains("session[0].timeout: must be > 0"));
}

#[test]
fn validate_self_dependency() {
    let toml = minimal_toml(
        "",
        "",
        "[[session]]\nname = \"s1\"\nspec = \"s\"\nharness = \"h\"\ntimeout = 60\ndepends_on = [\"s1\"]",
        "",
    );
    let manifest = load_from_str(&toml).unwrap();
    let err = manifest.validate().unwrap_err();
    assert!(err.to_string().contains("cannot depend on itself"));
}

#[test]
fn validate_unknown_dependency() {
    let toml = minimal_toml(
        "",
        "",
        "[[session]]\nname = \"s1\"\nspec = \"s\"\nharness = \"h\"\ntimeout = 60\ndepends_on = [\"nonexistent\"]",
        "",
    );
    let manifest = load_from_str(&toml).unwrap();
    let err = manifest.validate().unwrap_err();
    assert!(err.to_string().contains("unknown session `nonexistent`"));
}

#[test]
fn validate_circular_dependency() {
    let toml = minimal_toml(
        "",
        "",
        "[[session]]\nname = \"a\"\nspec = \"s\"\nharness = \"h\"\ntimeout = 60\ndepends_on = [\"b\"]\n\n\
         [[session]]\nname = \"b\"\nspec = \"s\"\nharness = \"h\"\ntimeout = 60\ndepends_on = [\"a\"]",
        "",
    );
    let manifest = load_from_str(&toml).unwrap();
    let err = manifest.validate().unwrap_err();
    assert!(err.to_string().contains("cycle detected"));
}

#[test]
fn validate_empty_image() {
    let toml = minimal_toml(
        "",
        "[environment]\nruntime = \"docker\"\nimage = \"\"",
        "",
        "",
    );
    let manifest = load_from_str(&toml).unwrap();
    let err = manifest.validate().unwrap_err();
    assert!(
        err.to_string()
            .contains("environment.image: must not be empty")
    );
}

#[test]
fn validate_empty_merge_target() {
    let toml = minimal_toml(
        "",
        "",
        "",
        "[merge]\nstrategy = \"sequential\"\ntarget = \"\"",
    );
    let manifest = load_from_str(&toml).unwrap();
    let err = manifest.validate().unwrap_err();
    assert!(err.to_string().contains("merge.target: must not be empty"));
}

#[test]
fn validate_invalid_merge_order() {
    let toml = minimal_toml(
        "",
        "",
        "",
        "[merge]\nstrategy = \"sequential\"\norder = [\"s1\", \"nonexistent\"]\ntarget = \"main\"",
    );
    let manifest = load_from_str(&toml).unwrap();
    let err = manifest.validate().unwrap_err();
    assert!(
        err.to_string()
            .contains("merge.order: unknown session `nonexistent`")
    );
}

#[test]
fn validate_no_sessions() {
    // TOML without any [[session]] — parses fine (session defaults to empty vec)
    // but validation rejects it because at least one session is required.
    let toml = "[job]\nname = \"test\"\nrepo = \"repo\"\nbase_ref = \"main\"\n\n\
                [environment]\nruntime = \"docker\"\nimage = \"img\"\n\n\
                [credentials]\nprovider = \"anthropic\"\nmodel = \"m\"\n\n\
                [merge]\nstrategy = \"sequential\"\ntarget = \"main\"";
    let manifest = load_from_str(toml).unwrap();
    let err = manifest.validate().unwrap_err();
    assert!(err.to_string().contains("session"));
}

#[test]
fn credential_resolution_from_env() {
    let manifest = load_from_str(VALID_MANIFEST).unwrap();

    // Capture the prior value so we can restore it unconditionally on drop, even on
    // panic. This prevents the modified value from leaking into *subsequent* tests.
    // Note: concurrent tests reading ANTHROPIC_API_KEY can still observe the
    // modified value while this test is running; run with --test-threads=1 if
    // strict env isolation is required.
    let prior = std::env::var("ANTHROPIC_API_KEY").ok();
    struct EnvGuard(Option<String>);
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            // SAFETY: Restoring the prior value on drop; runs even on panic.
            match &self.0 {
                Some(v) => unsafe { std::env::set_var("ANTHROPIC_API_KEY", v) },
                None => unsafe { std::env::remove_var("ANTHROPIC_API_KEY") },
            }
        }
    }
    let _guard = EnvGuard(prior);

    // SAFETY: Protected by EnvGuard above — env is restored even on panic.
    unsafe { std::env::set_var("ANTHROPIC_API_KEY", "test-value") };
    let creds = manifest.resolve_credentials();
    let status = creds.get("api_key").expect("should have api_key entry");
    assert!(matches!(status, CredentialStatus::Resolved { .. }));
    assert!(status.to_string().contains("resolved"));

    // SAFETY: Same guard — restored on drop.
    unsafe { std::env::remove_var("ANTHROPIC_API_KEY") };
    let creds = manifest.resolve_credentials();
    let status = creds.get("api_key").unwrap();
    assert!(matches!(status, CredentialStatus::Missing { .. }));
    assert!(status.to_string().contains("MISSING"));
}

#[test]
fn load_from_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("manifest.toml");
    std::fs::write(&path, VALID_MANIFEST).unwrap();
    let manifest = JobManifest::load(&path).expect("should load from file");
    assert_eq!(manifest.job.name, "test-job");
    manifest.validate().expect("should validate");
}

#[test]
fn load_nonexistent_file() {
    let err = JobManifest::load(Path::new("/tmp/nonexistent-manifest-12345.toml")).unwrap_err();
    assert!(err.to_string().contains("cannot read"));
}

#[test]
fn validate_multiple_errors_reported() {
    let toml = minimal_toml(
        "[job]\nname = \"\"\nrepo = \"\"\nbase_ref = \"main\"",
        "[environment]\nruntime = \"docker\"\nimage = \"\"",
        "[[session]]\nname = \"s1\"\nspec = \"s\"\nharness = \"h\"\ntimeout = 0",
        "[merge]\nstrategy = \"sequential\"\ntarget = \"\"",
    );
    let manifest = load_from_str(&toml).unwrap();
    let err = manifest.validate().unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("job.name"), "should report empty name: {msg}");
    assert!(msg.contains("job.repo"), "should report empty repo: {msg}");
    assert!(
        msg.contains("environment.image"),
        "should report empty image: {msg}"
    );
    assert!(msg.contains("timeout"), "should report zero timeout: {msg}");
    assert!(
        msg.contains("merge.target"),
        "should report empty target: {msg}"
    );
}

// ── resolve_repo_path tests ─────────────────────────────────

#[test]
fn resolve_repo_path_valid_absolute() {
    let dir = tempfile::tempdir().unwrap();
    let result = resolve_repo_path(dir.path().to_str().unwrap()).unwrap();
    assert!(result.is_absolute());
    assert_eq!(result, std::fs::canonicalize(dir.path()).unwrap());
}

#[test]
fn resolve_repo_path_rejects_http() {
    let err = resolve_repo_path("http://github.com/example/repo").unwrap_err();
    assert!(err.to_string().contains("not a URL"));
}

#[test]
fn resolve_repo_path_rejects_https() {
    let err = resolve_repo_path("https://github.com/example/repo").unwrap_err();
    assert!(err.to_string().contains("not a URL"));
}

#[test]
fn resolve_repo_path_rejects_git() {
    let err = resolve_repo_path("git://github.com/example/repo").unwrap_err();
    assert!(err.to_string().contains("not a URL"));
}

#[test]
fn resolve_repo_path_rejects_ssh() {
    let err = resolve_repo_path("ssh://git@github.com/example/repo").unwrap_err();
    assert!(err.to_string().contains("not a URL"));
}

#[test]
fn resolve_repo_path_rejects_scp_style() {
    let err = resolve_repo_path("git@github.com:example/repo").unwrap_err();
    assert!(err.to_string().contains("not a URL"));
}

#[test]
fn resolve_repo_path_relative_path() {
    let result = resolve_repo_path(".").unwrap();
    assert!(result.is_absolute());
    assert_eq!(result, std::fs::canonicalize(".").unwrap());
}

#[test]
fn resolve_repo_path_nonexistent() {
    let err = resolve_repo_path("/tmp/smelt-nonexistent-path-12345xyz").unwrap_err();
    assert!(err.to_string().contains("cannot resolve repo path"));
}

#[test]
fn resolve_repo_path_with_spaces() {
    let dir = tempfile::tempdir().unwrap();
    let spaced = dir.path().join("path with spaces");
    std::fs::create_dir_all(&spaced).unwrap();
    let result = resolve_repo_path(spaced.to_str().unwrap()).unwrap();
    assert!(result.is_absolute());
    assert_eq!(result, std::fs::canonicalize(&spaced).unwrap());
}

// --- job.name path-traversal validation tests (D145) ---

#[test]
fn validate_job_name_forward_slash_rejected() {
    let toml = minimal_toml(
        "[job]\nname = \"path/traversal\"\nrepo = \"repo\"\nbase_ref = \"main\"",
        "",
        "",
        "",
    );
    let manifest = load_from_str(&toml).unwrap();
    let err = manifest.validate().unwrap_err();
    assert!(
        err.to_string().contains("path separator"),
        "should reject slash in name: {err}"
    );
}

#[test]
fn validate_job_name_backslash_rejected() {
    let toml = minimal_toml(
        "[job]\nname = \"evil\\\\name\"\nrepo = \"repo\"\nbase_ref = \"main\"",
        "",
        "",
        "",
    );
    let manifest = load_from_str(&toml).unwrap();
    let err = manifest.validate().unwrap_err();
    assert!(
        err.to_string().contains("path separator"),
        "should reject backslash in name: {err}"
    );
}

#[test]
fn validate_job_name_dotdot_rejected() {
    let toml = minimal_toml(
        "[job]\nname = \"..\"\nrepo = \"repo\"\nbase_ref = \"main\"",
        "",
        "",
        "",
    );
    let manifest = load_from_str(&toml).unwrap();
    let err = manifest.validate().unwrap_err();
    assert!(
        err.to_string().contains("'.' or '..'"),
        "should reject '..' name: {err}"
    );
}

#[test]
fn validate_job_name_dot_rejected() {
    let toml = minimal_toml(
        "[job]\nname = \".\"\nrepo = \"repo\"\nbase_ref = \"main\"",
        "",
        "",
        "",
    );
    let manifest = load_from_str(&toml).unwrap();
    let err = manifest.validate().unwrap_err();
    assert!(
        err.to_string().contains("'.' or '..'"),
        "should reject '.' name: {err}"
    );
}

#[test]
fn validate_job_name_plain_valid() {
    // Regression guard: a normal name must pass the new path-safety check.
    let toml = minimal_toml(
        "[job]\nname = \"my-job-2026\"\nrepo = \"repo\"\nbase_ref = \"main\"",
        "",
        "",
        "",
    );
    let manifest = load_from_str(&toml).unwrap();
    manifest
        .validate()
        .expect("plain job name should pass validation");
}

// ── state_backend integration ──────────────────────────────

#[test]
fn manifest_state_backend_absent_defaults_to_none() {
    let toml = minimal_toml("", "", "", "");
    let manifest = load_from_str(&toml).unwrap();
    assert!(manifest.state_backend.is_none());
}

#[test]
fn manifest_state_backend_local_fs() {
    // state_backend as inline value must appear before any [table] section
    // or use inline syntax within the TOML structure.
    let toml = r#"
state_backend = "local_fs"

[job]
name = "test"
repo = "repo"
base_ref = "main"

[environment]
runtime = "docker"
image = "img"

[credentials]
provider = "anthropic"
model = "m"

[[session]]
name = "s1"
spec = "s"
harness = "h"
timeout = 60

[merge]
strategy = "sequential"
target = "main"
"#;
    let manifest = load_from_str(toml).unwrap();
    assert_eq!(
        manifest.state_backend,
        Some(crate::tracker::StateBackendConfig::LocalFs)
    );
}

#[test]
fn manifest_state_backend_linear() {
    let toml = r#"
[job]
name = "test"
repo = "repo"
base_ref = "main"

[environment]
runtime = "docker"
image = "img"

[credentials]
provider = "anthropic"
model = "m"

[[session]]
name = "s1"
spec = "s"
harness = "h"
timeout = 60

[merge]
strategy = "sequential"
target = "main"

[state_backend]
linear = { team_id = "TEAM" }
"#;
    let manifest = load_from_str(toml).unwrap();
    assert_eq!(
        manifest.state_backend,
        Some(crate::tracker::StateBackendConfig::Linear {
            team_id: "TEAM".into(),
            project_id: None,
        })
    );
}

#[test]
fn manifest_state_backend_github() {
    let toml = r#"
[job]
name = "test"
repo = "repo"
base_ref = "main"

[environment]
runtime = "docker"
image = "img"

[credentials]
provider = "anthropic"
model = "m"

[[session]]
name = "s1"
spec = "s"
harness = "h"
timeout = 60

[merge]
strategy = "sequential"
target = "main"

[state_backend]
github = { repo = "owner/repo" }
"#;
    let manifest = load_from_str(toml).unwrap();
    assert_eq!(
        manifest.state_backend,
        Some(crate::tracker::StateBackendConfig::GitHub {
            repo: "owner/repo".into(),
            label: None,
        })
    );
}

#[test]
fn manifest_state_backend_smelt() {
    let toml = r#"
[job]
name = "test"
repo = "repo"
base_ref = "main"

[environment]
runtime = "docker"
image = "img"

[credentials]
provider = "anthropic"
model = "m"

[[session]]
name = "s1"
spec = "s"
harness = "h"
timeout = 60

[merge]
strategy = "sequential"
target = "main"

[state_backend]
smelt = { url = "http://host.docker.internal:8765/api/v1/events", job_id = "job-1", token = "SMELT_WRITE_TOKEN" }
"#;
    let manifest = load_from_str(toml).unwrap();
    assert_eq!(
        manifest.state_backend,
        Some(crate::tracker::StateBackendConfig::Smelt {
            url: "http://host.docker.internal:8765/api/v1/events".into(),
            job_id: "job-1".into(),
            token: Some("SMELT_WRITE_TOKEN".into()),
        })
    );
}

#[test]
fn manifest_state_backend_ssh() {
    let toml = r#"
[job]
name = "test"
repo = "repo"
base_ref = "main"

[environment]
runtime = "docker"
image = "img"

[credentials]
provider = "anthropic"
model = "m"

[[session]]
name = "s1"
spec = "s"
harness = "h"
timeout = 60

[merge]
strategy = "sequential"
target = "main"

[state_backend]
ssh = { host = "ci.example.com", remote_assay_dir = "/opt/assay", user = "deploy", port = 2222 }
"#;
    let manifest = load_from_str(toml).unwrap();
    assert_eq!(
        manifest.state_backend,
        Some(crate::tracker::StateBackendConfig::Ssh {
            host: "ci.example.com".into(),
            remote_assay_dir: "/opt/assay".into(),
            user: Some("deploy".into()),
            port: Some(2222),
        })
    );
}

#[test]
fn manifest_state_backend_custom_json_value_from_toml() {
    // Verifies that serde_json::Value (used in Custom.config) deserializes
    // correctly from a TOML document — both implement serde, so cross-format
    // deserialization works via serde's type system.
    let toml = r#"
[job]
name = "test"
repo = "repo"
base_ref = "main"

[environment]
runtime = "docker"
image = "img"

[credentials]
provider = "anthropic"
model = "m"

[[session]]
name = "s1"
spec = "s"
harness = "h"
timeout = 60

[merge]
strategy = "sequential"
target = "main"

[state_backend.custom]
name = "redis"
[state_backend.custom.config]
url = "redis://localhost:6379"
db = 0
"#;
    let manifest = load_from_str(toml).unwrap();
    match manifest.state_backend {
        Some(crate::tracker::StateBackendConfig::Custom { name, config }) => {
            assert_eq!(name, "redis");
            assert_eq!(
                config["url"],
                serde_json::Value::String("redis://localhost:6379".into())
            );
            assert_eq!(config["db"], serde_json::Value::Number(0.into()));
        }
        other => panic!("expected Custom variant, got {other:?}"),
    }
}

// ── [[notify]] rules ──────────────────────────────────────────
#[test]
fn manifest_notify_absent_defaults_to_empty() {
    let manifest = load_from_str(&minimal_toml("", "", "", "")).unwrap();
    assert!(manifest.notify.is_empty());
}

#[test]
fn manifest_notify_single_rule() {
    let toml = r#"
[job]
name = "backend"
repo = "."
base_ref = "main"

[environment]
runtime = "docker"
image = "img"

[credentials]
provider = "anthropic"
model = "m"

[[session]]
name = "s1"
spec = "s"
harness = "h"
timeout = 60

[merge]
strategy = "sequential"
target = "main"

[[notify]]
target_job = "frontend"
on_session_complete = true
"#;
    let manifest = load_from_str(toml).unwrap();
    assert_eq!(manifest.notify.len(), 1);
    assert_eq!(manifest.notify[0].target_job, "frontend");
    assert!(manifest.notify[0].on_session_complete);
}

#[test]
fn manifest_notify_multiple_rules() {
    let toml = r#"
[job]
name = "backend"
repo = "."
base_ref = "main"

[environment]
runtime = "docker"
image = "img"

[credentials]
provider = "anthropic"
model = "m"

[[session]]
name = "s1"
spec = "s"
harness = "h"
timeout = 60

[merge]
strategy = "sequential"
target = "main"

[[notify]]
target_job = "frontend"
on_session_complete = true

[[notify]]
target_job = "docs"
on_session_complete = false
"#;
    let manifest = load_from_str(toml).unwrap();
    assert_eq!(manifest.notify.len(), 2);
    assert_eq!(manifest.notify[0].target_job, "frontend");
    assert!(manifest.notify[0].on_session_complete);
    assert_eq!(manifest.notify[1].target_job, "docs");
    assert!(!manifest.notify[1].on_session_complete);
}
