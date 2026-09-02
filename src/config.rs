use serde::Deserialize;
use std::sync::OnceLock;

fn default_host() -> String {
    "127.0.0.1".to_string()
}
fn default_port() -> u16 {
    1180
}
fn default_lists() -> bool {
    false
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_lists")]
    pub lists: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            lists: default_lists(),
        }
    }
}

fn cli_config_path() -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.iter()
        .position(|a| a == "-c" || a == "--config")
        .and_then(|i| args.get(i + 1))
        .cloned()
}

impl Config {
    pub fn load() -> Self {
        let Some(path) = cli_config_path() else {
            return Config::default();
        };
        match std::fs::read_to_string(&path) {
            Ok(content) => match toml::from_str::<Config>(&content) {
                Ok(cfg) => cfg,
                Err(e) => {
                    eprintln!("config parse error {}: {}, using defaults", path, e);
                    Config::default()
                }
            },
            Err(e) => {
                eprintln!("failed to read config {}: {}, using defaults", path, e);
                Config::default()
            }
        }
    }

    /// For tests: load from arbitrary path, falling back to defaults.
    #[cfg(test)]
    pub fn load_from_path(path: &str) -> Self {
        match std::fs::read_to_string(path) {
            Ok(content) => toml::from_str::<Config>(&content).unwrap_or_default(),
            Err(_) => Config::default(),
        }
    }
}

static CONFIG: OnceLock<Config> = OnceLock::new();

pub fn get() -> &'static Config {
    CONFIG.get_or_init(Config::load)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_default() {
        let cfg = Config::default();
        assert_eq!(cfg.host, "127.0.0.1");
        assert_eq!(cfg.port, 1180);
        assert!(!cfg.lists);
    }

    #[test]
    fn test_load_from_path_missing() {
        let cfg = Config::load_from_path("/tmp/nonexistent_tongwen_config_123.toml");
        assert_eq!(cfg.host, "127.0.0.1");
        assert_eq!(cfg.port, 1180);
    }

    #[test]
    fn test_load_partial() {
        let mut f = tempfile_or_fallback();
        writeln!(f.0, "port = 8080").unwrap();
        drop(f.0);
        let cfg = Config::load_from_path(&f.1);
        assert_eq!(cfg.host, "127.0.0.1");
        assert_eq!(cfg.port, 8080);
        assert!(!cfg.lists);
        let _ = std::fs::remove_file(&f.1);
    }

    #[test]
    fn test_load_full() {
        let mut f = tempfile_or_fallback();
        writeln!(f.0, "host = \"0.0.0.0\"\nport = 9000\nlists = true").unwrap();
        drop(f.0);
        let cfg = Config::load_from_path(&f.1);
        assert_eq!(cfg.host, "0.0.0.0");
        assert_eq!(cfg.port, 9000);
        assert!(cfg.lists);
        let _ = std::fs::remove_file(&f.1);
    }

    fn tempfile_or_fallback() -> (std::fs::File, String) {
        let path = format!("/tmp/tongwen_test_{}.toml", uuid::Uuid::new_v4().simple());
        let file = std::fs::File::create(&path).unwrap();
        (file, path)
    }
}
