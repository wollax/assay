//! Forge (PR creation) configuration tests.

use super::*;

const MANIFEST_WITH_FORGE: &str = r#"
[job]
name = "test-job"
repo = "https://github.com/example/repo"
base_ref = "main"

[environment]
runtime = "docker"
image = "ubuntu:22.04"

[credentials]
provider = "anthropic"
model = "claude-sonnet-4-20250514"

[[session]]
name = "frontend"
spec = "Implement the login page"
harness = "npm test"
timeout = 300

[merge]
strategy = "sequential"
target = "main"

[forge]
provider = "github"
repo = "owner/my-repo"
token_env = "GITHUB_TOKEN"
"#;

#[test]
fn test_parse_manifest_with_forge() {
    let manifest = load_from_str(MANIFEST_WITH_FORGE).expect("should parse");
    let forge = manifest.forge.as_ref().expect("forge should be Some");
    assert_eq!(forge.provider, "github");
    assert_eq!(forge.repo, "owner/my-repo");
    assert_eq!(forge.token_env, "GITHUB_TOKEN");
}

#[test]
fn test_parse_manifest_without_forge() {
    let manifest = load_from_str(VALID_MANIFEST).expect("should parse");
    assert!(
        manifest.forge.is_none(),
        "forge should be None when no [forge] section"
    );
}

#[test]
fn test_validate_forge_invalid_repo_format() {
    let toml = r#"
[job]
name = "test-job"
repo = "https://github.com/example/repo"
base_ref = "main"

[environment]
runtime = "docker"
image = "ubuntu:22.04"

[credentials]
provider = "anthropic"
model = "claude-sonnet-4-20250514"

[[session]]
name = "s1"
spec = "s"
harness = "h"
timeout = 60

[merge]
strategy = "sequential"
target = "main"

[forge]
provider = "github"
repo = "no-slash"
token_env = "GITHUB_TOKEN"
"#;
    let manifest = load_from_str(toml).expect("should parse");
    let err = manifest.validate().unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("owner/repo"),
        "should report invalid repo format: {msg}"
    );
}

#[test]
fn test_validate_forge_empty_token_env() {
    let toml = r#"
[job]
name = "test-job"
repo = "https://github.com/example/repo"
base_ref = "main"

[environment]
runtime = "docker"
image = "ubuntu:22.04"

[credentials]
provider = "anthropic"
model = "claude-sonnet-4-20250514"

[[session]]
name = "s1"
spec = "s"
harness = "h"
timeout = 60

[merge]
strategy = "sequential"
target = "main"

[forge]
provider = "github"
repo = "owner/repo"
token_env = ""
"#;
    let manifest = load_from_str(toml).expect("should parse");
    let err = manifest.validate().unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("forge.token_env: must not be empty"),
        "should report empty token_env: {msg}"
    );
}

#[test]
fn test_forge_deny_unknown_fields() {
    let toml = r#"
[job]
name = "test-job"
repo = "https://github.com/example/repo"
base_ref = "main"

[environment]
runtime = "docker"
image = "ubuntu:22.04"

[credentials]
provider = "anthropic"
model = "claude-sonnet-4-20250514"

[[session]]
name = "s1"
spec = "s"
harness = "h"
timeout = 60

[merge]
strategy = "sequential"
target = "main"

[forge]
provider = "github"
repo = "owner/repo"
token_env = "GITHUB_TOKEN"
unknown_field = "oops"
"#;
    let err = load_from_str(toml).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("unknown_field") || msg.contains("unknown field"),
        "should report unknown field in forge: {msg}"
    );
}
