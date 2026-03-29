//! Plugin Permission System
//!
//! Provides fine-grained permission control for plugins.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Permission identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    /// File system operations
    FileSystem(FileSystemPermission),
    /// Network operations
    Network(NetworkPermission),
    /// Process execution
    Process(ProcessPermission),
    /// Environment access
    Environment(EnvironmentPermission),
    /// Memory operations
    Memory(MemoryPermission),
    /// Custom permission
    Custom(String),
}

/// File system permission
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FileSystemPermission {
    /// Read files (path pattern)
    Read(PathPattern),
    /// Write files (path pattern)
    Write(PathPattern),
    /// Delete files (path pattern)
    Delete(PathPattern),
    /// List directories (path pattern)
    List(PathPattern),
    /// Create directories (path pattern)
    CreateDir(PathPattern),
}

/// Network permission
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NetworkPermission {
    /// Connect to a host (host pattern)
    Connect(HostPattern),
    /// Listen on a port
    Listen(u16),
    /// Make HTTP requests
    HttpRequest(HttpMethod, UrlPattern),
}

/// HTTP method
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    HEAD,
    OPTIONS,
    Any,
}

/// Process permission
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProcessPermission {
    /// Execute a command (command pattern)
    Execute(CommandPattern),
    /// Spawn child processes
    Spawn,
}

/// Environment permission
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EnvironmentPermission {
    /// Read an environment variable
    Read(String),
    /// Write an environment variable
    Write(String),
    /// Read all environment variables
    ReadAll,
}

/// Memory permission
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryPermission {
    /// Maximum memory in bytes
    Limit(u64),
    /// Grow memory
    Grow,
}

/// Path pattern for file permissions
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PathPattern {
    /// Exact path match
    Exact(PathBuf),
    /// Prefix match (all children)
    Prefix(PathBuf),
    /// Glob pattern
    Glob(String),
    /// Any path
    Any,
}

impl PathPattern {
    /// Check if a path matches this pattern
    pub fn matches(&self, path: &Path) -> bool {
        match self {
            PathPattern::Exact(pattern) => pattern == path,
            PathPattern::Prefix(pattern) => path.starts_with(pattern),
            PathPattern::Glob(glob) => {
                // Simple glob matching for now
                if glob == "*" {
                    return true;
                }
                let path_str = path.to_string_lossy().to_string();
                if let Some(pattern) = glob.strip_suffix('*') {
                    return path_str.starts_with(pattern);
                }
                if let Some(pattern) = glob.strip_prefix('*') {
                    return path_str.ends_with(pattern);
                }
                path_str == glob.as_str()
            }
            PathPattern::Any => true,
        }
    }
}

/// Host pattern for network permissions
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HostPattern {
    /// Exact host match
    Exact(String),
    /// Suffix match (domain and subdomains)
    Suffix(String),
    /// Any host
    Any,
}

impl HostPattern {
    /// Check if a host matches this pattern
    pub fn matches(&self, host: &str) -> bool {
        match self {
            HostPattern::Exact(pattern) => pattern == host,
            HostPattern::Suffix(pattern) => {
                host == pattern || host.ends_with(&format!(".{}", pattern))
            }
            HostPattern::Any => true,
        }
    }
}

/// URL pattern for HTTP permissions
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UrlPattern {
    /// Exact URL match
    Exact(String),
    /// Prefix match
    Prefix(String),
    /// Any URL
    Any,
}

impl UrlPattern {
    /// Check if a URL matches this pattern
    pub fn matches(&self, url: &str) -> bool {
        match self {
            UrlPattern::Exact(pattern) => pattern == url,
            UrlPattern::Prefix(pattern) => url.starts_with(pattern),
            UrlPattern::Any => true,
        }
    }
}

/// Command pattern for process permissions
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CommandPattern {
    /// Exact command match
    Exact(String),
    /// Any command
    Any,
}

impl CommandPattern {
    /// Check if a command matches this pattern
    pub fn matches(&self, command: &str) -> bool {
        match self {
            CommandPattern::Exact(pattern) => pattern == command,
            CommandPattern::Any => true,
        }
    }
}

