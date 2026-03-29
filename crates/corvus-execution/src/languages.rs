//! Multi-language support for code execution
//!
//! Provides enhanced language detection, runtime management,
//! and language-specific execution strategies.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Extended programming language support
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    /// Python
    Python,
    /// JavaScript (Node.js)
    JavaScript,
    /// TypeScript (Node.js + ts-node)
    TypeScript,
    /// Rust
    Rust,
    /// Shell script
    Shell,
    /// Go
    Go,
    /// C
    C,
    /// C++
    Cpp,
    /// Java
    Java,
    /// Ruby
    Ruby,
    /// PHP
    Php,
    /// Julia
    Julia,
    /// R
    R,
    /// Perl
    Perl,
    /// Swift
    Swift,
    /// Kotlin
    Kotlin,
    /// C#
    CSharp,
    /// Lua
    Lua,
    /// Dart
    Dart,
}

/// Language runtime information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageRuntime {
    /// The language
    pub language: Language,
    /// Whether the runtime is available
    pub available: bool,
    /// Detected version (if available)
    pub version: Option<String>,
    /// Path to the interpreter/compiler
    pub executable_path: Option<PathBuf>,
    /// Required dependencies
    pub required_dependencies: Vec<String>,
}

/// Enhanced language detector
pub struct LanguageDetector {
    /// Language-specific patterns
    patterns: HashMap<Language, Vec<String>>,
    /// Shebang patterns
    shebang_patterns: HashMap<Language, Vec<String>>,
}

impl Default for LanguageDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageDetector {
    /// Create a new language detector
    pub fn new() -> Self {
        let mut patterns = HashMap::new();
        let mut shebang_patterns = HashMap::new();

        // Python
        patterns.insert(
            Language::Python,
            vec![
                "def ".to_string(),
                "import ".to_string(),
                "from ".to_string(),
                "print(".to_string(),
                "if __name__".to_string(),
            ],
        );
        shebang_patterns.insert(
            Language::Python,
            vec!["python".to_string(), "python3".to_string()],
        );

        // JavaScript
        patterns.insert(
            Language::JavaScript,
            vec![
                "function ".to_string(),
                "const ".to_string(),
                "let ".to_string(),
                "var ".to_string(),
                "console.log".to_string(),
                "=>".to_string(),
            ],
        );
        shebang_patterns.insert(
            Language::JavaScript,
            vec!["node".to_string()],
        );

        // TypeScript
        patterns.insert(
            Language::TypeScript,
            vec![
                "interface ".to_string(),
                "type ".to_string(),
                ": string".to_string(),
                ": number".to_string(),
                ": boolean".to_string(),
            ],
        );
        shebang_patterns.insert(
            Language::TypeScript,
            vec!["ts-node".to_string()],
        );

        // Rust
        patterns.insert(
            Language::Rust,
            vec![
                "fn main()".to_string(),
                "fn ".to_string(),
                "use ".to_string(),
                "impl ".to_string(),
                "pub fn".to_string(),
                "println!".to_string(),
            ],
        );
        shebang_patterns.insert(
            Language::Rust,
            vec!["rust".to_string()],
        );

        // Shell
        patterns.insert(
            Language::Shell,
            vec![
                "#!/bin".to_string(),
                "echo ".to_string(),
                "export ".to_string(),
                "if [".to_string(),
                "for ".to_string(),
                "do ".to_string(),
                "done".to_string(),
            ],
        );
        shebang_patterns.insert(
            Language::Shell,
            vec!["bash".to_string(), "sh".to_string(), "zsh".to_string()],
        );

        // Go
        patterns.insert(
            Language::Go,
            vec![
                "package main".to_string(),
                "func main()".to_string(),
                "import \"".to_string(),
                "fmt.".to_string(),
            ],
        );
        shebang_patterns.insert(
            Language::Go,
            vec!["go".to_string()],
        );

        // C
        patterns.insert(
            Language::C,
            vec![
                "#include <".to_string(),
                "int main(".to_string(),
                "printf(".to_string(),
            ],
        );
        shebang_patterns.insert(
            Language::C,
            vec!["gcc".to_string(), "clang".to_string()],
        );

        // C++
        patterns.insert(
            Language::Cpp,
            vec![
                "#include <".to_string(),
                "int main(".to_string(),
                "std::".to_string(),
                "cout <<".to_string(),
                "class ".to_string(),
            ],
        );
        shebang_patterns.insert(
            Language::Cpp,
            vec!["g++".to_string(), "clang++".to_string()],
        );

        // Java
        patterns.insert(
            Language::Java,
            vec![
                "public class".to_string(),
                "public static void main".to_string(),
                "System.out.println".to_string(),
            ],
        );
        shebang_patterns.insert(
            Language::Java,
            vec!["java".to_string(), "javac".to_string()],
        );

        // Ruby
        patterns.insert(
            Language::Ruby,
            vec![
                "def ".to_string(),
                "puts ".to_string(),
                "require ".to_string(),
                "end".to_string(),
            ],
        );
        shebang_patterns.insert(
            Language::Ruby,
            vec!["ruby".to_string()],
        );

        // PHP
        patterns.insert(
            Language::Php,
            vec![
                "<?php".to_string(),
                "echo ".to_string(),
                "$".to_string(),
            ],
        );
        shebang_patterns.insert(
            Language::Php,
            vec!["php".to_string()],
        );

        // Julia
        patterns.insert(
            Language::Julia,
            vec![
                "function ".to_string(),
                "println(".to_string(),
                "using ".to_string(),
                "end".to_string(),
            ],
        );
        shebang_patterns.insert(
            Language::Julia,
            vec!["julia".to_string()],
        );

        // R
        patterns.insert(
            Language::R,
            vec![
                "function(".to_string(),
                "print(".to_string(),
                "library(".to_string(),
                "<-".to_string(),
            ],
        );
        shebang_patterns.insert(
            Language::R,
            vec!["Rscript".to_string()],
        );

        // Perl
        patterns.insert(
            Language::Perl,
            vec![
                "my $".to_string(),
                "print ".to_string(),
                "use ".to_string(),
            ],
        );
        shebang_patterns.insert(
            Language::Perl,
            vec!["perl".to_string()],
        );

        // Swift
        patterns.insert(
            Language::Swift,
            vec![
                "func ".to_string(),
                "print(".to_string(),
                "import ".to_string(),
                "var ".to_string(),
                "let ".to_string(),
            ],
        );
        shebang_patterns.insert(
            Language::Swift,
            vec!["swift".to_string()],
        );

        // Kotlin
        patterns.insert(
            Language::Kotlin,
            vec![
                "fun main()".to_string(),
                "fun ".to_string(),
                "val ".to_string(),
                "var ".to_string(),
                "println(".to_string(),
            ],
        );
        shebang_patterns.insert(
            Language::Kotlin,
            vec!["kotlin".to_string()],
        );

        // C#
        patterns.insert(
            Language::CSharp,
            vec![
                "using System".to_string(),
                "class ".to_string(),
                "static void Main".to_string(),
                "Console.WriteLine".to_string(),
            ],
        );
        shebang_patterns.insert(
            Language::CSharp,
            vec!["dotnet".to_string(), "csc".to_string()],
        );

        // Lua
        patterns.insert(
            Language::Lua,
            vec![
                "function ".to_string(),
                "print(".to_string(),
                "local ".to_string(),
                "end".to_string(),
            ],
        );
        shebang_patterns.insert(
            Language::Lua,
            vec!["lua".to_string()],
        );

        // Dart
        patterns.insert(
            Language::Dart,
            vec![
                "void main()".to_string(),
                "print(".to_string(),
                "import '".to_string(),
                "var ".to_string(),
                "final ".to_string(),
            ],
        );
        shebang_patterns.insert(
            Language::Dart,
            vec!["dart".to_string()],
        );

        Self {
            patterns,
            shebang_patterns,
        }
    }

