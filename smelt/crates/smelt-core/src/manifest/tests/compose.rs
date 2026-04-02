//! Docker Compose / services configuration tests.

use super::*;

#[test]
fn test_compose_manifest_roundtrip_with_services() {
    let manifest = load_from_str(VALID_COMPOSE_MANIFEST).expect("should parse");
    assert_eq!(manifest.services.len(), 2);
    assert_eq!(manifest.services[0].name, "postgres");
    assert_eq!(manifest.services[0].image, "postgres:16");
    // extra keys are present
    assert!(
        manifest.services[0].extra.contains_key("port"),
        "extra should have 'port'"
    );
    assert!(
        manifest.services[0].extra.contains_key("restart"),
        "extra should have 'restart'"
    );
    assert!(
        manifest.services[0].extra.contains_key("command"),
        "extra should have 'command'"
    );
    assert!(
        manifest.services[0].extra.contains_key("tag"),
        "extra should have 'tag'"
    );
    // serde flatten must NOT capture name/image into extra
    assert!(
        !manifest.services[0].extra.contains_key("name"),
        "extra must not contain 'name'"
    );
    assert!(
        !manifest.services[0].extra.contains_key("image"),
        "extra must not contain 'image'"
    );
    // second service is bare
    assert_eq!(manifest.services[1].name, "redis");
    assert_eq!(manifest.services[1].image, "redis:7");
    assert!(
        manifest.services[1].extra.is_empty(),
        "redis extra should be empty"
    );
}

#[test]
fn test_compose_manifest_roundtrip_no_services() {
    // VALID_MANIFEST uses runtime = "docker" and has no [[services]] section
    let manifest = load_from_str(VALID_MANIFEST).expect("should parse");
    assert!(
        manifest.services.is_empty(),
        "docker manifest should have no services"
    );
}

#[test]
fn test_compose_service_extra_does_not_contain_name_or_image() {
    let toml = r#"
[job]
name = "j"
repo = "r"
base_ref = "main"

[environment]
runtime = "compose"
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

[[services]]
name = "mydb"
image = "postgres:16"
port = 5432
"#;
    let manifest = load_from_str(toml).expect("should parse");
    let svc = &manifest.services[0];
    assert!(
        !svc.extra.contains_key("name"),
        "serde flatten must exclude 'name' from extra"
    );
    assert!(
        !svc.extra.contains_key("image"),
        "serde flatten must exclude 'image' from extra"
    );
    assert!(svc.extra.contains_key("port"), "extra should have 'port'");
}

#[test]
fn test_compose_service_passthrough_types() {
    let manifest = load_from_str(VALID_COMPOSE_MANIFEST).expect("should parse");
    let svc = &manifest.services[0]; // postgres with all extra types

    // integer
    let port = svc.extra.get("port").expect("port must be present");
    assert!(
        matches!(port, toml::Value::Integer(5432)),
        "port should be Integer(5432), got {port:?}"
    );

    // boolean
    let restart = svc.extra.get("restart").expect("restart must be present");
    assert!(
        matches!(restart, toml::Value::Boolean(true)),
        "restart should be Boolean(true), got {restart:?}"
    );

    // array
    let command = svc.extra.get("command").expect("command must be present");
    assert!(
        matches!(command, toml::Value::Array(_)),
        "command should be Array, got {command:?}"
    );
    if let toml::Value::Array(arr) = command {
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0], toml::Value::String("pg_isready".to_string()));
    }

    // string
    let tag = svc.extra.get("tag").expect("tag must be present");
    assert!(
        matches!(tag, toml::Value::String(_)),
        "tag should be String, got {tag:?}"
    );
}

#[test]
fn test_validate_compose_service_missing_name() {
    let toml = r#"
[job]
name = "j"
repo = "r"
base_ref = "main"

[environment]
runtime = "compose"
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

[[services]]
name = ""
image = "img"
"#;
    let manifest = load_from_str(toml).expect("should parse");
    let err = manifest.validate().unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("services[0].name"),
        "should report empty service name: {msg}"
    );
}

#[test]
fn test_validate_compose_service_missing_image() {
    let toml = r#"
[job]
name = "j"
repo = "r"
base_ref = "main"

[environment]
runtime = "compose"
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

[[services]]
name = "svc"
image = ""
"#;
    let manifest = load_from_str(toml).expect("should parse");
    let err = manifest.validate().unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("services[0].image"),
        "should report empty service image: {msg}"
    );
}

#[test]
fn test_validate_services_require_compose_runtime() {
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

[[services]]
name = "db"
image = "postgres:16"
"#;
    let manifest = load_from_str(toml).expect("should parse");
    let err = manifest.validate().unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("services:"),
        "should report services error: {msg}"
    );
    assert!(
        msg.contains("compose"),
        "should mention compose runtime: {msg}"
    );
}

#[test]
fn test_validate_compose_empty_services_allowed() {
    let toml = r#"
[job]
name = "j"
repo = "r"
base_ref = "main"

[environment]
runtime = "compose"
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
    assert!(manifest.services.is_empty());
    manifest
        .validate()
        .expect("compose with no services should be valid");
}
