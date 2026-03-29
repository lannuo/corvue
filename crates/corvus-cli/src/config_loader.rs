//! Multi-format configuration loader
//!
//! Supports JSON, TOML, and YAML configuration files.

use crate::config::Config;
use serde::{de::DeserializeOwned, Serialize};
use std::path::{Path, PathBuf};

/// Configuration file format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFormat {
    /// JSON format
    Json,
    /// TOML format
    Toml,
    /// YAML format
    Yaml,
}

impl ConfigFormat {
    /// Detect format from file extension
    pub fn from_path(path: &Path) -> Option<Self> {
        path.extension().and_then(|ext| ext.to_str()).map(|ext| match ext.to_lowercase().as_str() {
            "json" => ConfigFormat::Json,
            "toml" => ConfigFormat::Toml,
            "yaml" | "yml" => ConfigFormat::Yaml,
            _ => ConfigFormat::Json, // Default to JSON
        })
    }

    /// Get the preferred file extension for this format
    pub fn extension(&self) -> &'static str {
        match self {
            ConfigFormat::Json => "json",
            ConfigFormat::Toml => "toml",
            ConfigFormat::Yaml => "yaml",
        }
    }
}

/// Configuration loader with multi-format support
pub struct ConfigLoader {
    /// Default format to use when not specified
    default_format: ConfigFormat,
    /// Config directory
    config_dir: PathBuf,
}

impl ConfigLoader {
    /// Create a new config loader with JSON as default
    pub fn new() -> Self {
        Self::with_format(ConfigFormat::Json)
    }

    /// Create a new config loader with the specified default format
    pub fn with_format(default_format: ConfigFormat) -> Self {
        Self {
            default_format,
            config_dir: crate::config::Config::config_dir(),
        }
    }

    /// Create a new config loader with a custom config directory
    pub fn with_dir(config_dir: PathBuf, default_format: ConfigFormat) -> Self {
        Self {
            default_format,
            config_dir,
        }
    }

    /// Load configuration from the default location
    /// Tries all formats in order: JSON -> TOML -> YAML
    pub fn load(&self) -> anyhow::Result<Config> {
        // Try JSON first
        let json_path = self.config_dir.join("config.json");
        if json_path.exists() {
            return self.load_from(&json_path);
        }

        // Try TOML
        let toml_path = self.config_dir.join("config.toml");
        if toml_path.exists() {
            return self.load_from(&toml_path);
        }

        // Try YAML
        let yaml_path = self.config_dir.join("config.yaml");
        if yaml_path.exists() {
            return self.load_from(&yaml_path);
        }

        // Try YML
        let yml_path = self.config_dir.join("config.yml");
        if yml_path.exists() {
            return self.load_from(&yml_path);
        }

        // No config file found, return default
        Ok(Config::default())
    }

    /// Load configuration from a specific file
    pub fn load_from(&self, path: &Path) -> anyhow::Result<Config> {
        let format = ConfigFormat::from_path(path).unwrap_or(self.default_format);
        let content = std::fs::read_to_string(path)?;
        self.from_str(&content, format)
    }

    /// Parse configuration from string
    pub fn from_str(&self, content: &str, format: ConfigFormat) -> anyhow::Result<Config> {
        match format {
            ConfigFormat::Json => Ok(serde_json::from_str(content)?),
            ConfigFormat::Toml => Ok(toml::from_str(content)?),
            ConfigFormat::Yaml => Ok(serde_yaml::from_str(content)?),
        }
    }

    /// Save configuration to the default location using the default format
    pub fn save(&self, config: &Config) -> anyhow::Result<()> {
        let filename = format!("config.{}", self.default_format.extension());
        let path = self.config_dir.join(filename);
        self.save_to(config, &path)
    }

    /// Save configuration to a specific file
    pub fn save_to(&self, config: &Config, path: &Path) -> anyhow::Result<()> {
        let format = ConfigFormat::from_path(path).unwrap_or(self.default_format);
        let content = self.to_string(config, format)?;

        std::fs::create_dir_all(&self.config_dir)?;
        std::fs::write(path, content)?;

        Ok(())
    }

    /// Serialize configuration to string
    pub fn to_string(&self, config: &Config, format: ConfigFormat) -> anyhow::Result<String> {
        match format {
            ConfigFormat::Json => Ok(serde_json::to_string_pretty(config)?),
            ConfigFormat::Toml => Ok(toml::to_string_pretty(config)?),
            ConfigFormat::Yaml => Ok(serde_yaml::to_string(config)?),
        }
    }

