//! Security enhancements for sandboxed code execution
//!
//! Provides fine-grained permission control, resource limits,
//! network access control, and system call filtering.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Permission type for sandbox operations
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    /// File system read access
    FileRead(PathPattern),
    /// File system write access
    FileWrite(PathPattern),
    /// File system delete access
    FileDelete(PathPattern),
    /// File system execute access
    FileExecute(PathPattern),
    /// Network access
    Network(NetworkPermission),
    /// Process spawning
    ProcessSpawn(ProcessPermission),
    /// Environment variable access
    Environment(EnvironmentPermission),
    /// Memory operations
    Memory(MemoryPermission),
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
                let path_str = path.to_string_lossy().to_string();
                if glob == "*" {
                    return true;
                }
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

/// Network permission
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NetworkPermission {
    /// Connect to a specific host
    Connect(HostPattern, PortRange),
    /// Listen on a port
    Listen(u16),
    /// Any network access
    Any,
}

/// Host pattern for network permissions
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HostPattern {
    /// Exact host match
    Exact(String),
    /// Suffix match (domain and subdomains)
    Suffix(String),
    /// IP address range (CIDR)
    Cidr(String),
    /// Any host
    Any,
}

/// Port range for network permissions
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PortRange {
    /// Single port
    Single(u16),
    /// Range of ports (inclusive)
    Range(u16, u16),
    /// Any port
    Any,
}

/// Process permission
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProcessPermission {
    /// Execute a specific command
    Execute(CommandPattern),
    /// Any process
    Any,
}

/// Command pattern for process permissions
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CommandPattern {
    /// Exact command match
    Exact(String),
    /// Any command
    Any,
}

/// Environment variable permission
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EnvironmentPermission {
    /// Read a specific environment variable
    Read(String),
    /// Write a specific environment variable
    Write(String),
    /// Read all environment variables
    ReadAll,
    /// Write all environment variables
    WriteAll,
    /// Any environment access
    Any,
}

/// Memory permission
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryPermission {
    /// Maximum memory in bytes
    Limit(u64),
    /// Grow memory beyond limit
    Grow,
}

