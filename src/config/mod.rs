pub mod agent_catalog;
pub mod settings;

pub use settings::BmuxConfig;

use std::path::PathBuf;

/// Returns the default config file path: ~/.config/bmux/config.toml
pub fn default_config_path() -> PathBuf {
    let mut path = dirs_config();
    path.push("bmux");
    path.push("config.toml");
    path
}

fn dirs_config() -> PathBuf {
    std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let mut home = home_dir();
            home.push(".config");
            home
        })
}

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

/// Load config from the default path, falling back to defaults on any error.
pub fn load_config() -> BmuxConfig {
    let path = default_config_path();
    BmuxConfig::load_or_default(&path)
}

/// Print the current active configuration to stdout.
pub fn show_config() {
    let config = load_config();
    println!("{}", config.display());
}

/// Validate the config file and report errors without applying changes.
pub fn validate_config() {
    let path = default_config_path();
    match BmuxConfig::load_from_path(&path) {
        Ok(_) => println!("Config OK: {}", path.display()),
        Err(e) => {
            eprintln!("Config error: {e}");
            std::process::exit(1);
        }
    }
}
