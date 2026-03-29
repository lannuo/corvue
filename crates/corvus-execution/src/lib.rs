//! Corvus Execution - Sandboxed code execution
//!
//! Cross-platform sandboxed code execution for Corvus AI Agent.

#![warn(missing_docs)]

pub mod simple;
pub mod sandbox;
pub mod languages;
pub mod security;

pub use simple::SimpleExecutor;
pub use sandbox::{detect_language, ExecutionResult, Language, SandboxConfig, SandboxExecutor};
pub use languages::{
    LanguageDetector, LanguageRuntime, Language as ExtendedLanguage, RuntimeManager,
};
pub use security::{
    Permission, PermissionSet, PathPattern, NetworkPermission, NetworkConfig,
    ResourceLimits, SecurityManager, AuditLogEntry, AuditEventType,
};