    /// Convert configuration from one format to another
    pub fn convert_format(&self, input: &Path, output: &Path) -> anyhow::Result<()> {
        let config = self.load_from(input)?;
        self.save_to(&config, output)?;
        Ok(())
    }

    /// Get the config directory
    pub fn config_dir(&self) -> &PathBuf {
        &self.config_dir
    }

    /// Get the default format
    pub fn default_format(&self) -> ConfigFormat {
        self.default_format
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Generic multi-format file loader
pub struct MultiFormatLoader;

impl MultiFormatLoader {
    /// Load a type from a file, auto-detecting format
    pub fn load<T: DeserializeOwned>(path: &Path) -> anyhow::Result<T> {
        let format = ConfigFormat::from_path(path).unwrap_or(ConfigFormat::Json);
        let content = std::fs::read_to_string(path)?;
        Self::from_str(&content, format)
    }

    /// Load from string with specified format
    pub fn from_str<T: DeserializeOwned>(content: &str, format: ConfigFormat) -> anyhow::Result<T> {
        match format {
            ConfigFormat::Json => Ok(serde_json::from_str(content)?),
            ConfigFormat::Toml => Ok(toml::from_str(content)?),
            ConfigFormat::Yaml => Ok(serde_yaml::from_str(content)?),
        }
    }

    /// Save a type to a file, auto-detecting format from path
    pub fn save<T: Serialize>(value: &T, path: &Path) -> anyhow::Result<()> {
        let format = ConfigFormat::from_path(path).unwrap_or(ConfigFormat::Json);
        let content = Self::to_string(value, format)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Serialize to string with specified format
    pub fn to_string<T: Serialize>(value: &T, format: ConfigFormat) -> anyhow::Result<String> {
        match format {
            ConfigFormat::Json => Ok(serde_json::to_string_pretty(value)?),
            ConfigFormat::Toml => Ok(toml::to_string_pretty(value)?),
            ConfigFormat::Yaml => Ok(serde_yaml::to_string(value)?),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_config_format_from_path() {
        assert_eq!(
            ConfigFormat::from_path(Path::new("config.json")),
            Some(ConfigFormat::Json)
        );
        assert_eq!(
            ConfigFormat::from_path(Path::new("config.toml")),
            Some(ConfigFormat::Toml)
        );
        assert_eq!(
            ConfigFormat::from_path(Path::new("config.yaml")),
            Some(ConfigFormat::Yaml)
        );
        assert_eq!(
            ConfigFormat::from_path(Path::new("config.yml")),
            Some(ConfigFormat::Yaml)
        );
    }

    #[test]
    fn test_config_format_extension() {
        assert_eq!(ConfigFormat::Json.extension(), "json");
        assert_eq!(ConfigFormat::Toml.extension(), "toml");
        assert_eq!(ConfigFormat::Yaml.extension(), "yaml");
    }

    #[test]
    fn test_config_loader_default() {
        let loader = ConfigLoader::default();
        assert_eq!(loader.default_format(), ConfigFormat::Json);
    }

    #[test]
    fn test_multi_format_roundtrip_json() {
        let config = Config::default();
        let json = MultiFormatLoader::to_string(&config, ConfigFormat::Json).unwrap();
        let parsed: Config = MultiFormatLoader::from_str(&json, ConfigFormat::Json).unwrap();
        assert_eq!(parsed.default_model, config.default_model);
    }

    #[test]
    fn test_multi_format_roundtrip_toml() {
        let config = Config::default();
        let toml = MultiFormatLoader::to_string(&config, ConfigFormat::Toml).unwrap();
        let parsed: Config = MultiFormatLoader::from_str(&toml, ConfigFormat::Toml).unwrap();
        assert_eq!(parsed.default_model, config.default_model);
    }

    #[test]
    fn test_multi_format_roundtrip_yaml() {
        let config = Config::default();
        let yaml = MultiFormatLoader::to_string(&config, ConfigFormat::Yaml).unwrap();
        let parsed: Config = MultiFormatLoader::from_str(&yaml, ConfigFormat::Yaml).unwrap();
        assert_eq!(parsed.default_model, config.default_model);
    }

    #[test]
    fn test_config_loader_save_and_load() {
        let temp_dir = tempdir().unwrap();
        let loader = ConfigLoader::with_dir(temp_dir.path().to_path_buf(), ConfigFormat::Json);

        let mut config = Config::default();
        config.default_model = "test-model".to_string();

        loader.save(&config).unwrap();

        let loaded = loader.load().unwrap();
        assert_eq!(loaded.default_model, "test-model");
    }
}
