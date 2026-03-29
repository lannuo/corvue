//! Filesystem MCP Server
//!
//! Provides file system operations as an MCP server.

use crate::error::{ProtocolError, Result};
use crate::mcp::framework::*;
use crate::mcp::protocol::*;
use serde_json::json;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Filesystem server configuration
#[derive(Debug, Clone)]
pub struct FilesystemConfig {
    /// Root directories that are accessible
    pub roots: Vec<PathBuf>,
    /// Whether write operations are allowed
    pub allow_write: bool,
    /// Whether delete operations are allowed
    pub allow_delete: bool,
    /// Maximum file size to read/write (bytes)
    pub max_file_size: u64,
}

impl Default for FilesystemConfig {
    fn default() -> Self {
        Self {
            roots: vec![],
            allow_write: false,
            allow_delete: false,
            max_file_size: 10 * 1024 * 1024, // 10 MB
        }
    }
}

/// Filesystem MCP server
pub struct FilesystemServer {
    config: FilesystemConfig,
}

impl FilesystemServer {
    /// Create a new filesystem server
    pub fn new(config: FilesystemConfig) -> Self {
        Self {
            config,
        }
    }

    /// Create with default config and single root
    pub fn with_root(root: PathBuf) -> Self {
        let config = FilesystemConfig {
            roots: vec![root],
            ..FilesystemConfig::default()
        };
        Self::new(config)
    }

    /// Allow write operations
    pub fn allow_write(mut self) -> Self {
        self.config.allow_write = true;
        self
    }

    /// Allow delete operations
    pub fn allow_delete(mut self) -> Self {
        self.config.allow_delete = true;
        self
    }