    /// Detect language from code content
    pub fn detect(&self, code: &str) -> Language {
        let code_lower = code.to_lowercase();

        // Check for shebang first
        if let Some(lang) = self.detect_from_shebang(code) {
            return lang;
        }

        // Score each language based on pattern matches
        let mut scores = HashMap::new();

        for (lang, patterns) in &self.patterns {
            let mut score = 0;
            for pattern in patterns {
                if code_lower.contains(&pattern.to_lowercase()) {
                    score += 1;
                }
            }
            scores.insert(*lang, score);
        }

        // Find language with highest score
        scores
            .into_iter()
            .max_by_key(|(_, score)| *score)
            .map(|(lang, _)| lang)
            .unwrap_or(Language::Python)
    }

    /// Detect language from shebang line
    fn detect_from_shebang(&self, code: &str) -> Option<Language> {
        let first_line = code.lines().next()?.trim();

        if !first_line.starts_with("#!") {
            return None;
        }

        let shebang = first_line.strip_prefix("#!")?.trim();

        for (lang, patterns) in &self.shebang_patterns {
            for pattern in patterns {
                if shebang.contains(pattern) {
                    return Some(*lang);
                }
            }
        }

        None
    }

    /// Detect language from file extension
    pub fn detect_from_extension(&self, path: &Path) -> Option<Language> {
        let ext = path.extension()?.to_str()?.to_lowercase();

        match ext.as_str() {
            "py" => Some(Language::Python),
            "js" => Some(Language::JavaScript),
            "ts" => Some(Language::TypeScript),
            "rs" => Some(Language::Rust),
            "sh" | "bash" => Some(Language::Shell),
            "go" => Some(Language::Go),
            "c" => Some(Language::C),
            "cpp" | "cc" | "cxx" => Some(Language::Cpp),
            "java" => Some(Language::Java),
            "rb" => Some(Language::Ruby),
            "php" => Some(Language::Php),
            "jl" => Some(Language::Julia),
            "r" => Some(Language::R),
            "pl" => Some(Language::Perl),
            "swift" => Some(Language::Swift),
            "kt" => Some(Language::Kotlin),
            "cs" => Some(Language::CSharp),
            "lua" => Some(Language::Lua),
            "dart" => Some(Language::Dart),
            _ => None,
        }
    }

