//! Plugin Lifecycle Management
//!
//! Provides basic plugin lifecycle management.

use crate::error::{PluginError, PluginResult};
use crate::{Plugin, PluginContext, PluginMetadata};
use crate::permission::PermissionManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Plugin state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginState {
    /// Plugin is loaded and active
    Active,
    /// Plugin is loaded but disabled
    Inactive,
    /// Plugin is not loaded
    Unloaded,
    /// Plugin failed to load
    Failed,
}

/// Plugin information with state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    /// Plugin metadata
    pub metadata: PluginMetadata,
    /// Current state
    pub state: PluginState,
}

/// Plugin registry - manages all plugin lifecycle operations
pub struct PluginRegistry {
    /// Loaded plugins
    plugins: HashMap<String, LoadedPlugin>,
}

/// A loaded plugin with additional state
struct LoadedPlugin {
    /// The plugin instance
    plugin: Arc<dyn Plugin>,
    /// Current state
    state: PluginState,
}

impl PluginRegistry {
    /// Create a new plugin registry
    pub fn new(
        _context: PluginContext,
        _permissions: PermissionManager,
    ) -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }

    /// Register an in-memory plugin
    pub async fn register_plugin(&mut self, plugin: Box<dyn Plugin>) -> PluginResult<()> {
        let name = plugin.metadata().name.clone();

        tracing::info!("Registering plugin: {} v{}", name, plugin.metadata().version);

        // Check if plugin is already registered
        if self.plugins.contains_key(&name) {
            return Err(PluginError::Other(format!(
                "Plugin '{}' already registered",
                name
            )));
        }

        // Store the plugin
        self.plugins.insert(
            name.clone(),
            LoadedPlugin {
                plugin: Arc::from(plugin),
                state: PluginState::Active,
            },
        );

        tracing::info!("Plugin registered: {}", name);
        Ok(())
    }

    /// Unload a plugin
    pub fn unload_plugin(&mut self, name: &str) -> PluginResult<()> {
        if self.plugins.remove(name).is_some() {
            tracing::info!("Unloading plugin: {}", name);
            Ok(())
        } else {
            Err(PluginError::NotFound(name.to_string()))
        }
    }

    /// Get a plugin by name
    pub fn get_plugin(&self, name: &str) -> Option<Arc<dyn Plugin>> {
        self.plugins.get(name).and_then(|p| {
            if p.state == PluginState::Active {
                Some(p.plugin.clone())
            } else {
                None
            }
        })
    }

    /// Get all active plugins
    pub fn active_plugins(&self) -> Vec<Arc<dyn Plugin>> {
        self.plugins
            .values()
            .filter(|p| p.state == PluginState::Active)
            .map(|p| p.plugin.clone())
            .collect()
    }

    /// List all loaded plugins
    pub fn list_plugins(&self) -> Vec<PluginInfo> {
        self.plugins
            .values()
            .map(|p| PluginInfo {
                metadata: p.plugin.metadata().clone(),
                state: p.state,
            })
            .collect()
    }

    /// Get all tools from active plugins
    pub fn all_tools(&self) -> Vec<Arc<dyn corvus_core::tool::Tool>> {
        self.active_plugins()
            .iter()
            .flat_map(|plugin| plugin.tools())
            .collect()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new(
            PluginContext {
                workdir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                config_dir: dirs::config_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join("corvus"),
                config: None,
            },
            PermissionManager::default(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_registry_default() {
        let _registry = PluginRegistry::default();
    }

    #[test]
    fn test_plugin_registry_list_plugins_empty() {
        let registry = PluginRegistry::default();
        let plugins = registry.list_plugins();
        assert!(plugins.is_empty());
    }

    #[test]
    fn test_plugin_registry_all_tools_empty() {
        let registry = PluginRegistry::default();
        let tools = registry.all_tools();
        assert!(tools.is_empty());
    }
}