/// Permission set for a plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionSet {
    /// Granted permissions
    permissions: HashSet<Permission>,
    /// Denied permissions (override granted)
    denied: HashSet<Permission>,
}

impl Default for PermissionSet {
    fn default() -> Self {
        Self::new()
    }
}

impl PermissionSet {
    /// Create a new empty permission set
    pub fn new() -> Self {
        Self {
            permissions: HashSet::new(),
            denied: HashSet::new(),
        }
    }

    /// Create a permission set with all permissions (use with caution!)
    pub fn all() -> Self {
        // This is a simplification - in practice we'd define what "all" means
        Self::new()
    }

    /// Grant a permission
    pub fn grant(&mut self, permission: Permission) {
        self.denied.remove(&permission);
        self.permissions.insert(permission);
    }

    /// Deny a permission (overrides grants)
    pub fn deny(&mut self, permission: Permission) {
        self.permissions.remove(&permission);
        self.denied.insert(permission);
    }

    /// Check if a permission is granted
    pub fn has_permission(&self, permission: &Permission) -> bool {
        if self.denied.contains(permission) {
            return false;
        }
        self.permissions.contains(permission)
    }

    /// Get all granted permissions
    pub fn granted(&self) -> impl Iterator<Item = &Permission> {
        self.permissions.iter()
    }

    /// Get all denied permissions
    pub fn denied(&self) -> impl Iterator<Item = &Permission> {
        self.denied.iter()
    }

    /// Merge with another permission set
    pub fn merge(&mut self, other: &PermissionSet) {
        for perm in other.permissions.iter() {
            self.grant(perm.clone());
        }
        for perm in other.denied.iter() {
            self.deny(perm.clone());
        }
    }
}

/// Permission check result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionCheck {
    /// Permission granted
    Granted,
    /// Permission denied
    Denied(Permission),
    /// Permission not specified (treated as denied)
    NotSpecified(Permission),
}

/// Permission manager for all plugins
pub struct PermissionManager {
    /// Default permissions for new plugins
    default_permissions: PermissionSet,
    /// Per-plugin permissions
    plugin_permissions: HashMap<String, PermissionSet>,
}

impl PermissionManager {
    /// Create a new permission manager with empty defaults
    pub fn new() -> Self {
        Self {
            default_permissions: PermissionSet::new(),
            plugin_permissions: HashMap::new(),
        }
    }

    /// Create a permission manager with default permissions
    pub fn with_defaults(defaults: PermissionSet) -> Self {
        Self {
            default_permissions: defaults,
            plugin_permissions: HashMap::new(),
        }
    }

    /// Set permissions for a plugin
    pub fn set_plugin_permissions(&mut self, plugin_name: &str, permissions: PermissionSet) {
        self.plugin_permissions
            .insert(plugin_name.to_string(), permissions);
    }

    /// Get permissions for a plugin (combines with defaults)
    pub fn get_plugin_permissions(&self, plugin_name: &str) -> PermissionSet {
        let mut permissions = self.default_permissions.clone();
        if let Some(plugin_perms) = self.plugin_permissions.get(plugin_name) {
            permissions.merge(plugin_perms);
        }
        permissions
    }

    /// Check if a plugin has a permission
    pub fn check_permission(&self, plugin_name: &str, permission: &Permission) -> PermissionCheck {
        let permissions = self.get_plugin_permissions(plugin_name);

        if permissions.denied().any(|p| p == permission) {
            return PermissionCheck::Denied(permission.clone());
        }

        if permissions.has_permission(permission) {
            return PermissionCheck::Granted;
        }

        PermissionCheck::NotSpecified(permission.clone())
    }

    /// Grant a permission to a plugin
    pub fn grant(&mut self, plugin_name: &str, permission: Permission) {
        let permissions = self
            .plugin_permissions
            .entry(plugin_name.to_string())
            .or_insert_with(PermissionSet::new);
        permissions.grant(permission);
    }

    /// Deny a permission for a plugin
    pub fn deny(&mut self, plugin_name: &str, permission: Permission) {
        let permissions = self
            .plugin_permissions
            .entry(plugin_name.to_string())
            .or_insert_with(PermissionSet::new);
        permissions.deny(permission);
    }

