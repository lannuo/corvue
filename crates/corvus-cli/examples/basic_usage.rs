//! Basic usage example for Corvus
//!
//! This example demonstrates the core functionality of Corvus:
//! - Chain-of-Thought reasoning
//! - Memory system with TagMemo
//! - Language detection
//! - Security configuration

use corvus_reasoning::chain_of_thought::ChainOfThought;
use corvus_memory::tagmemo::TagMemoMemory;
use corvus_execution::languages::LanguageDetector;
use corvus_execution::security::{SecurityManager, PermissionSet, Permission, PathPattern};
use corvus_core::memory::{MemoryItem, ContentType, MemorySystem};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Corvus Basic Usage Example ===\n");

    // Example 1: Chain-of-Thought Reasoning
    println!("1. Chain-of-Thought Reasoning");
    println!("------------------------------");
    let mut cot = ChainOfThought::new();

    cot.observe("我需要计算 25 + 37 * 2".to_string(), 0.9);
    cot.reason("根据数学运算优先级，应该先算乘法再算加法".to_string(), 0.95);
    cot.plan("先计算 37 * 2，然后加上 25".to_string(), 0.9);
    cot.execute("37 * 2 = 74".to_string(), 0.9);
    cot.execute("25 + 74 = 99".to_string(), 0.95);
    cot.conclude("结果是 99".to_string(), 0.98);

    println!("{}", cot);
    println!("整体置信度: {:.2}\n", cot.overall_confidence());

    // Example 2: TagMemo Memory System
    println!("2. TagMemo Memory System");
    println!("------------------------");
    let memory = TagMemoMemory::with_in_memory_storage(128)?;

    // Add some tags
    memory.add_tag("rust".to_string(), true, None);
    memory.add_tag("programming".to_string(), true, None);
    memory.add_tag("systems".to_string(), false, None);

    // Associate tags
    memory.associate_tags("rust", "programming", 0.9);
    memory.associate_tags("rust", "systems", 0.8);

    // Store a memory item using the MemorySystem trait
    let item = MemoryItem::new(
        "Rust is a systems programming language",
        ContentType::Text,
    ).with_tags(vec!["rust", "programming"]);
    let memory_id = MemorySystem::store(&memory, item).await?;

    println!("Added memory with ID: {}", memory_id);
    println!("Total tags: {}", memory.all_tags().len());
    println!("Core tags: {}\n", memory.core_tags().len());

    // Example 3: Language Detection
    println!("3. Language Detection");
    println!("---------------------");
    let detector = LanguageDetector::new();

    let code_samples = vec![
        ("print('Hello, World!')", "Python"),
        ("console.log('Hello, World!');", "JavaScript"),
        ("fn main() { println!(\"Hello, World!\"); }", "Rust"),
    ];

    for (code, expected) in code_samples {
        let detected = detector.detect(code);
        println!("Code: {}", code);
        println!("Expected: {}, Detected: {:?}\n", expected, detected);
    }

    // Example 4: Security Configuration
    println!("4. Security Configuration");
    println!("--------------------------");
    let mut security = SecurityManager::new();

    // Grant some permissions
    let mut permissions = PermissionSet::new();
    permissions.grant(Permission::FileRead(PathPattern::Any));
    permissions.grant(Permission::FileWrite(PathPattern::Prefix("/tmp".into())));

    *security.permissions_mut() = permissions;

    println!("Security manager created");
    println!("Can read /etc/passwd: {}", security.check_file_read("/etc/passwd".as_ref()));
    println!("Can write /tmp/test.txt: {}", security.check_file_write("/tmp/test.txt".as_ref()));

    println!("\n=== Example Complete ===");

    Ok(())
}