    /// Get all supported languages
    pub fn all_languages() -> Vec<Language> {
        vec![
            Language::Python,
            Language::JavaScript,
            Language::TypeScript,
            Language::Rust,
            Language::Shell,
            Language::Go,
            Language::C,
            Language::Cpp,
            Language::Java,
            Language::Ruby,
            Language::Php,
            Language::Julia,
            Language::R,
            Language::Perl,
            Language::Swift,
            Language::Kotlin,
            Language::CSharp,
            Language::Lua,
            Language::Dart,
        ]
    }
}

impl Language {
    /// Get file extension for this language
    pub fn extension(&self) -> &'static str {
        match self {
            Language::Python => "py",
            Language::JavaScript => "js",
            Language::TypeScript => "ts",
            Language::Rust => "rs",
            Language::Shell => "sh",
            Language::Go => "go",
            Language::C => "c",
            Language::Cpp => "cpp",
            Language::Java => "java",
            Language::Ruby => "rb",
            Language::Php => "php",
            Language::Julia => "jl",
            Language::R => "r",
            Language::Perl => "pl",
            Language::Swift => "swift",
            Language::Kotlin => "kt",
            Language::CSharp => "cs",
            Language::Lua => "lua",
            Language::Dart => "dart",
        }
    }

    /// Get display name for this language
    pub fn display_name(&self) -> &'static str {
        match self {
            Language::Python => "Python",
            Language::JavaScript => "JavaScript",
            Language::TypeScript => "TypeScript",
            Language::Rust => "Rust",
            Language::Shell => "Shell",
            Language::Go => "Go",
            Language::C => "C",
            Language::Cpp => "C++",
            Language::Java => "Java",
            Language::Ruby => "Ruby",
            Language::Php => "PHP",
            Language::Julia => "Julia",
            Language::R => "R",
            Language::Perl => "Perl",
            Language::Swift => "Swift",
            Language::Kotlin => "Kotlin",
            Language::CSharp => "C#",
            Language::Lua => "Lua",
            Language::Dart => "Dart",
        }
    }

    /// Check if the language runtime is available
    pub fn is_available(&self) -> bool {
        match self {
            Language::Python => check_command("python3") || check_command("python"),
            Language::JavaScript => check_command("node"),
            Language::TypeScript => check_command("ts-node") || (check_command("node") && check_command("tsc")),
            Language::Rust => check_command("rustc"),
            Language::Shell => true, // Always available on Unix
            Language::Go => check_command("go"),
            Language::C => check_command("gcc") || check_command("clang"),
            Language::Cpp => check_command("g++") || check_command("clang++"),
            Language::Java => check_command("java") && check_command("javac"),
            Language::Ruby => check_command("ruby"),
            Language::Php => check_command("php"),
            Language::Julia => check_command("julia"),
            Language::R => check_command("Rscript"),
            Language::Perl => check_command("perl"),
            Language::Swift => check_command("swift"),
            Language::Kotlin => check_command("kotlin") || check_command("kotlinc"),
            Language::CSharp => check_command("dotnet"),
            Language::Lua => check_command("lua"),
            Language::Dart => check_command("dart"),
        }
    }

    /// Get the runtime version (if available)
    pub fn get_version(&self) -> Option<String> {
        match self {
            Language::Python => get_command_output("python3", &["--version"]),
            Language::JavaScript => get_command_output("node", &["--version"]),
            Language::TypeScript => get_command_output("tsc", &["--version"]),
            Language::Rust => get_command_output("rustc", &["--version"]),
            Language::Shell => Some("Bash".to_string()),
            Language::Go => get_command_output("go", &["version"]),
            Language::C => get_command_output("gcc", &["--version"]),
            Language::Cpp => get_command_output("g++", &["--version"]),
            Language::Java => get_command_output("java", &["-version"]),
            Language::Ruby => get_command_output("ruby", &["--version"]),
            Language::Php => get_command_output("php", &["--version"]),
            Language::Julia => get_command_output("julia", &["--version"]),
            Language::R => get_command_output("Rscript", &["--version"]),
            Language::Perl => get_command_output("perl", &["--version"]),
            Language::Swift => get_command_output("swift", &["--version"]),
            Language::Kotlin => get_command_output("kotlinc", &["-version"]),
            Language::CSharp => get_command_output("dotnet", &["--version"]),
            Language::Lua => get_command_output("lua", &["-v"]),
            Language::Dart => get_command_output("dart", &["--version"]),
        }
    }
}