/// Permission set for sandbox configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionSet {
    /// Granted permissions
    granted: HashSet<Permission>,
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
            granted: HashSet::new(),
            denied: HashSet::new(),
        }
    }

    /// Create a permission set with minimal (safe) permissions
    pub fn minimal() -> Self {
        let set = Self::new();
        // Minimal is empty - no permissions by default
        set
    }

    /// Create a permission set with no restrictions (use with caution!)
    pub fn unrestricted() -> Self {
        let mut set = Self::new();
        set.grant(Permission::FileRead(PathPattern::Any));
        set.grant(Permission::FileWrite(PathPattern::Any));
        set.grant(Permission::FileDelete(PathPattern::Any));
        set.grant(Permission::FileExecute(PathPattern::Any));
        set.grant(Permission::Network(NetworkPermission::Any));
        set.grant(Permission::ProcessSpawn(ProcessPermission::Any));
        set.grant(Permission::Environment(EnvironmentPermission::Any));
        set
    }

    /// Grant a permission
    pub fn grant(&mut self, permission: Permission) {
        self.denied.remove(&permission);
        self.granted.insert(permission);
    }

    /// Deny a permission (overrides grants)
    pub fn deny(&mut self, permission: Permission) {
        self.granted.remove(&permission);
        self.denied.insert(permission);
    }

    /// Check if a permission is granted
    pub fn has_permission(&self, permission: &Permission) -> bool {
        if self.denied.contains(permission) {
            return false;
        }
        self.granted.contains(permission)
    }

    /// Check if file read is allowed
    pub fn can_read_file(&self, path: &Path) -> bool {
        // Check denied first
        for perm in &self.denied {
            if let Permission::FileRead(pattern) = perm {
                if pattern.matches(path) {
                    return false;
                }
            }
        }

        // Check granted
        for perm in &self.granted {
            if let Permission::FileRead(pattern) = perm {
                if pattern.matches(path) {
                    return true;
                }
            }
        }

        false
    }

    /// Check if file write is allowed
    pub fn can_write_file(&self, path: &Path) -> bool {
        // Check denied first
        for perm in &self.denied {
            if let Permission::FileWrite(pattern) = perm {
                if pattern.matches(path) {
                    return false;
                }
            }
        }

        // Check granted
        for perm in &self.granted {
            if let Permission::FileWrite(pattern) = perm {
                if pattern.matches(path) {
                    return true;
                }
            }
        }

        false
    }

    /// Check if file delete is allowed
    pub fn can_delete_file(&self, path: &Path) -> bool {
        // Check denied first
        for perm in &self.denied {
            if let Permission::FileDelete(pattern) = perm {
                if pattern.matches(path) {
                    return false;
                }
            }
        }

        // Check granted
        for perm in &self.granted {
            if let Permission::FileDelete(pattern) = perm {
                if pattern.matches(path) {
                    return true;
                }
            }
        }

        false
    }

    /// Check if network connect is allowed
    pub fn can_connect(&self, host: &str, port: u16) -> bool {
        // Check denied first
        for perm in &self.denied {
            if let Permission::Network(net_perm) = perm {
                if Self::network_matches(net_perm, host, port) {
                    return false;
                }
            }
        }

        // Check granted
        for perm in &self.granted {
            if let Permission::Network(net_perm) = perm {
                if Self::network_matches(net_perm, host, port) {
                    return true;
                }
            }
        }

        false
    }

    /// Check if a network permission matches
    fn network_matches(perm: &NetworkPermission, host: &str, port: u16) -> bool {
        match perm {
            NetworkPermission::Any => true,
            NetworkPermission::Listen(_) => false,
            NetworkPermission::Connect(host_pat, port_pat) => {
                let host_match = match host_pat {
                    HostPattern::Any => true,
                    HostPattern::Exact(h) => h == host,
                    HostPattern::Suffix(s) => host == s || host.ends_with(&format!(".{}", s)),
                    HostPattern::Cidr(_) => true, // Simplified for now
                };

                let port_match = match port_pat {
                    PortRange::Any => true,
                    PortRange::Single(p) => *p == port,
                    PortRange::Range(start, end) => port >= *start && port <= *end,
                };

                host_match && port_match
            }
        }
    }

    /// Check if process spawn is allowed
    pub fn can_spawn(&self, command: &str) -> bool {
        // Check denied first
        for perm in &self.denied {
            if let Permission::ProcessSpawn(proc_perm) = perm {
                if Self::process_matches(proc_perm, command) {
                    return false;
                }
            }
        }

        // Check granted
        for perm in &self.granted {
            if let Permission::ProcessSpawn(proc_perm) = perm {
                if Self::process_matches(proc_perm, command) {
                    return true;
                }
            }
        }

        false
    }

    /// Check if a process permission matches
    fn process_matches(perm: &ProcessPermission, command: &str) -> bool {
        match perm {
            ProcessPermission::Any => true,
            ProcessPermission::Execute(cmd_pat) => match cmd_pat {
                CommandPattern::Any => true,
                CommandPattern::Exact(cmd) => cmd == command,
            },
        }
    }

    /// Get all granted permissions
    pub fn granted(&self) -> impl Iterator<Item = &Permission> {
        self.granted.iter()
    }

    /// Get all denied permissions
    pub fn denied(&self) -> impl Iterator<Item = &Permission> {
        self.denied.iter()
    }

    /// Merge with another permission set
    pub fn merge(&mut self, other: &PermissionSet) {
        for perm in other.granted.iter() {
            self.grant(perm.clone());
        }
        for perm in other.denied.iter() {
            self.deny(perm.clone());
        }
    }
}

/// Resource limits for sandbox execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum CPU time in seconds
    pub cpu_time: Option<Duration>,
    /// Maximum memory in bytes
    pub memory: Option<u64>,
    /// Maximum disk space in bytes
    pub disk_space: Option<u64>,
    /// Maximum number of open file descriptors
    pub open_files: Option<u32>,
    /// Maximum number of processes
    pub max_processes: Option<u32>,
    /// Maximum file size for created files
    pub max_file_size: Option<u64>,
    /// Maximum number of files that can be created
    pub max_files: Option<usize>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            cpu_time: Some(Duration::from_secs(30)),
            memory: Some(512 * 1024 * 1024), // 512 MB
            disk_space: Some(100 * 1024 * 1024), // 100 MB
            open_files: Some(1024),
            max_processes: Some(10),
            max_file_size: Some(10 * 1024 * 1024), // 10 MB
            max_files: Some(100),
        }
    }
}

/// Network access control configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Whether network access is allowed at all
    pub allow_network: bool,
    /// Allowed hosts and ports
    pub allowed_connections: Vec<(HostPattern, PortRange)>,
    /// Blocked hosts and ports
    pub blocked_connections: Vec<(HostPattern, PortRange)>,
    /// Whether localhost is allowed
    pub allow_localhost: bool,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            allow_network: false,
            allowed_connections: Vec::new(),
            blocked_connections: Vec::new(),
            allow_localhost: false,
        }
    }
}

