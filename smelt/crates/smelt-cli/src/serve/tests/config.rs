use crate::serve::config::ServerConfig;

#[test]
fn test_worker_config_roundtrip() {
    let toml = r#"
queue_dir = "/tmp/smelt-queue"
max_concurrent = 2

[[workers]]
host = "worker1.example.com"
user = "smelt"
key_env = "WORKER_SSH_KEY"
port = 2222
"#;
    let config: ServerConfig = toml::from_str(toml).expect("valid TOML with workers should parse");
    assert_eq!(config.workers.len(), 1);
    let w = &config.workers[0];
    assert_eq!(w.host, "worker1.example.com");
    assert_eq!(w.user, "smelt");
    assert_eq!(w.key_env, "WORKER_SSH_KEY");
    assert_eq!(w.port, 2222);
}

#[test]
fn test_worker_config_defaults() {
    let toml = r#"
queue_dir = "/tmp/smelt-queue"
max_concurrent = 2

[[workers]]
host = "worker1.example.com"
user = "smelt"
key_env = "WORKER_SSH_KEY"
"#;
    let config: ServerConfig = toml::from_str(toml).expect("worker without port should parse");
    assert_eq!(config.workers.len(), 1);
    assert_eq!(config.workers[0].port, 22, "default port should be 22");
}

#[test]
fn test_server_config_no_workers_parses() {
    let toml = r#"
queue_dir = "/tmp/smelt-queue"
max_concurrent = 2
"#;
    let config: ServerConfig = toml::from_str(toml).expect("config without workers should parse");
    assert!(
        config.workers.is_empty(),
        "workers should default to empty vec"
    );
    assert_eq!(
        config.ssh_timeout_secs, 3,
        "ssh_timeout_secs should default to 3"
    );
}

#[test]
fn test_worker_config_deny_unknown_fields() {
    let toml = r#"
queue_dir = "/tmp/smelt-queue"
max_concurrent = 2

[[workers]]
host = "worker1.example.com"
user = "smelt"
key_env = "WORKER_SSH_KEY"
unknown_field = "should fail"
"#;
    let result: Result<ServerConfig, _> = toml::from_str(toml);
    assert!(
        result.is_err(),
        "unknown field in [[workers]] should fail to parse"
    );
}

#[test]
fn test_worker_config_empty_host_fails_validation() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, r#"queue_dir = "/tmp/smelt-queue""#).unwrap();
    writeln!(f, "max_concurrent = 2").unwrap();
    writeln!(f).unwrap();
    writeln!(f, "[[workers]]").unwrap();
    writeln!(f, r#"host = """#).unwrap();
    writeln!(f, r#"user = "smelt""#).unwrap();
    writeln!(f, r#"key_env = "WORKER_SSH_KEY""#).unwrap();
    f.flush().unwrap();

    let result = ServerConfig::load(f.path());
    assert!(result.is_err(), "empty host should fail validation");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("host"),
        "error should mention 'host', got: {err_msg}"
    );
}

#[test]
fn test_worker_config_empty_user_fails_validation() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, r#"queue_dir = "/tmp/smelt-queue""#).unwrap();
    writeln!(f, "max_concurrent = 2").unwrap();
    writeln!(f).unwrap();
    writeln!(f, "[[workers]]").unwrap();
    writeln!(f, r#"host = "worker1.example.com""#).unwrap();
    writeln!(f, r#"user = """#).unwrap();
    writeln!(f, r#"key_env = "WORKER_SSH_KEY""#).unwrap();
    f.flush().unwrap();

    let result = ServerConfig::load(f.path());
    assert!(result.is_err(), "empty user should fail validation");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("user"),
        "error should mention 'user', got: {err_msg}"
    );
}

#[test]
fn test_server_config_roundtrip() {
    let toml = r#"
queue_dir = "/tmp/smelt-queue"
max_concurrent = 4
retry_attempts = 5
retry_backoff_secs = 10

[server]
host = "0.0.0.0"
port = 9000
"#;
    let config: ServerConfig = toml::from_str(toml).expect("valid TOML should parse");
    assert_eq!(
        config.queue_dir,
        std::path::PathBuf::from("/tmp/smelt-queue")
    );
    assert_eq!(config.max_concurrent, 4);
    assert_eq!(config.retry_attempts, 5);
    assert_eq!(config.retry_backoff_secs, 10);
    assert_eq!(config.server.host, "0.0.0.0");
    assert_eq!(config.server.port, 9000);
}

#[test]
fn test_server_config_missing_queue_dir() {
    let toml = r#"
max_concurrent = 2
"#;
    let result: Result<ServerConfig, _> = toml::from_str(toml);
    assert!(
        result.is_err(),
        "missing required field queue_dir should fail"
    );
}

#[test]
fn test_server_config_invalid_max_concurrent() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, r#"queue_dir = "/tmp/smelt-queue""#).unwrap();
    writeln!(f, "max_concurrent = 0").unwrap();
    f.flush().unwrap();

    let result = ServerConfig::load(f.path());
    assert!(result.is_err(), "max_concurrent=0 should fail validation");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("max_concurrent"),
        "error message should mention 'max_concurrent', got: {err_msg}"
    );
}
