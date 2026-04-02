//! Kubernetes runtime configuration tests.

use super::*;

const KUBERNETES_MANIFEST: &str = r#"
[job]
name = "kube-job"
repo = "https://github.com/example/repo"
base_ref = "main"

[environment]
runtime = "kubernetes"
image = "ubuntu:22.04"

[credentials]
provider = "anthropic"
model = "claude-sonnet-4-20250514"

[[session]]
name = "agent"
spec = "Run the task"
harness = "kata run"
timeout = 600

[merge]
strategy = "sequential"
target = "main"

[kubernetes]
namespace = "smelt-jobs"
context = "my-cluster"
ssh_key_env = "SSH_PRIVATE_KEY"
cpu_request = "500m"
memory_request = "512Mi"
cpu_limit = "2"
memory_limit = "2Gi"
"#;

#[test]
fn test_kubernetes_roundtrip_present() {
    let manifest = load_from_str(KUBERNETES_MANIFEST).expect("should parse");
    let kube = manifest
        .kubernetes
        .as_ref()
        .expect("kubernetes should be Some");
    assert_eq!(kube.namespace, "smelt-jobs");
    assert_eq!(kube.context.as_deref(), Some("my-cluster"));
    assert_eq!(kube.ssh_key_env, "SSH_PRIVATE_KEY");
    assert_eq!(kube.cpu_request.as_deref(), Some("500m"));
    assert_eq!(kube.memory_request.as_deref(), Some("512Mi"));
    assert_eq!(kube.cpu_limit.as_deref(), Some("2"));
    assert_eq!(kube.memory_limit.as_deref(), Some("2Gi"));
}

#[test]
fn test_kubernetes_roundtrip_absent() {
    let manifest = load_from_str(VALID_MANIFEST).expect("should parse");
    assert!(
        manifest.kubernetes.is_none(),
        "kubernetes should be None when no [kubernetes] section"
    );
}

#[test]
fn test_validate_kubernetes_runtime_requires_block() {
    let toml = r#"
[job]
name = "j"
repo = "r"
base_ref = "main"

[environment]
runtime = "kubernetes"
image = "img"

[credentials]
provider = "anthropic"
model = "m"

[[session]]
name = "s"
spec = "s"
harness = "h"
timeout = 60

[merge]
strategy = "sequential"
target = "main"
"#;
    let manifest = load_from_str(toml).expect("should parse");
    let err = manifest.validate().unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("kubernetes"),
        "should report kubernetes error: {msg}"
    );
}

#[test]
fn test_validate_kubernetes_block_requires_runtime() {
    let toml = r#"
[job]
name = "j"
repo = "r"
base_ref = "main"

[environment]
runtime = "docker"
image = "img"

[credentials]
provider = "anthropic"
model = "m"

[[session]]
name = "s"
spec = "s"
harness = "h"
timeout = 60

[merge]
strategy = "sequential"
target = "main"

[kubernetes]
namespace = "smelt-jobs"
ssh_key_env = "SSH_PRIVATE_KEY"
"#;
    let manifest = load_from_str(toml).expect("should parse");
    let err = manifest.validate().unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("kubernetes"),
        "should report kubernetes error: {msg}"
    );
}

#[test]
fn test_validate_kubernetes_empty_namespace() {
    let toml = r#"
[job]
name = "j"
repo = "r"
base_ref = "main"

[environment]
runtime = "kubernetes"
image = "img"

[credentials]
provider = "anthropic"
model = "m"

[[session]]
name = "s"
spec = "s"
harness = "h"
timeout = 60

[merge]
strategy = "sequential"
target = "main"

[kubernetes]
namespace = ""
ssh_key_env = "SSH_PRIVATE_KEY"
"#;
    let manifest = load_from_str(toml).expect("should parse");
    let err = manifest.validate().unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("namespace"),
        "should report namespace error: {msg}"
    );
}

#[test]
fn test_validate_kubernetes_empty_ssh_key_env() {
    let toml = r#"
[job]
name = "j"
repo = "r"
base_ref = "main"

[environment]
runtime = "kubernetes"
image = "img"

[credentials]
provider = "anthropic"
model = "m"

[[session]]
name = "s"
spec = "s"
harness = "h"
timeout = 60

[merge]
strategy = "sequential"
target = "main"

[kubernetes]
namespace = "smelt-jobs"
ssh_key_env = ""
"#;
    let manifest = load_from_str(toml).expect("should parse");
    let err = manifest.validate().unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("ssh_key_env"),
        "should report ssh_key_env error: {msg}"
    );
}

#[test]
fn test_validate_kubernetes_valid() {
    let manifest = load_from_str(KUBERNETES_MANIFEST).expect("should parse");
    manifest
        .validate()
        .expect("fully valid kubernetes manifest should pass validation");
}

#[test]
fn test_validate_runtime_compose_valid() {
    let manifest = load_from_str(VALID_COMPOSE_MANIFEST).expect("should parse");
    manifest
        .validate()
        .expect("compose manifest with services should be valid");
}

#[test]
fn test_validate_runtime_unknown_rejected() {
    let toml = r#"
[job]
name = "j"
repo = "r"
base_ref = "main"

[environment]
runtime = "podman"
image = "img"

[credentials]
provider = "anthropic"
model = "m"

[[session]]
name = "s"
spec = "s"
harness = "h"
timeout = 60

[merge]
strategy = "sequential"
target = "main"
"#;
    let manifest = load_from_str(toml).expect("should parse");
    let err = manifest.validate().unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("environment.runtime"),
        "should report unknown runtime: {msg}"
    );
}
