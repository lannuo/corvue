//! Plugin Lifecycle Management
//!
//! Provides comprehensive plugin lifecycle management: load, unload, reload, enable, disable.

use crate::error::{PluginError, PluginResult};
use crate::{Plugin, PluginContext, PluginMetadata};
use crate::permission::{PermissionManager, PermissionSet};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

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
    /// Load time (if loaded)
    pub loaded_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Error message if failed
    pub error: Option<String>,
    /// Path to plugin file (if applicable)
    pub path: Option<PathBuf>,
}

/// Plugin loader trait - for loading plugins from different sources
#[async_trait]
pub trait PluginLoader: Send + Sync + 'static {
    /// Check if this loader can load from the given path
    fn can_load(&self, path: &Path) -> bool;

    /// Load a plugin from the given path
    async fn load(&self, path: &Path, context: PluginContext) -> PluginResult<Box<dyn Plugin>>;

    /// Get the name of this loader
    fn name(&self) -> &'static str;
}

/// File system plugin loader - loads from .wasm or .dll files
pub struct FilePluginLoader;

#[async_trait]
impl PluginLoader for FilePluginLoader {
    fn can_load(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension() {
            ext == "wasm" || ext == "dll" || ext == "so" || ext == "dylib"
        } else {
            false
        }
    }

    async fn load(&self, _path: &Path, _context: PluginContext) -> PluginResult<Box<dyn Plugin>> {
        // This is a placeholder - actual implementation would load the plugin
        Err(PluginError::LoadError(
            "File plugin loading not implemented yet".to_string(),
        ))
    }

    fn name(&self) -> &'static str {
        "file"
    }
}

/// Plugin registry - manages all plugin lifecycle operations
pub struct PluginRegistry {
    /// Loaded plugins with state
    plugins: RwLock<HashMap<String, LoadedPlugin>>,
    /// Plugin context
    context: PluginContext,
    /// Permission manager
    permissions: PermissionManager,
    /// Plugin loaders
    loaders: Vec<Arc<dyn PluginLoader>>,
    /// Plugin directory (for auto-discovery)
    plugin_dir: Option<PathBuf>,
}

/// A loaded plugin with additional state
struct LoadedPlugin {
    /// The plugin instance
    plugin: Arc<dyn Plugin>,
    /// Current state
    state: PluginState,
    /// Load time
    loaded_at: chrono::DateTime<chrono::Utc>,
    /// Path to plugin file (if any)
    path: Option<PathBuf>,
    /// Last error (if any)
    last_error: Option<String>,
}

impl PluginRegistry {
    /// Create a new plugin registry
    pub fn new(
        context: PluginContext,
        permissions: PermissionManager,
        plugin_dir: Option<PathBuf>,
    ) -> Self {
        let mut registry = Self {
            plugins: RwLock::new(HashMap::new()),
            context,
            permissions,
            loaders: Vec::new(),
            plugin_dir,
        };

        // Register default loaders
        registry.register_loader(Arc::new(FilePluginLoader));

        registry
    }

    /// Register a plugin loader
    pub fn register_loader(&mut self, loader: Arc<dyn PluginLoader>) {
        self.loaders.push(loader);
    }

    /// Register an in-memory plugin
    pub async fn register_plugin(&self, plugin: Box<dyn Plugin>) -> PluginResult<()> {
        let name = plugin.metadata().name.clone();
        let metadata = plugin.metadata().clone();

        tracing::info!("Registering plugin: {} v{}", name, metadata.version);

        // Check if plugin is already registered
        {
            let plugins = self.plugins.read().await;
            if plugins.contains_key(&name) {
                return Err(PluginError::LoadError(format!(
                    "Plugin '{}' already registered",
                    name
                )));
            }
        }

        // Initialize the plugin
        let mut plugin = plugin;
        plugin.initialize(self.context.clone()).await?;

        // Store the plugin
        let mut plugins = self.plugins.write().await;
        plugins.insert(
            name.clone(),
            LoadedPlugin {
                plugin: Arc::from(plugin),
                state: PluginState::Active,
                loaded_at: chrono::Utc::now(),
                path: None,
                last_error: None,
            },
        );

        tracing::info!("Plugin registered: {}", name);
        Ok(())
    }

    /// Load a plugin from a file
    pub async fn load_plugin(&self, path: impl AsRef<Path>) -> PluginResult<()> {
        let path = path.as_ref();

        // Find a loader that can handle this path
        let loader = self
            .loaders
            .iter()
            .find(|l| l.can_load(path))
            .ok_or_else(|| {
                PluginError::LoadError(format!("No loader found for {:?}", path))
            })?;

        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        tracing::info!("Loading plugin from: {:?}", path);

        // Load the plugin
        let plugin = loader.load(path, self.context.clone()).await?;
        let metadata = plugin.metadata().clone();

        // Check for existing plugin
        {
            let plugins = self.plugins.read().await;
            if plugins.contains_key(&metadata.name) {
                return Err(PluginError::LoadError(format!(
                    "Plugin '{}' already loaded",
                    metadata.name
                )));
            }
        }

        // Store the plugin
        let mut plugins = self.plugins.write().await;
        plugins.insert(
            metadata.name.clone(),
            LoadedPlugin {
                plugin: Arc::from(plugin),
                state: PluginState::Active,
                loaded_at: chrono::Utc::now(),
                path: Some(path.to_path_buf()),
                last_error: None,
            },
        );

        tracing::info!("Plugin loaded: {} v{}", metadata.name, metadata.version);
        Ok(())
    }

