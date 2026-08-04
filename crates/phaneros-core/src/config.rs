use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to determine standard system configuration directory")]
    ConfigDirNotFound,
    #[error("Failed to read configuration file at {path}: {source}")]
    ReadFileFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to parse TOML configuration at {path}: {source}")]
    ParseFailed {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("Failed to serialize configuration to TOML: {0}")]
    SerializeFailed(#[from] toml::ser::Error),
    #[error("Failed to write configuration file at {path}: {source}")]
    WriteFileFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct PhanerosConfig {
    #[serde(default)]
    pub daemon: DaemonSettings,
    #[serde(default)]
    pub drives: BTreeMap<String, DriveConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct DaemonSettings {
    #[serde(default = "default_store_url")]
    pub store_url: String,

    #[serde(default = "default_log_level")]
    pub log_level: String,

    pub ipc_socket: Option<PathBuf>,

    #[serde(default = "default_max_concurrent_uploads")]
    pub max_concurrent_uploads: usize,

    #[serde(default = "default_compression")]
    pub compression: String,

    #[serde(default = "default_false")]
    pub enable_telemetry: bool,
}

fn default_false() -> bool {
    false
}

fn default_store_url() -> String {
    "http://localhost:8080".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_max_concurrent_uploads() -> usize {
    4
}

fn default_compression() -> String {
    "zstd".to_string()
}

impl Default for DaemonSettings {
    fn default() -> Self {
        Self {
            store_url: default_store_url(),
            log_level: default_log_level(),
            ipc_socket: None,
            max_concurrent_uploads: default_max_concurrent_uploads(),
            compression: default_compression(),
            enable_telemetry: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct DriveConfig {
    pub path: PathBuf,

    #[serde(default)]
    pub token: String,

    pub store_url: Option<String>,

    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

impl DriveConfig {
    pub fn new(path: PathBuf, token: String, store_url: Option<String>) -> Self {
        Self {
            path,
            token,
            store_url,
            enabled: true,
        }
    }

    /// Resolves the effective store URL for this drive, falling back to global daemon store_url if absent.
    pub fn get_effective_store_url<'a>(&'a self, daemon_store_url: &'a str) -> &'a str {
        self.store_url
            .as_deref()
            .unwrap_or(daemon_store_url)
    }

    /// Returns the expanded local path (resolving leading `~`).
    pub fn expanded_path(&self) -> PathBuf {
        expand_tilde(&self.path)
    }
}

impl Default for PhanerosConfig {
    fn default() -> Self {
        let mut drives = BTreeMap::new();
        drives.insert(
            "default".to_string(),
            DriveConfig {
                path: PathBuf::from("~/Documents/Phaneros"),
                token: String::new(),
                store_url: None,
                enabled: true,
            },
        );

        Self {
            daemon: DaemonSettings::default(),
            drives,
        }
    }
}

impl PhanerosConfig {
    /// Returns the default OS-specific configuration path (`~/.config/phaneros/config.toml` on Unix/macOS).
    pub fn default_config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|dir| dir.join("phaneros").join("config.toml"))
    }

    /// Loads configuration from a given file path.
    pub fn load_from_path(path: &Path) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path).map_err(|e| ConfigError::ReadFileFailed {
            path: path.to_path_buf(),
            source: e,
        })?;

        let config: PhanerosConfig =
            toml::from_str(&content).map_err(|e| ConfigError::ParseFailed {
                path: path.to_path_buf(),
                source: e,
            })?;

        Ok(config)
    }

    /// Loads configuration from custom path if provided, or from standard default path.
    /// If default file does not exist, returns a default `PhanerosConfig`.
    pub fn load_or_default(custom_path: Option<&Path>) -> Result<(Self, PathBuf), ConfigError> {
        let target_path = match custom_path {
            Some(p) => p.to_path_buf(),
            None => Self::default_config_path().ok_or(ConfigError::ConfigDirNotFound)?,
        };

        if target_path.exists() {
            let config = Self::load_from_path(&target_path)?;
            Ok((config, target_path))
        } else {
            Ok((Self::default(), target_path))
        }
    }

    /// Saves the configuration to the specified path, creating parent directories if necessary.
    pub fn save_to_path(&self, path: &Path) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| ConfigError::WriteFileFailed {
                path: path.to_path_buf(),
                source: e,
            })?;
        }

        let toml_string = toml::to_string_pretty(self)?;
        fs::write(path, toml_string).map_err(|e| ConfigError::WriteFileFailed {
            path: path.to_path_buf(),
            source: e,
        })?;

        Ok(())
    }
}

/// Helper function to expand `~` in paths to the user's home directory.
pub fn expand_tilde(path: &Path) -> PathBuf {
    if let Ok(stripped) = path.strip_prefix("~") {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped);
        }
    }
    path.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_parse_valid_config_toml() {
        let toml_str = r#"
[daemon]
store_url = "http://my-remote-store:8080"
log_level = "debug"
ipc_socket = "/tmp/phaneros_test.sock"
max_concurrent_uploads = 8
compression = "none"

[drives.default]
path = "/home/user/sync"
token = "token-123"
enabled = true

[drives.work]
path = "/home/user/work"
store_url = "http://work-store:8080"
token = "work-token"
enabled = false
"#;

        let config: PhanerosConfig = toml::from_str(toml_str).unwrap();

        assert_eq!(config.daemon.store_url, "http://my-remote-store:8080");
        assert_eq!(config.daemon.log_level, "debug");
        assert_eq!(
            config.daemon.ipc_socket,
            Some(PathBuf::from("/tmp/phaneros_test.sock"))
        );
        assert_eq!(config.daemon.max_concurrent_uploads, 8);
        assert_eq!(config.daemon.compression, "none");

        assert_eq!(config.drives.len(), 2);

        let default_drive = config.drives.get("default").unwrap();
        assert_eq!(default_drive.path, PathBuf::from("/home/user/sync"));
        assert_eq!(
            default_drive.get_effective_store_url(&config.daemon.store_url),
            "http://my-remote-store:8080"
        );
        assert!(default_drive.enabled);

        let work_drive = config.drives.get("work").unwrap();
        assert_eq!(work_drive.path, PathBuf::from("/home/user/work"));
        assert_eq!(
            work_drive.get_effective_store_url(&config.daemon.store_url),
            "http://work-store:8080"
        );
        assert!(!work_drive.enabled);
    }

    #[tokio::test]
    async fn test_roundtrip_save_and_load() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("config.toml");

        let mut config = PhanerosConfig::default();
        config.daemon.store_url = "http://test-server:9000".to_string();
        config.drives.insert(
            "custom".to_string(),
            DriveConfig::new(
                PathBuf::from("/tmp/test_dir"),
                "secret".to_string(),
                Some("http://custom-store:9000".to_string()),
            ),
        );

        config.save_to_path(&file_path).unwrap();

        let loaded_config = PhanerosConfig::load_from_path(&file_path).unwrap();
        assert_eq!(config, loaded_config);
    }
}
