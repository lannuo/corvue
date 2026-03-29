//! Corvus Plugin System
//!
//! Provides a plugin system for extending Corvus with custom tools and functionality.

use async_trait::async_trait;
use corvus_core::tool::Tool;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

pub mod error;

pub use error::{PluginError, PluginResult};

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Plugin name
    pub name: String,
    /// Plugin version
    pub version: String,
    /// Plugin description
    pub description: String,
    /// Plugin author
    pub author: String,
}

/// Plugin trait - all plugins must implement this
#[async_trait]
pub trait Plugin: Send + Sync + 'static {
    /// Get plugin metadata
    fn metadata(&self) -> &PluginMetadata;

    /// Initialize the plugin
    async fn initialize(&mut self, context: PluginContext) -> PluginResult<()>;

    /// Get tools provided by this plugin
    fn tools(&self) -> Vec<Arc<dyn Tool>>;

    /// Get as Any for downcasting
    fn as_any(&self) -> &dyn Any;
}

/// Context provided to plugins during initialization
#[derive(Debug, Clone)]
pub struct PluginContext {
    /// Working directory
    pub workdir: PathBuf,
    /// Configuration directory
    pub config_dir: PathBuf,
    /// Plugin-specific configuration
    pub config: Option<serde_json::Value>,
}

/// Plugin manager - loads and manages plugins
pub struct PluginManager {
    /// Loaded plugins
    plugins: HashMap<String, Arc<dyn Plugin>>,
    /// Plugin context
    context: PluginContext,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new(workdir: impl Into<PathBuf>, config_dir: impl Into<PathBuf>) -> Self {
        Self {
            plugins: HashMap::new(),
            context: PluginContext {
                workdir: workdir.into(),
                config_dir: config_dir.into(),
                config: None,
            },
        }
    }

    /// Register a plugin
    pub async fn register_plugin(&mut self, mut plugin: Box<dyn Plugin>) -> PluginResult<()> {
        let name = plugin.metadata().name.clone();

        tracing::info!("Initializing plugin: {} v{}", name, plugin.metadata().version);

        plugin.initialize(self.context.clone()).await?;

        self.plugins.insert(name.clone(), Arc::from(plugin));

        tracing::info!("Plugin registered: {}", name);
        Ok(())
    }

    /// Get all tools from all plugins
    pub fn all_tools(&self) -> Vec<Arc<dyn Tool>> {
        self.plugins
            .values()
            .flat_map(|plugin| plugin.tools())
            .collect()
    }

    /// Get a plugin by name
    pub fn get_plugin(&self, name: &str) -> Option<&Arc<dyn Plugin>> {
        self.plugins.get(name)
    }

    /// List all loaded plugins
    pub fn list_plugins(&self) -> Vec<&PluginMetadata> {
        self.plugins
            .values()
            .map(|p| p.metadata())
            .collect()
    }
}

/// A simple plugin that provides a single tool
pub struct SimpleToolPlugin {
    metadata: PluginMetadata,
    tool: Arc<dyn Tool>,
}

impl SimpleToolPlugin {
    /// Create a new simple tool plugin
    pub fn new(metadata: PluginMetadata, tool: impl Tool + 'static) -> Self {
        Self {
            metadata,
            tool: Arc::new(tool),
        }
    }
}

#[async_trait]
impl Plugin for SimpleToolPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn initialize(&mut self, _context: PluginContext) -> PluginResult<()> {
        Ok(())
    }

    fn tools(&self) -> Vec<Arc<dyn Tool>> {
        vec![self.tool.clone()]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_metadata() {
        let metadata = PluginMetadata {
            name: "test-plugin".to_string(),
            version: "0.1.0".to_string(),
            description: "A test plugin".to_string(),
            author: "Test Author".to_string(),
        };

        assert_eq!(metadata.name, "test-plugin");
        assert_eq!(metadata.version, "0.1.0");
    }
}