    /// Build the MCP server
    pub fn build(self) -> SimpleMcpServer {
        let mut builder = McpServerBuilder::new(
            "corvus-filesystem".to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
        )
        .with_instructions(
            "Filesystem server for reading and writing files. \
             Use the read_file tool to read files, write_file to write files, \
             and list_directory to explore directories."
                .to_string(),
        );

        // Read file tool
        let read_file_tool = Tool {
            name: "read_file".to_string(),
            description: "Read the contents of a file".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to read"
                    }
                },
                "required": ["path"]
            }),
        };

        let config_clone = self.config.clone();
        builder = builder.register_tool(read_file_tool, move |args| {
            let path_str = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ProtocolError::InvalidParams("Missing path".to_string()))?;

            let path = Path::new(path_str);
            let validated = Self::validate_path_static(&config_clone, path)?;

            let metadata = std::fs::metadata(&validated)
                .map_err(|e| ProtocolError::Protocol(format!("File not found: {}", e)))?;

            if metadata.len() > config_clone.max_file_size {
                return Ok(tool_response_error(format!(
                    "File too large: {} bytes (max: {} bytes)",
                    metadata.len(),
                    config_clone.max_file_size
                )));
            }

            let content = std::fs::read_to_string(&validated)
                .map_err(|e| ProtocolError::Protocol(format!("Failed to read file: {}", e)))?;

            Ok(tool_response_text(content))
        });

        // List directory tool
        let list_dir_tool = Tool {
            name: "list_directory".to_string(),
            description: "List the contents of a directory".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the directory to list"
                    },
                    "recursive": {
                        "type": "boolean",
                        "description": "Whether to list recursively",
                        "default": false
                    }
                },
                "required": ["path"]
            }),
        };

        let config_clone = self.config.clone();
        builder = builder.register_tool(list_dir_tool, move |args| {
            let path_str = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ProtocolError::InvalidParams("Missing path".to_string()))?;

            let recursive = args.get("recursive").and_then(|v| v.as_bool()).unwrap_or(false);

            let path = Path::new(path_str);
            let validated = Self::validate_path_static(&config_clone, path)?;

            let mut entries = Vec::new();

            if recursive {
                for entry in WalkDir::new(&validated) {
                    let entry = entry
                        .map_err(|e| ProtocolError::Protocol(format!("Failed to list directory: {}", e)))?;
                    let relative = entry
                        .path()
                        .strip_prefix(&validated)
                        .unwrap_or(entry.path());
                    entries.push(format!(
                        "{}{}",
                        relative.display(),
                        if entry.file_type().is_dir() { "/" } else { "" }
                    ));
                }
            } else {
                for entry in std::fs::read_dir(&validated)
                    .map_err(|e| ProtocolError::Protocol(format!("Failed to list directory: {}", e)))?
                {
                    let entry = entry
                        .map_err(|e| ProtocolError::Protocol(format!("Failed to list directory: {}", e)))?;
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    let is_dir = entry
                        .file_type()
                        .map(|ft| ft.is_dir())
                        .unwrap_or(false);
                    entries.push(format!("{}{}", file_name, if is_dir { "/" } else { "" }));
                }
            }

            entries.sort();
            Ok(tool_response_text(entries.join("\n")))
        });

        // Write file tool (if allowed)
        if self.config.allow_write {
            let write_file_tool = Tool {
                name: "write_file".to_string(),
                description: "Write content to a file".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the file to write"
                        },
                        "content": {
                            "type": "string",
                            "description": "Content to write to the file"
                        }
                    },
                    "required": ["path", "content"]
                }),
            };

            let config_clone = self.config.clone();
            builder = builder.register_tool(write_file_tool, move |args| {
                let path_str = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ProtocolError::InvalidParams("Missing path".to_string()))?;

                let content = args
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ProtocolError::InvalidParams("Missing content".to_string()))?;

                if content.len() as u64 > config_clone.max_file_size {
                    return Ok(tool_response_error(format!(
                        "Content too large: {} bytes (max: {} bytes)",
                        content.len(),
                        config_clone.max_file_size
                    )));
                }

                let path = Path::new(path_str);
                let validated = Self::validate_path_static(&config_clone, path)?;

                // Create parent directory if needed
                if let Some(parent) = validated.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| ProtocolError::Protocol(format!("Failed to create directory: {}", e)))?;
                }

                std::fs::write(&validated, content)
                    .map_err(|e| ProtocolError::Protocol(format!("Failed to write file: {}", e)))?;

                Ok(tool_response_text(format!("Wrote file: {}", validated.display())))
            });
        }

        // Delete file tool (if allowed)
        if self.config.allow_delete {
            let delete_file_tool = Tool {
                name: "delete_file".to_string(),
                description: "Delete a file".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the file to delete"
                        }
                    },
                    "required": ["path"]
                }),
            };

            let config_clone = self.config.clone();
            builder = builder.register_tool(delete_file_tool, move |args| {
                let path_str = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ProtocolError::InvalidParams("Missing path".to_string()))?;

                let path = Path::new(path_str);
                let validated = Self::validate_path_static(&config_clone, path)?;

                std::fs::remove_file(&validated)
                    .map_err(|e| ProtocolError::Protocol(format!("Failed to delete file: {}", e)))?;

                Ok(tool_response_text(format!("Deleted file: {}", validated.display())))
            });
        }

        builder.build()
    }

    /// Static version of path validation for use in closures
    fn validate_path_static(config: &FilesystemConfig, path: &Path) -> Result<PathBuf> {
        let canonical = path
            .canonicalize()
            .map_err(|e| ProtocolError::Protocol(format!("Invalid path: {}", e)))?;

        for root in &config.roots {
            let canonical_root = match root.canonicalize() {
                Ok(p) => p,
                Err(_) => continue,
            };

            if canonical.starts_with(&canonical_root) {
                return Ok(canonical);
            }
        }

        Err(ProtocolError::Protocol(format!(
            "Path not in allowed roots: {}",
            path.display()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_config_default() {
        let config = FilesystemConfig::default();
        assert!(!config.allow_write);
        assert!(!config.allow_delete);
    }

    #[test]
    fn test_server_creation() {
        let dir = tempdir().unwrap();
        let server = FilesystemServer::with_root(dir.path().to_path_buf());
        assert_eq!(server.config.roots.len(), 1);
    }

    #[test]
    fn test_allow_write() {
        let dir = tempdir().unwrap();
        let server = FilesystemServer::with_root(dir.path().to_path_buf()).allow_write();
        assert!(server.config.allow_write);
    }

    #[test]
    fn test_allow_delete() {
        let dir = tempdir().unwrap();
        let server = FilesystemServer::with_root(dir.path().to_path_buf()).allow_delete();
        assert!(server.config.allow_delete);
    }

    #[tokio::test]
    async fn test_build_server() {
        let dir = tempdir().unwrap();
        let server = FilesystemServer::with_root(dir.path().to_path_buf()).build();

        // Should have at least the read_file and list_directory tools
        let tools = server.tools();
        assert!(tools.len() >= 2);
        assert!(tools.iter().any(|t| t.name == "read_file"));
        assert!(tools.iter().any(|t| t.name == "list_directory"));
    }

    #[tokio::test]
    async fn test_build_with_write() {
        let dir = tempdir().unwrap();
        let server = FilesystemServer::with_root(dir.path().to_path_buf())
            .allow_write()
            .build();

        let tools = server.tools();
        assert!(tools.iter().any(|t| t.name == "write_file"));
    }

    #[tokio::test]
    async fn test_build_with_delete() {
        let dir = tempdir().unwrap();
        let server = FilesystemServer::with_root(dir.path().to_path_buf())
            .allow_delete()
            .build();

        let tools = server.tools();
        assert!(tools.iter().any(|t| t.name == "delete_file"));
    }
}