/// Check if a command is available
fn check_command(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get command output (first line)
fn get_command_output(cmd: &str, args: &[&str]) -> Option<String> {
    Command::new(cmd)
        .args(args)
        .output()
        .ok()
        .and_then(|o| {
            String::from_utf8(o.stdout)
                .or_else(|_| String::from_utf8(o.stderr))
                .ok()
        })
        .map(|s| s.lines().next().unwrap_or("").to_string())
}

/// Runtime manager for checking and managing language runtimes
pub struct RuntimeManager {
    /// Cached runtime information
    runtimes: HashMap<Language, LanguageRuntime>,
}

impl RuntimeManager {
    /// Create a new runtime manager
    pub fn new() -> Self {
        Self {
            runtimes: HashMap::new(),
        }
    }

    /// Check and cache all available runtimes
    pub fn refresh_all(&mut self) {
        for lang in LanguageDetector::all_languages() {
            let runtime = LanguageRuntime {
                language: lang,
                available: lang.is_available(),
                version: lang.get_version(),
                executable_path: None,
                required_dependencies: Vec::new(),
            };
            self.runtimes.insert(lang, runtime);
        }
    }

    /// Get runtime info for a specific language
    pub fn get_runtime(&self, lang: Language) -> Option<&LanguageRuntime> {
        self.runtimes.get(&lang)
    }

    /// Get all available languages
    pub fn available_languages(&self) -> Vec<Language> {
        self.runtimes
            .values()
            .filter(|r| r.available)
            .map(|r| r.language)
            .collect()
    }

    /// List all runtimes with their status
    pub fn list_runtimes(&self) -> Vec<&LanguageRuntime> {
        self.runtimes.values().collect()
    }
}

impl Default for RuntimeManager {
    fn default() -> Self {
        let mut manager = Self::new();
        manager.refresh_all();
        manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_detector_creation() {
        let detector = LanguageDetector::new();
        assert!(!detector.patterns.is_empty());
    }

    #[test]
    fn test_detect_python() {
        let detector = LanguageDetector::new();
        let code = r#"
def hello():
    print("Hello, World!")

hello()
"#;
        assert_eq!(detector.detect(code), Language::Python);
    }

    #[test]
    fn test_detect_javascript() {
        let detector = LanguageDetector::new();
        let code = r#"
function hello() {
    console.log("Hello, World!");
}

hello();
"#;
        assert_eq!(detector.detect(code), Language::JavaScript);
    }

    #[test]
    fn test_detect_rust() {
        let detector = LanguageDetector::new();
        let code = r#"
fn main() {
    println!("Hello, World!");
}
"#;
        assert_eq!(detector.detect(code), Language::Rust);
    }

    #[test]
    fn test_detect_shell() {
        let detector = LanguageDetector::new();
        let code = r#"#!/bin/bash
echo "Hello, World!"
"#;
        assert_eq!(detector.detect(code), Language::Shell);
    }

    #[test]
    fn test_detect_from_extension() {
        let detector = LanguageDetector::new();

        assert_eq!(
            detector.detect_from_extension(Path::new("test.py")),
            Some(Language::Python)
        );
        assert_eq!(
            detector.detect_from_extension(Path::new("test.js")),
            Some(Language::JavaScript)
        );
        assert_eq!(
            detector.detect_from_extension(Path::new("test.rs")),
            Some(Language::Rust)
        );
    }

    #[test]
    fn test_language_display_name() {
        assert_eq!(Language::Python.display_name(), "Python");
        assert_eq!(Language::JavaScript.display_name(), "JavaScript");
        assert_eq!(Language::Rust.display_name(), "Rust");
        assert_eq!(Language::Cpp.display_name(), "C++");
        assert_eq!(Language::CSharp.display_name(), "C#");
    }

    #[test]
    fn test_language_extension() {
        assert_eq!(Language::Python.extension(), "py");
        assert_eq!(Language::JavaScript.extension(), "js");
        assert_eq!(Language::TypeScript.extension(), "ts");
        assert_eq!(Language::Rust.extension(), "rs");
    }

    #[test]
    fn test_all_languages() {
        let langs = LanguageDetector::all_languages();
        assert!(!langs.is_empty());
        assert!(langs.contains(&Language::Python));
        assert!(langs.contains(&Language::Rust));
    }

    #[test]
    fn test_runtime_manager() {
        let manager = RuntimeManager::new();
        // Should not panic
        let _ = manager.list_runtimes();
    }
}
