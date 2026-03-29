//! Plugin system for Corvus
//!
//! This module provides the foundational traits for extending Corvus with plugins.

use crate::error::Result;
use crate::tool::Tool;
use std::any::Any;
use std::sync::Arc;

/// Plugin metadata
#[derive(Debug, Clone)]
pub struct PluginMetadata {
    /// Plugin name
    pub name: String,
    /// Plugin version
    pub version: String,
    /// Plugin description
    pub description: String,
    /// Plugin author
    pub author: Option<String>,
}

/// Trait for Corvus plugins
pub trait Plugin: Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> PluginMetadata;

    /// Initialize the plugin
    fn initialize(&mut self) -> Result<()> {
        Ok(())
    }

    /// Get tools provided by this plugin
    fn tools(&self) -> Vec<Arc<dyn Tool>> {
        Vec::new()
    }

    /// Get the plugin as Any for downcasting
    fn as_any(&self) -> &dyn Any;

    /// Get the plugin as mutable Any for downcasting
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Plugin manager for loading and managing plugins
pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    /// Register a plugin
    pub fn register(&mut self, plugin: Box<dyn Plugin>) -> Result<()> {
        let mut plugin = plugin;
        plugin.initialize()?;
        self.plugins.push(plugin);
        Ok(())
    }

    /// Get all registered plugins
    pub fn plugins(&self) -> &[Box<dyn Plugin>] {
        &self.plugins
    }

    /// Get all tools from all registered plugins
    pub fn all_tools(&self) -> Vec<Arc<dyn Tool>> {
        let mut tools = Vec::new();
        for plugin in &self.plugins {
            tools.extend(plugin.tools());
        }
        tools
    }

    /// Find a plugin by name
    pub fn find_plugin(&self, name: &str) -> Option<&dyn Plugin> {
        self.plugins
            .iter()
            .find(|p| p.metadata().name == name)
            .map(|p| p.as_ref())
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::Any;

    struct TestPlugin {
        initialized: bool,
    }

    impl TestPlugin {
        fn new() -> Self {
            Self { initialized: false }
        }
    }

    impl Plugin for TestPlugin {
        fn metadata(&self) -> PluginMetadata {
            PluginMetadata {
                name: "test-plugin".to_string(),
                version: "0.1.0".to_string(),
                description: "A test plugin".to_string(),
                author: Some("Test Author".to_string()),
            }
        }

        fn initialize(&mut self) -> Result<()> {
            self.initialized = true;
            Ok(())
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    #[test]
    fn test_plugin_metadata() {
        let plugin = TestPlugin::new();
        let metadata = plugin.metadata();
        assert_eq!(metadata.name, "test-plugin");
        assert_eq!(metadata.version, "0.1.0");
    }

    #[test]
    fn test_plugin_manager() {
        let mut manager = PluginManager::new();
        manager.register(Box::new(TestPlugin::new())).unwrap();
        assert_eq!(manager.plugins().len(), 1);

        let plugin = manager.find_plugin("test-plugin");
        assert!(plugin.is_some());
    }
}
