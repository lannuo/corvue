//! File search tool for Corvus
//!
//! Provides recursive file search functionality with pattern matching.

use async_trait::async_trait;
use corvus_core::error::Result;
use corvus_core::tool::{Tool, ToolDefinition, ToolResult};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::{Path, PathBuf};

/// Arguments for the search tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchArgs {
    /// Directory to search in (default: current directory)
    #[serde(default)]
    pub directory: Option<String>,
    /// Pattern to search for (glob pattern)
    pub pattern: String,
    /// File patterns to include (glob patterns, e.g., "*.rs")
    #[serde(default)]
    pub include: Vec<String>,
    /// File patterns to exclude (glob patterns)
    #[serde(default)]
    pub exclude: Vec<String>,
    /// Maximum depth to search (default: 10)
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
}

fn default_max_depth() -> usize {
    10
}

/// File search tool
pub struct SearchTool {
    /// Base directory for search operations
    base_dir: PathBuf,
}

impl SearchTool {
    /// Create a new search tool with the given base directory
    pub fn new(base_dir: impl AsRef<Path>) -> Self {
        Self {
            base_dir: base_dir.as_ref().to_path_buf(),
        }
    }

    /// Simple recursive file search
    fn search_files_sync(&self, args: &SearchArgs) -> Result<Vec<String>> {
        let search_dir = if let Some(dir) = &args.directory {
            self.base_dir.join(dir)
        } else {
            self.base_dir.clone()
        };

        if !search_dir.exists() {
            return Ok(vec![]);
        }

        let mut results = Vec::new();
        self.walk_directory(&search_dir, args, 0, &mut results)?;

        Ok(results)
    }

    /// Walk directory recursively
    fn walk_directory(
        &self,
        dir: &Path,
        args: &SearchArgs,
        depth: usize,
        results: &mut Vec<String>,
    ) -> Result<()> {
        if depth > args.max_depth {
            return Ok(());
        }

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    // Skip hidden directories
                    if !dir_name.starts_with('.') {
                        self.walk_directory(&path, args, depth + 1, results)?;
                    }
                } else if path.is_file() {
                    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                    // Check exclude patterns
                    let excluded = args.exclude.iter().any(|pat| glob_match(pat, file_name));
                    if excluded {
                        continue;
                    }

                    // Check include patterns
                    let include = args.include.is_empty()
                        || args.include.iter().any(|pat| glob_match(pat, file_name));

                    // Check pattern match
                    let matches = glob_match(&args.pattern, file_name);

                    if include && matches {
                        if let Ok(relative) = path.strip_prefix(&self.base_dir) {
                            results.push(relative.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

/// Simple glob pattern matching (supports * and ?)
fn glob_match(pattern: &str, text: &str) -> bool {
    let mut pattern_chars = pattern.chars().peekable();
    let mut text_chars = text.chars().peekable();

    while let Some(p) = pattern_chars.next() {
        match p {
            '*' => {
                // Match any sequence of characters
                if pattern_chars.peek().is_none() {
                    // * at end matches everything
                    return true;
                }
                // Try to match the rest of the pattern somewhere in the text
                let remaining_pattern: String = pattern_chars.collect();
                let text_str: String = text_chars.collect();
                for i in 0..=text_str.len() {
                    if glob_match(&remaining_pattern, &text_str[i..]) {
                        return true;
                    }
                }
                return false;
            }
            '?' => {
                // Match any single character
                if text_chars.next().is_none() {
                    return false;
                }
            }
            _ => {
                // Exact character match
                if let Some(t) = text_chars.next() {
                    if p != t {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        }
    }

    // Both should be exhausted
    text_chars.peek().is_none()
}

#[async_trait]
impl Tool for SearchTool {
    fn name(&self) -> &str {
        "file_search"
    }

    fn description(&self) -> &str {
        "Search for files in the directory structure"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition::new(
            self.name(),
            self.description(),
            json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Pattern to search for (supports * and ? wildcards)"
                    },
                    "directory": {
                        "type": "string",
                        "description": "Directory to search in (default: current directory)"
                    },
                    "include": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "File patterns to include (e.g., [\"*.rs\", \"*.md\"])"
                    },
                    "exclude": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "File patterns to exclude"
                    },
                    "max_depth": {
                        "type": "number",
                        "description": "Maximum directory depth to search (default: 10)"
                    }
                },
                "required": ["pattern"]
            }),
        )
    }

    async fn call(&self, arguments: serde_json::Value) -> Result<ToolResult> {
        let args: SearchArgs = serde_json::from_value(arguments)?;

        let results = self.search_files_sync(&args)?;

        let content = if results.is_empty() {
            json!({
                "success": true,
                "message": "No matching files found.",
                "files": []
            })
        } else {
            json!({
                "success": true,
                "message": format!("Found {} matching files", results.len()),
                "files": results
            })
        };

        Ok(ToolResult::success("", serde_json::to_string(&content)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_match() {
        assert!(glob_match("*.rs", "test.rs"));
        assert!(glob_match("test.*", "test.rs"));
        assert!(glob_match("t?st.rs", "test.rs"));
        assert!(!glob_match("*.rs", "test.txt"));
        assert!(glob_match("*test*", "my_test_file.rs"));
    }

    #[test]
    fn test_search_tool_creation() {
        let tool = SearchTool::new(".");
        assert_eq!(tool.name(), "file_search");
    }
}
