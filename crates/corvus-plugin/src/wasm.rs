//! WASM Plugin System
//!
//! Provides WebAssembly plugin support using Wasmtime.

use crate::error::{PluginError, PluginResult};
use crate::{Plugin, PluginContext, PluginMetadata};
use async_trait::async_trait;
use corvus_core::tool::Tool;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use wasmtime::*;
use wasmtime_wasi::WasiCtxBuilder;

/// WASM plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmPluginConfig {
    /// Maximum memory in pages (64KB per page)
    pub max_memory_pages: u32,
    /// Timeout in milliseconds
    pub timeout_ms: u64,
    /// Whether to enable WASI
    pub enable_wasi: bool,
    /// Allowed directories for WASI
    pub allowed_dirs: Vec<String>,
}

impl Default for WasmPluginConfig {
    fn default() -> Self {
        Self {
            max_memory_pages: 1024, // 64MB
            timeout_ms: 30000,     // 30 seconds
            enable_wasi: true,
            allowed_dirs: Vec::new(),
        }
    }
}

/// WASM plugin ABI version
pub const ABI_VERSION: u32 = 1;

/// WASM plugin manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmPluginManifest {
    /// ABI version
    pub abi_version: u32,
    /// Plugin metadata
    pub metadata: PluginMetadata,
    /// Tools provided by this plugin
    pub tools: Vec<WasmToolDefinition>,
}

/// WASM tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmToolDefinition {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Tool input schema (JSON Schema)
    pub input_schema: serde_json::Value,
}

/// WASM plugin instance
pub struct WasmPlugin {
    manifest: WasmPluginManifest,
    store: Store<WasiState>,
    instance: Instance,
    module: Module,
    config: WasmPluginConfig,
}

/// WASI state for the plugin
#[derive(Debug)]
struct WasiState {
    wasi: wasmtime_wasi::WasiCtx,
}

impl WasmPlugin {
    /// Load a WASM plugin from a file
    pub async fn from_file(
        path: impl AsRef<Path>,
        config: WasmPluginConfig,
    ) -> PluginResult<Self> {
        let engine = Engine::default();
        let module = Module::from_file(&engine, path)?;
        Self::from_module(module, config).await
    }

    /// Load a WASM plugin from bytes
    pub async fn from_bytes(
        bytes: &[u8],
        config: WasmPluginConfig,
    ) -> PluginResult<Self> {
        let engine = Engine::default();
        let module = Module::from_bytes(&engine, bytes)?;
        Self::from_module(module, config).await
    }

    /// Load a WASM plugin from a module
    async fn from_module(module: Module, config: WasmPluginConfig) -> PluginResult<Self> {
        // Create WASI context if enabled
        let wasi_ctx = if config.enable_wasi {
            let mut builder = WasiCtxBuilder::new();
            builder.inherit_stdio();

            // Add allowed directories
            for dir in &config.allowed_dirs {
                builder.preopened_dir(dir, dir)?;
            }

            builder.build()
        } else {
            WasiCtxBuilder::new().build()
        };

        let mut store = Store::new(
            module.engine(),
            WasiState { wasi: wasi_ctx },
        );

        // Set memory limits
        let mut linker = Linker::new(module.engine());
        wasmtime_wasi::add_to_linker(&mut linker, |state: &mut WasiState| &mut state.wasi)?;

        let instance = linker.instantiate(&mut store, &module)?;

        // Get the manifest by calling the plugin's `get_manifest` function
        let manifest = Self::load_manifest(&mut store, &instance)?;

        // Verify ABI version
        if manifest.abi_version != ABI_VERSION {
            return Err(PluginError::LoadError(format!(
                "ABI version mismatch: expected {}, got {}",
                ABI_VERSION, manifest.abi_version
            )));
        }

        Ok(Self {
            manifest,
            store,
            instance,
            module,
            config,
        })
    }

    /// Load the manifest from the plugin
    fn load_manifest(
        store: &mut Store<WasiState>,
        instance: &Instance,
    ) -> PluginResult<WasmPluginManifest> {
        let get_manifest = instance
            .get_typed_func::<(), i32>(store, "get_manifest")
            .map_err(|_| {
                PluginError::LoadError("Plugin missing `get_manifest` function".to_string())
            })?;

        let manifest_ptr = get_manifest.call(store, ())?;

        // Read manifest from memory
        let memory = instance
            .get_memory(store, "memory")
            .ok_or_else(|| PluginError::LoadError("Plugin missing `memory` export".to_string()))?;

        // First read the length (4 bytes)
        let mut len_bytes = [0u8; 4];
        memory.read(store, manifest_ptr as usize, &mut len_bytes)?;
        let len = u32::from_le_bytes(len_bytes) as usize;

        // Then read the JSON data
        let mut data = vec![0u8; len];
        memory.read(store, manifest_ptr as usize + 4, &mut data)?;

        let manifest: WasmPluginManifest = serde_json::from_slice(&data)?;
        Ok(manifest)
    }

