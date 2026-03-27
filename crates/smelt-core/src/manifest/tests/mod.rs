//! Tests for the manifest module, organized by domain.

use std::path::Path;

use super::*;

mod compose;
mod core;
mod forge;
mod kubernetes;

/// Minimal valid docker-runtime manifest used across test submodules.
pub(super) const VALID_MANIFEST: &str = r#"
[job]
name = "test-job"
repo = "https://github.com/example/repo"
base_ref = "main"

[environment]
runtime = "docker"
image = "ubuntu:22.04"

[environment.resources]
cpu = "2"
memory = "4G"

[credentials]
provider = "anthropic"
model = "claude-sonnet-4-20250514"

[credentials.env]
api_key = "ANTHROPIC_API_KEY"

[[session]]
name = "frontend"
spec = "Implement the login page"
harness = "npm test"
timeout = 300

[[session]]
name = "backend"
spec = "Implement the auth endpoint"
harness = "cargo test"
timeout = 600
depends_on = ["frontend"]

[merge]
strategy = "sequential"
order = ["frontend", "backend"]
ai_resolution = true
target = "main"
"#;

/// Minimal compose manifest with two `[[services]]` entries.
pub(super) const VALID_COMPOSE_MANIFEST: &str = r#"
[job]
name = "compose-job"
repo = "https://github.com/example/repo"
base_ref = "main"

[environment]
runtime = "compose"
image = "ubuntu:22.04"

[credentials]
provider = "anthropic"
model = "claude-sonnet-4-20250514"

[[session]]
name = "run"
spec = "Run the suite"
harness = "pytest"
timeout = 300

[merge]
strategy = "sequential"
target = "main"

[[services]]
name = "postgres"
image = "postgres:16"
port = 5432
restart = true
command = ["pg_isready", "-U", "postgres"]
tag = "db"

[[services]]
name = "redis"
image = "redis:7"
"#;

/// Parse a manifest from a TOML string using a synthetic source path.
pub(super) fn load_from_str(content: &str) -> crate::Result<JobManifest> {
    JobManifest::from_str(content, Path::new("test.toml"))
}
