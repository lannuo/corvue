//! File operations tool for the agent

use async_trait::async_trait;
use corvus_core::error::Result;
use corvus_core::tool::{Tool, ToolDefinition, ToolResult};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::{Path, PathBuf};

/// File operations tool
pub struct FileTool {
    /// Working directory root (all operations are relative to this)
    workdir: PathBuf,
}

impl FileTool {
    /// Create a new file tool with the given working directory
    pub fn new(workdir: impl AsRef<Path>) -> Self {
        Self {
            workdir: workdir.as_ref().to_path_buf(),
        }
    }

    /// Get the full path, resolving relative to workdir
    fn full_path(&self, path: impl AsRef<Path>) -> PathBuf {
        let path = path.as_ref();
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.workdir.join(path)
        }
    }

    /// Read a file
    fn read_file(&self, path: impl AsRef<Path>) -> Result<String> {
        let full_path = self.full_path(path);
        Ok(std::fs::read_to_string(full_path)?)
    }

    /// Write a file
    fn write_file(&self, path: impl AsRef<Path>, content: &str) -> Result<()> {
        let full_path = self.full_path(path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(std::fs::write(full_path, content)?)
    }

    /// List directory contents
    fn list_dir(&self, path: impl AsRef<Path>) -> Result<Vec<String>> {
        let full_path = self.full_path(path);
        let mut entries = Vec::new();

        for entry in std::fs::read_dir(full_path)? {
            let entry = entry?;
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy().to_string();
            entries.push(file_name);
        }

        Ok(entries)
    }

    /// Check if a path exists
    fn exists(&self, path: impl AsRef<Path>) -> bool {
        self.full_path(path).exists()
    }

    /// Get file metadata
    fn file_info(&self, path: impl AsRef<Path>) -> Result<FileInfo> {
        let full_path = self.full_path(path);
        let metadata = std::fs::metadata(full_path)?;

        let modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Ok(FileInfo {
            is_file: metadata.is_file(),
            is_dir: metadata.is_dir(),
            size: metadata.len(),
            modified,
        })
    }

    /// Delete a file or directory
    fn delete(&self, path: impl AsRef<Path>, recursive: bool) -> Result<()> {
        let full_path = self.full_path(path);

        if full_path.is_dir() {
            if recursive {
                std::fs::remove_dir_all(full_path)?;
            } else {
                std::fs::remove_dir(full_path)?;
            }
        } else {
            std::fs::remove_file(full_path)?;
        }

        Ok(())
    }

    /// Create a directory
    fn create_dir(&self, path: impl AsRef<Path>, parents: bool) -> Result<()> {
        let full_path = self.full_path(path);
        if parents {
            std::fs::create_dir_all(full_path)?;
        } else {
            std::fs::create_dir(full_path)?;
        }
        Ok(())
    }
}

/// File metadata info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub is_file: bool,
    pub is_dir: bool,
    pub size: u64,
    pub modified: u64,
}

/// Arguments for file tool
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum FileArgs {
    Read { path: String },
    Write { path: String, content: String },
    List { path: String },
    Exists { path: String },
    Info { path: String },
    Delete { path: String, recursive: Option<bool> },
    CreateDir { path: String, parents: Option<bool> },
}

#[async_trait]
impl Tool for FileTool {
    fn name(&self) -> &str {
        "file_operations"
    }

    fn description(&self) -> &str {
        "Read, write, list, and manipulate files and directories."
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition::new(
            self.name(),
            self.description(),
            json!({
                "type": "object",
                "properties": {
                    "operation": {
                        "type": "string",
                        "description": "The operation to perform",
                        "enum": ["read", "write", "list", "exists", "info", "delete", "create_dir"]
                    },
                    "path": {
                        "type": "string",
                        "description": "The file or directory path"
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write (for write operation)"
                    },
                    "recursive": {
                        "type": "boolean",
                        "description": "Whether to delete recursively (for delete operation)"
                    },
                    "parents": {
                        "type": "boolean",
                        "description": "Whether to create parent directories (for create_dir operation)"
                    }
                },
                "required": ["operation", "path"]
            }),
        )
    }

    async fn call(&self, arguments: serde_json::Value) -> Result<ToolResult> {
        let args: FileArgs = serde_json::from_value(arguments)?;

        let result = match args {
            FileArgs::Read { path } => {
                let content = self.read_file(&path)?;
                json!({
                    "success": true,
                    "content": content
                })
            }
            FileArgs::Write { path, content } => {
                self.write_file(&path, &content)?;
                json!({
                    "success": true,
                    "message": format!("Wrote {} bytes to {}", content.len(), path)
                })
            }
            FileArgs::List { path } => {
                let entries = self.list_dir(&path)?;
                json!({
                    "success": true,
                    "entries": entries
                })
            }
            FileArgs::Exists { path } => {
                let exists = self.exists(&path);
                json!({
                    "success": true,
                    "exists": exists
                })
            }
            FileArgs::Info { path } => {
                let info = self.file_info(&path)?;
                json!({
                    "success": true,
                    "info": info
                })
            }
            FileArgs::Delete { path, recursive } => {
                self.delete(&path, recursive.unwrap_or(false))?;
                json!({
                    "success": true,
                    "message": format!("Deleted {}", path)
                })
            }
            FileArgs::CreateDir { path, parents } => {
                self.create_dir(&path, parents.unwrap_or(true))?;
                json!({
                    "success": true,
                    "message": format!("Created directory {}", path)
                })
            }
        };

        Ok(ToolResult::success("", serde_json::to_string(&result)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_file_tool_creation() {
        let dir = tempdir().unwrap();
        let tool = FileTool::new(dir.path());
        assert_eq!(tool.name(), "file_operations");
    }

    #[test]
    fn test_file_write_read() -> Result<()> {
        let dir = tempdir().unwrap();
        let tool = FileTool::new(dir.path());

        tool.write_file("test.txt", "Hello, World!")?;
        let content = tool.read_file("test.txt")?;
        assert_eq!(content, "Hello, World!");

        Ok(())
    }
}