    /// Set the default permissions
    pub fn set_defaults(&mut self, defaults: PermissionSet) {
        self.default_permissions = defaults;
    }
}

impl Default for PermissionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Common permission presets
pub mod presets {
    use super::*;

    /// Minimal permissions - no access
    pub fn minimal() -> PermissionSet {
        PermissionSet::new()
    }

    /// Read-only file access to a directory
    pub fn read_only(dir: impl Into<PathBuf>) -> PermissionSet {
        let mut perms = PermissionSet::new();
        let path = PathPattern::Prefix(dir.into());
        perms.grant(Permission::FileSystem(FileSystemPermission::Read(
            path.clone(),
        )));
        perms.grant(Permission::FileSystem(FileSystemPermission::List(path)));
        perms
    }

    /// Read-write access to a directory
    pub fn read_write(dir: impl Into<PathBuf>) -> PermissionSet {
        let path = PathPattern::Prefix(dir.into());
        let mut perms = PermissionSet::new();
        perms.grant(Permission::FileSystem(FileSystemPermission::Read(path.clone())));
        perms.grant(Permission::FileSystem(FileSystemPermission::List(path.clone())));
        perms.grant(Permission::FileSystem(FileSystemPermission::Write(path.clone())));
        perms.grant(Permission::FileSystem(FileSystemPermission::Delete(path.clone())));
        perms.grant(Permission::FileSystem(FileSystemPermission::CreateDir(path)));
        perms
    }

    /// Network access to specific hosts
    pub fn network(hosts: Vec<HostPattern>) -> PermissionSet {
        let mut perms = PermissionSet::new();
        for host in hosts {
            perms.grant(Permission::Network(NetworkPermission::Connect(host)));
        }
        perms
    }

    /// Full access (use with caution!)
    pub fn full_access() -> PermissionSet {
        let mut perms = PermissionSet::new();
        perms.grant(Permission::FileSystem(FileSystemPermission::Read(PathPattern::Any)));
        perms.grant(Permission::FileSystem(FileSystemPermission::Write(PathPattern::Any)));
        perms.grant(Permission::FileSystem(FileSystemPermission::Delete(PathPattern::Any)));
        perms.grant(Permission::FileSystem(FileSystemPermission::List(PathPattern::Any)));
        perms.grant(Permission::FileSystem(FileSystemPermission::CreateDir(PathPattern::Any)));
        perms.grant(Permission::Network(NetworkPermission::Connect(HostPattern::Any)));
        perms.grant(Permission::Process(ProcessPermission::Execute(CommandPattern::Any)));
        perms.grant(Permission::Environment(EnvironmentPermission::ReadAll));
        perms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_pattern_exact() {
        let pattern = PathPattern::Exact(PathBuf::from("/tmp/test.txt"));
        assert!(pattern.matches(Path::new("/tmp/test.txt")));
        assert!(!pattern.matches(Path::new("/tmp/other.txt")));
    }

    #[test]
    fn test_path_pattern_prefix() {
        let pattern = PathPattern::Prefix(PathBuf::from("/tmp"));
        assert!(pattern.matches(Path::new("/tmp/test.txt")));
        assert!(pattern.matches(Path::new("/tmp/subdir/file.txt")));
        assert!(!pattern.matches(Path::new("/home/user/file.txt")));
    }

    #[test]
    fn test_permission_set_grant() {
        let mut perms = PermissionSet::new();
        let perm = Permission::FileSystem(FileSystemPermission::Read(PathPattern::Any));
        perms.grant(perm.clone());
        assert!(perms.has_permission(&perm));
    }

    #[test]
    fn test_permission_set_deny() {
        let mut perms = PermissionSet::new();
        let perm = Permission::FileSystem(FileSystemPermission::Read(PathPattern::Any));
        perms.grant(perm.clone());
        perms.deny(perm.clone());
        assert!(!perms.has_permission(&perm));
    }

    #[test]
    fn test_permission_manager() {
        let mut manager = PermissionManager::new();
        let perm = Permission::FileSystem(FileSystemPermission::Read(PathPattern::Any));
        manager.grant("test-plugin", perm.clone());
        assert_eq!(
            manager.check_permission("test-plugin", &perm),
            PermissionCheck::Granted
        );
    }
}
