//! System information tool for Corvus

use async_trait::async_trait;
use corvus_core::error::Result;
use corvus_core::tool::{Tool, ToolDefinition, ToolResult};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// System information arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemArgs {
    /// Information to retrieve (all, os, cpu, memory, disk, network)
    #[serde(default = "default_info_type")]
    pub info: String,
}

fn default_info_type() -> String {
    "all".to_string()
}

/// System information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// Operating system
    pub os: Option<OsInfo>,
    /// CPU information
    pub cpu: Option<CpuInfo>,
}

/// OS information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsInfo {
    /// OS name
    pub name: String,
    /// Architecture
    pub arch: String,
}

/// CPU information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
    /// Number of cores
    pub cores: usize,
}

/// System information tool
pub struct SystemTool;

impl SystemTool {
    /// Create a new system tool
    pub fn new() -> Self {
        Self
    }

    /// Get OS information
    fn get_os_info(&self) -> Option<OsInfo> {
        Some(OsInfo {
            name: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
        })
    }

    /// Get CPU information
    fn get_cpu_info(&self) -> Option<CpuInfo> {
        let num_cpus = num_cpus::get();
        Some(CpuInfo {
            cores: num_cpus,
        })
    }
}

impl Default for SystemTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for SystemTool {
    fn name(&self) -> &str {
        "system_info"
    }

    fn description(&self) -> &str {
        "Get system information (OS, CPU)"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition::new(
            self.name(),
            self.description(),
            json!({
                "type": "object",
                "properties": {
                    "info": {
                        "type": "string",
                        "description": "Type of information to retrieve",
                        "enum": ["all", "os", "cpu"],
                        "default": "all"
                    }
                }
            }),
        )
    }

    async fn call(&self, arguments: serde_json::Value) -> Result<ToolResult> {
        let args: SystemArgs = serde_json::from_value(arguments)?;

        let mut info = SystemInfo {
            os: None,
            cpu: None,
        };

        match args.info.as_str() {
            "all" => {
                info.os = self.get_os_info();
                info.cpu = self.get_cpu_info();
            }
            "os" => {
                info.os = self.get_os_info();
            }
            "cpu" => {
                info.cpu = self.get_cpu_info();
            }
            _ => {
                return Ok(ToolResult::error(
                    "",
                    format!("Invalid info type: {}", args.info),
                ));
            }
        }

        let result = json!({
            "success": true,
            "info": info
        });

        Ok(ToolResult::success("", serde_json::to_string(&result)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_tool_creation() {
        let tool = SystemTool::new();
        assert_eq!(tool.name(), "system_info");
    }
}