/// System call filter configuration (Linux only)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyscallFilter {
    /// Allowed system calls
    pub allowed_syscalls: HashSet<String>,
    /// Blocked system calls
    pub blocked_syscalls: HashSet<String>,
    /// Whether to use default safe list
    pub use_default_safe: bool,
}

impl Default for SyscallFilter {
    fn default() -> Self {
        let mut allowed = HashSet::new();
        // Basic syscalls for most programs
        allowed.insert("read".to_string());
        allowed.insert("write".to_string());
        allowed.insert("open".to_string());
        allowed.insert("close".to_string());
        allowed.insert("stat".to_string());
        allowed.insert("fstat".to_string());
        allowed.insert("lseek".to_string());
        allowed.insert("mmap".to_string());
        allowed.insert("munmap".to_string());
        allowed.insert("brk".to_string());
        allowed.insert("exit".to_string());
        allowed.insert("exit_group".to_string());
        allowed.insert("getpid".to_string());
        allowed.insert("getppid".to_string());
        allowed.insert("getuid".to_string());
        allowed.insert("geteuid".to_string());
        allowed.insert("getgid".to_string());
        allowed.insert("getegid".to_string());
        allowed.insert("rt_sigaction".to_string());
        allowed.insert("rt_sigprocmask".to_string());
        allowed.insert("rt_sigreturn".to_string());
        allowed.insert("sigaltstack".to_string());
        allowed.insert("arch_prctl".to_string());
        allowed.insert("access".to_string());
        allowed.insert("faccessat".to_string());
        allowed.insert("readlink".to_string());
        allowed.insert("readlinkat".to_string());

        Self {
            allowed_syscalls: allowed,
            blocked_syscalls: HashSet::new(),
            use_default_safe: true,
        }
    }
}

/// Security audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Event type
    pub event_type: AuditEventType,
    /// Subject (what was accessed)
    pub subject: String,
    /// Action attempted
    pub action: String,
    /// Whether it was allowed
    pub allowed: bool,
    /// Additional details
    pub details: HashMap<String, String>,
}

/// Type of audit event
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditEventType {
    /// File system access
    FileSystem,
    /// Network access
    Network,
    /// Process spawning
    Process,
    /// Environment access
    Environment,
    /// Memory operation
    Memory,
    /// System call
    Syscall,
}

/// Security manager that coordinates all security components
pub struct SecurityManager {
    /// Permission set
    permissions: PermissionSet,
    /// Resource limits
    resource_limits: ResourceLimits,
    /// Network config
    network_config: NetworkConfig,
    /// Syscall filter (Linux only)
    syscall_filter: SyscallFilter,
    /// Audit log
    audit_log: Vec<AuditLogEntry>,
    /// Maximum audit log size
    max_audit_log_size: usize,
}

impl SecurityManager {
    /// Create a new security manager with default settings
    pub fn new() -> Self {
        Self {
            permissions: PermissionSet::minimal(),
            resource_limits: ResourceLimits::default(),
            network_config: NetworkConfig::default(),
            syscall_filter: SyscallFilter::default(),
            audit_log: Vec::new(),
            max_audit_log_size: 1000,
        }
    }

    /// Create with custom permissions
    pub fn with_permissions(permissions: PermissionSet) -> Self {
        Self {
            permissions,
            resource_limits: ResourceLimits::default(),
            network_config: NetworkConfig::default(),
            syscall_filter: SyscallFilter::default(),
            audit_log: Vec::new(),
            max_audit_log_size: 1000,
        }
    }

    /// Get the permission set
    pub fn permissions(&self) -> &PermissionSet {
        &self.permissions
    }

    /// Get mutable permission set
    pub fn permissions_mut(&mut self) -> &mut PermissionSet {
        &mut self.permissions
    }

    /// Get resource limits
    pub fn resource_limits(&self) -> &ResourceLimits {
        &self.resource_limits
    }

    /// Set resource limits
    pub fn set_resource_limits(&mut self, limits: ResourceLimits) {
        self.resource_limits = limits;
    }

    /// Get network config
    pub fn network_config(&self) -> &NetworkConfig {
        &self.network_config
    }

    /// Set network config
    pub fn set_network_config(&mut self, config: NetworkConfig) {
        self.network_config = config;
    }

