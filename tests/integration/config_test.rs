use bmux::config::settings::BmuxConfig;
use std::io::Write;
use tempfile::NamedTempFile;

fn toml_file(content: &str) -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f
}

#[test]
fn integration_config_full_load() {
    let f = toml_file(
        r#"
prefix = "ctrl-a"
history_limit = 20000
default_shell = "/usr/bin/fish"
mouse_support = true

[agents.claude_code]
binary = "/usr/local/bin/claude"

[agents.opencode]
binary = "oc"

[agents.pi]
binary = "pi-agent"
"#,
    );
    let cfg = BmuxConfig::load_from_path(f.path()).expect("should parse");
    assert_eq!(cfg.prefix, "ctrl-a");
    assert_eq!(cfg.history_limit, 20000);
    assert_eq!(cfg.default_shell, "/usr/bin/fish");
    assert!(cfg.mouse_support);
    assert_eq!(cfg.agents.claude_code.binary, "/usr/local/bin/claude");
    assert_eq!(cfg.agents.opencode.binary, "oc");
    assert_eq!(cfg.agents.pi.binary, "pi-agent");
}

#[test]
fn integration_config_defaults_when_file_absent() {
    let cfg = BmuxConfig::load_or_default(std::path::Path::new(
        "/absolutely/does/not/exist.toml",
    ));
    assert_eq!(cfg.prefix, "ctrl-b");
    assert_eq!(cfg.history_limit, 10_000);
    assert!(!cfg.mouse_support);
}

#[test]
fn integration_hot_reload_success() {
    let f = toml_file(r#"prefix = "ctrl-space""#);
    let result = BmuxConfig::hot_reload(f.path());
    assert!(result.is_ok());
    assert_eq!(result.unwrap().prefix, "ctrl-space");
}

#[test]
fn integration_hot_reload_invalid_keeps_error_message() {
    let f = toml_file("this {{ is not toml }");
    let result = BmuxConfig::hot_reload(f.path());
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(!err.is_empty(), "Error message should not be empty");
}

#[test]
fn integration_config_example_file_is_valid_toml() {
    // Validate that the example config file ships valid TOML
    let example_path = std::path::Path::new(
        concat!(env!("CARGO_MANIFEST_DIR"), "/.config/bmux/config.toml.example"),
    );
    assert!(example_path.exists(), "config.toml.example must exist");

    // Strip comment lines that have inline values — the example uses commented-out
    // lines for optional fields. Load as-is; toml crate ignores comments.
    let result = BmuxConfig::load_from_path(example_path);
    assert!(
        result.is_ok(),
        "config.toml.example must parse cleanly: {:?}",
        result.err()
    );
}