    /// Unload a plugin
    pub async fn unload_plugin(&self, name: &str) -> PluginResult<()> {
        let mut plugins = self.plugins.write().await;

        if let Some(mut loaded) = plugins.remove(name) {
            tracing::info!("Unloading plugin: {}", name);
            loaded.state = PluginState::Unloaded;
            Ok(())
        } else {
            Err(PluginError::PluginNotFound(name.to_string()))
        }
    }

    /// Reload a plugin
    pub async fn reload_plugin(&self, name: &str) -> PluginResult<()> {
        // Get the plugin path first
        let path = {
            let plugins = self.plugins.read().await;
            plugins
                .get(name)
                .and_then(|p| p.path.clone())
                .ok_or_else(|| PluginError::PluginNotFound(name.to_string()))?
        };

        // Unload the plugin
        self.unload_plugin(name).await?;

        // Reload it
        self.load_plugin(&path).await
    }

    /// Enable a disabled plugin
    pub async fn enable_plugin(&self, name: &str) -> PluginResult<()> {
        let mut plugins = self.plugins.write().await;

        if let Some(plugin) = plugins.get_mut(name) {
            if plugin.state == PluginState::Inactive {
                plugin.state = PluginState::Active;
                tracing::info!("Plugin enabled: {}", name);
                Ok(())
            } else {
                Err(PluginError::LoadError(format!(
                    "Plugin '{}' is not inactive",
                    name
                )))
            }
        } else {
            Err(PluginError::PluginNotFound(name.to_string()))
        }
    }

    /// Disable an active plugin
    pub async fn disable_plugin(&self, name: &str) -> PluginResult<()> {
        let mut plugins = self.plugins.write().await;

        if let Some(plugin) = plugins.get_mut(name) {
            if plugin.state == PluginState::Active {
                plugin.state = PluginState::Inactive;
                tracing::info!("Plugin disabled: {}", name);
                Ok(())
            } else {
                Err(PluginError::LoadError(format!(
                    "Plugin '{}' is not active",
                    name
                )))
            }
        } else {
            Err(PluginError::PluginNotFound(name.to_string()))
        }
    }

    /// Get a plugin by name
    pub async fn get_plugin(&self, name: &str) -> Option<Arc<dyn Plugin>> {
        let plugins = self.plugins.read().await;
        plugins.get(name).and_then(|p| {
            if p.state == PluginState::Active {
                Some(p.plugin.clone())
            } else {
                None
            }
        })
    }

    /// Get all active plugins
    pub async fn active_plugins(&self) -> Vec<Arc<dyn Plugin>> {
        let plugins = self.plugins.read().await;
        plugins
            .values()
            .filter(|p| p.state == PluginState::Active)
            .map(|p| p.plugin.clone())
            .collect()
    }

    /// Get information about all plugins
    pub async fn list_plugins(&self) -> Vec<PluginInfo> {
        let plugins = self.plugins.read().await;
        plugins
            .values()
            .map(|p| PluginInfo {
                metadata: p.plugin.metadata().clone(),
                state: p.state,
                loaded_at: Some(p.loaded_at),
                error: p.last_error.clone(),
                path: p.path.clone(),
            })
            .collect()
    }

    /// Get all tools from active plugins
    pub async fn all_tools(&self) -> Vec<Arc<dyn corvus_core::tool::Tool>> {
        let plugins = self.active_plugins().await;
        plugins
            .iter()
            .flat_map(|plugin| plugin.tools())
            .collect()
    }

    /// Auto-discover and load plugins from the plugin directory
    pub async fn discover_plugins(&self) -> PluginResult<Vec<String>> {
        let Some(plugin_dir) = &self.plugin_dir else {
            return Ok(Vec::new());
        };

        if !plugin_dir.exists() {
            return Ok(Vec::new());
        }

        let mut loaded = Vec::new();

        let mut entries = tokio::fs::read_dir(plugin_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() {
                for loader in &self.loaders {
                    if loader.can_load(&path) {
                        match self.load_plugin(&path).await {
                            Ok(_) => {
                                loaded.push(
                                    path.file_name()
                                        .and_then(|s| s.to_str())
                                        .unwrap_or("unknown")
                                        .to_string(),
                                );
                            }
                            Err(e) => {
                                tracing::warn!("Failed to load plugin {:?}: {}", path, e);
                            }
                        }
                        break;
                    }
                }
            }
        }

        Ok(loaded)
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
            None,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_state_default() {
        assert_eq!(PluginState::Unloaded as u8, PluginState::Unloaded as u8);
    }

    #[test]
    fn test_plugin_registry_default() {
        let _registry = PluginRegistry::default();
    }

    #[tokio::test]
    async fn test_plugin_registry_list_plugins_empty() {
        let registry = PluginRegistry::default();
        let plugins = registry.list_plugins().await;
        assert!(plugins.is_empty());
    }

    #[tokio::test]
    async fn test_plugin_registry_all_tools_empty() {
        let registry = PluginRegistry::default();
        let tools = registry.all_tools().await;
        assert!(tools.is_empty());
    }
}