    /// Check if file read is allowed and log it
    pub fn check_file_read(&mut self, path: &Path) -> bool {
        let allowed = self.permissions.can_read_file(path);
        self.log_audit_event(
            AuditEventType::FileSystem,
            path.to_string_lossy().to_string(),
            "read".to_string(),
            allowed,
        );
        allowed
    }

    /// Check if file write is allowed and log it
    pub fn check_file_write(&mut self, path: &Path) -> bool {
        let allowed = self.permissions.can_write_file(path);
        self.log_audit_event(
            AuditEventType::FileSystem,
            path.to_string_lossy().to_string(),
            "write".to_string(),
            allowed,
        );
        allowed
    }

    /// Check if file delete is allowed and log it
    pub fn check_file_delete(&mut self, path: &Path) -> bool {
        let allowed = self.permissions.can_delete_file(path);
        self.log_audit_event(
            AuditEventType::FileSystem,
            path.to_string_lossy().to_string(),
            "delete".to_string(),
            allowed,
        );
        allowed
    }

    /// Check if network connect is allowed and log it
    pub fn check_network_connect(&mut self, host: &str, port: u16) -> bool {
        let allowed = if !self.network_config.allow_network {
            false
        } else {
            self.permissions.can_connect(host, port)
        };
        self.log_audit_event(
            AuditEventType::Network,
            format!("{}:{}", host, port),
            "connect".to_string(),
            allowed,
        );
        allowed
    }

    /// Check if process spawn is allowed and log it
    pub fn check_process_spawn(&mut self, command: &str) -> bool {
        let allowed = self.permissions.can_spawn(command);
        self.log_audit_event(
            AuditEventType::Process,
            command.to_string(),
            "spawn".to_string(),
            allowed,
        );
        allowed
    }

    /// Log an audit event
    fn log_audit_event(
        &mut self,
        event_type: AuditEventType,
        subject: String,
        action: String,
        allowed: bool,
    ) {
        let entry = AuditLogEntry {
            timestamp: chrono::Utc::now(),
            event_type,
            subject,
            action,
            allowed,
            details: HashMap::new(),
        };

        self.audit_log.push(entry);

        if self.audit_log.len() > self.max_audit_log_size {
            self.audit_log.remove(0);
        }
    }

    /// Get the audit log
    pub fn audit_log(&self) -> &[AuditLogEntry] {
        &self.audit_log
    }

    /// Get blocked actions from audit log
    pub fn blocked_actions(&self) -> Vec<&AuditLogEntry> {
        self.audit_log
            .iter()
            .filter(|e| !e.allowed)
            .collect()
    }

    /// Clear the audit log
    pub fn clear_audit_log(&mut self) {
        self.audit_log.clear();
    }
}

impl Default for SecurityManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_set_creation() {
        let set = PermissionSet::new();
        assert!(set.granted.is_empty());
        assert!(set.denied.is_empty());
    }

    #[test]
    fn test_permission_grant() {
        let mut set = PermissionSet::new();
        let perm = Permission::FileRead(PathPattern::Any);
        set.grant(perm.clone());
        assert!(set.has_permission(&perm));
    }

    #[test]
    fn test_permission_deny() {
        let mut set = PermissionSet::new();
        let perm = Permission::FileRead(PathPattern::Any);
        set.grant(perm.clone());
        set.deny(perm.clone());
        assert!(!set.has_permission(&perm));
    }

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
    fn test_security_manager_creation() {
        let manager = SecurityManager::new();
        assert_eq!(manager.audit_log().len(), 0);
    }

    #[test]
    fn test_resource_limits_default() {
        let limits = ResourceLimits::default();
        assert!(limits.cpu_time.is_some());
        assert!(limits.memory.is_some());
    }

    #[test]
    fn test_network_config_default() {
        let config = NetworkConfig::default();
        assert!(!config.allow_network);
    }

    #[test]
    fn test_minimal_permissions() {
        let set = PermissionSet::minimal();
        // Minimal is empty for maximum security
        assert!(set.granted.is_empty());
    }

    #[test]
    fn test_audit_logging() {
        let mut manager = SecurityManager::new();

        // This should be denied by default
        let allowed = manager.check_file_read(Path::new("/etc/passwd"));
        assert!(!allowed);

        // Should have an audit log entry
        assert_eq!(manager.audit_log().len(), 1);
    }

    #[test]
    fn test_blocked_actions() {
        let mut manager = SecurityManager::new();
        manager.check_file_read(Path::new("/etc/passwd"));

        let blocked = manager.blocked_actions();
        assert_eq!(blocked.len(), 1);
    }
}