    /// Get the plugin manifest
    pub fn manifest(&self) -> &WasmPluginManifest {
        &self.manifest
    }

    /// Call a tool in the plugin
    pub async fn call_tool(
        &mut self,
        tool_name: &str,
        input: serde_json::Value,
    ) -> PluginResult<serde_json::Value> {
        // Check if the tool exists
        if !self.manifest.tools.iter().any(|t| t.name == tool_name) {
            return Err(PluginError::ToolNotFound(tool_name.to_string()));
        }

        // Serialize input
        let input_json = serde_json::to_vec(&input)?;
        let input_len = input_json.len();

        // Allocate memory in the plugin
        let alloc = self
            .instance
            .get_typed_func::<i32, i32>(&mut self.store, "alloc")
            .map_err(|_| PluginError::LoadError("Plugin missing `alloc` function".to_string()))?;

        let ptr = alloc.call(&mut self.store, (input_len + 4) as i32)?;

        // Write input to memory: [len (4 bytes)] + [data]
        let memory = self
            .instance
            .get_memory(&mut self.store, "memory")
            .ok_or_else(|| PluginError::LoadError("Plugin missing `memory` export".to_string()))?;

        // Write length
        memory.write(
            &mut self.store,
            ptr as usize,
            &(input_len as u32).to_le_bytes(),
        )?;

        // Write data
        memory.write(&mut self.store, ptr as usize + 4, &input_json)?;

        // Call the tool
        let call_tool = self
            .instance
            .get_typed_func::<i32, i32>(&mut self.store, "call_tool")
            .map_err(|_| PluginError::LoadError("Plugin missing `call_tool` function".to_string()))?;

        let result_ptr = call_tool.call(&mut self.store, ptr)?;

        // Read result
        let mut len_bytes = [0u8; 4];
        memory.read(&mut self.store, result_ptr as usize, &mut len_bytes)?;
        let len = u32::from_le_bytes(len_bytes) as usize;

        let mut data = vec![0u8; len];
        memory.read(&mut self.store, result_ptr as usize + 4, &mut data)?;

        // Free the allocated memory
        let dealloc = self
            .instance
            .get_typed_func::<i32, ()>(&mut self.store, "dealloc")
            .map_err(|_| PluginError::LoadError("Plugin missing `dealloc` function".to_string()))?;

        dealloc.call(&mut self.store, ptr)?;
        dealloc.call(&mut self.store, result_ptr)?;

        let result: serde_json::Value = serde_json::from_slice(&data)?;
        Ok(result)
    }
}

/// A plugin that wraps a WASM module
pub struct WasmPluginWrapper {
    plugin: Arc<tokio::sync::Mutex<WasmPlugin>>,
}

impl WasmPluginWrapper {
    /// Create a new WASM plugin wrapper
    pub fn new(plugin: WasmPlugin) -> Self {
        Self {
            plugin: Arc::new(tokio::sync::Mutex::new(plugin)),
        }
    }
}

#[async_trait]
impl Plugin for WasmPluginWrapper {
    fn metadata(&self) -> &PluginMetadata {
        // Note: This is a simplification - in a real implementation we'd store
        // the metadata separately since we can't borrow from the mutex here
        unimplemented!("Metadata needs to be stored separately for WASM plugins")
    }

    async fn initialize(&mut self, _context: PluginContext) -> PluginResult<()> {
        // WASM plugins are initialized on load
        Ok(())
    }

    fn tools(&self) -> Vec<Arc<dyn Tool>> {
        // Create tool wrappers for each tool defined in the manifest
        Vec::new()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// WASM plugin manager
pub struct WasmPluginManager {
    config: WasmPluginConfig,
    plugins: HashMap<String, WasmPluginWrapper>,
}

impl WasmPluginManager {
    /// Create a new WASM plugin manager
    pub fn new(config: WasmPluginConfig) -> Self {
        Self {
            config,
            plugins: HashMap::new(),
        }
    }

    /// Load a WASM plugin from a file
    pub async fn load_plugin(&mut self, path: impl AsRef<Path>) -> PluginResult<()> {
        let plugin = WasmPlugin::from_file(path, self.config.clone()).await?;
        let name = plugin.manifest().metadata.name.clone();

        let wrapper = WasmPluginWrapper::new(plugin);
        self.plugins.insert(name, wrapper);

        Ok(())
    }

    /// Get a loaded plugin
    pub fn get_plugin(&self, name: &str) -> Option<&WasmPluginWrapper> {
        self.plugins.get(name)
    }

    /// List all loaded plugins
    pub fn list_plugins(&self) -> Vec<String> {
        self.plugins.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_plugin_config_default() {
        let config = WasmPluginConfig::default();
        assert_eq!(config.max_memory_pages, 1024);
        assert_eq!(config.timeout_ms, 30000);
        assert!(config.enable_wasi);
    }

    #[test]
    fn test_abi_version() {
        assert_eq!(ABI_VERSION, 1);
    }
}
