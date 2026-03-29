//! Output formatting and color coding utilities

use console::style;
use corvus_core::completion::{CompletionDelta, MessageDelta, ToolCallDelta};
use corvus_core::tool::ToolCall;
use std::collections::HashMap;
use std::fmt;

/// A styled code block with language info
#[derive(Debug, Clone)]
pub struct CodeBlock {
    /// The code content
    pub content: String,
    /// Language identifier (if provided)
    pub language: Option<String>,
}

/// Parse text to extract code blocks
pub fn parse_code_blocks(text: &str) -> Vec<CodeBlock> {
    let mut blocks = Vec::new();
    let mut in_block = false;
    let mut current_lang: Option<String> = None;
    let mut current_content = String::new();

    for line in text.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("```") {
            if in_block {
                // End of block
                blocks.push(CodeBlock {
                    content: current_content.trim_end().to_string(),
                    language: current_lang.take(),
                });
                current_content.clear();
                in_block = false;
            } else {
                // Start of block
                let lang = trimmed.trim_start_matches("```").trim().to_string();
                current_lang = if lang.is_empty() { None } else { Some(lang) };
                in_block = true;
            }
        } else if in_block {
            current_content.push_str(line);
            current_content.push('\n');
        }
    }

    blocks
}

/// Display a styled header
pub fn print_header(title: &str) {
    println!("\n{}", style(title).bold().underlined());
}

/// Display a success message
pub fn print_success(message: &str) {
    println!("{} {}", style("✓").green().bold(), style(message).green());
}

/// Display an info message
pub fn print_info(message: &str) {
    println!("{} {}", style("ℹ").cyan().bold(), style(message).cyan());
}

/// Display a warning message
pub fn print_warning(message: &str) {
    println!("{} {}", style("⚠").yellow().bold(), style(message).yellow());
}

/// Display a code block with syntax highlighting (basic)
pub fn print_code_block(block: &CodeBlock) {
    let lang_label = block.language.as_deref().unwrap_or("code");
    println!("\n{}", style(format!("[{}]", lang_label)).dim().bold());

    for line in block.content.lines() {
        println!("  {}", style(line).dim());
    }
}

/// Format and print text with code blocks
pub fn format_response(text: &str) {
    let blocks = parse_code_blocks(text);

    if blocks.is_empty() {
        println!("{}", style(text));
        return;
    }

    // Simple version: print all non-code text, then all code blocks
    // For a better implementation, we'd track positions properly
    println!("{}", style(text));
}

/// A simple progress bar
pub struct ProgressBar {
    current: u64,
    total: u64,
    width: usize,
    message: String,
}

impl ProgressBar {
    /// Create a new progress bar
    pub fn new(total: u64, message: impl Into<String>) -> Self {
        Self {
            current: 0,
            total,
            width: 50,
            message: message.into(),
        }
    }

    /// Set the current progress
    pub fn set(&mut self, current: u64) {
        self.current = current.min(self.total);
        self.render();
    }

    /// Increment the progress
    pub fn inc(&mut self, amount: u64) {
        self.current = (self.current + amount).min(self.total);
        self.render();
    }

    /// Finish the progress bar
    pub fn finish(&mut self) {
        self.current = self.total;
        self.render();
        println!();
    }

    fn render(&self) {
        let percent = if self.total == 0 {
            100
        } else {
            (self.current * 100) / self.total
        };

        let filled = (self.current * self.width as u64) / self.total.max(1);
        let empty = self.width - filled as usize;

        let bar = format!(
            "[{}{}] {}% - {}",
            "█".repeat(filled as usize),
            "░".repeat(empty),
            percent,
            self.message
        );

        print!("\r{}", style(bar).cyan());
        std::io::Write::flush(&mut std::io::stdout()).ok();
    }
}

impl Drop for ProgressBar {
    fn drop(&mut self) {
        if self.current < self.total {
            println!();
        }
    }
}

/// Format a list of items
pub fn print_list<T: fmt::Display>(items: &[T], title: Option<&str>) {
    if let Some(t) = title {
        println!("\n{}", style(t).bold());
    }

    for (i, item) in items.iter().enumerate() {
        println!("  {}. {}", style(i + 1).dim(), item);
    }
}

/// Format key-value pairs
pub fn print_key_value<K: fmt::Display, V: fmt::Display>(pairs: &[(K, V)]) {
    for (key, value) in pairs {
        println!("  {}: {}", style(key).cyan().bold(), value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_code_blocks() {
        let text = "Some text\n```rust\nfn main() {}\n```\nMore text";
        let blocks = parse_code_blocks(text);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].language, Some("rust".to_string()));
        assert_eq!(blocks[0].content, "fn main() {}");
    }

    #[test]
    fn test_parse_multiple_blocks() {
        let text = "```python\nprint(1)\n```\n```rust\nlet x = 1;\n```";
        let blocks = parse_code_blocks(text);
        assert_eq!(blocks.len(), 2);
    }

    #[test]
    fn test_parse_no_blocks() {
        let text = "Just some plain text";
        let blocks = parse_code_blocks(text);
        assert_eq!(blocks.len(), 0);
    }

    #[test]
    fn test_streaming_handler_new() {
        let handler = StreamingResponseHandler::new();
        assert!(handler.content().is_empty());
        assert!(!handler.has_tool_calls());
    }

    #[test]
    fn test_streaming_handler_reset() {
        let mut handler = StreamingResponseHandler::new();
        handler.content.push_str("test content");
        handler.reset();
        assert!(handler.content().is_empty());
    }
}

/// Streaming response handler that accumulates deltas and displays them
pub struct StreamingResponseHandler {
    /// Full accumulated content
    content: String,
    /// Tool calls being accumulated
    tool_calls: HashMap<u32, PartialToolCall>,
    /// Completed tool calls
    completed_tool_calls: Vec<ToolCall>,
    /// Whether the first chunk has been printed
    first_chunk: bool,
    /// Current line buffer (for partial lines)
    line_buffer: String,
    /// Whether we're currently in a tool call
    in_tool_call: bool,
}

/// A partially accumulated tool call
#[derive(Debug, Clone)]
struct PartialToolCall {
    pub id: Option<String>,
    pub name: Option<String>,
    pub arguments: String,
    pub shown: bool,
}

impl StreamingResponseHandler {
    /// Create a new streaming response handler
    pub fn new() -> Self {
        Self {
            content: String::new(),
            tool_calls: HashMap::new(),
            completed_tool_calls: Vec::new(),
            first_chunk: true,
            line_buffer: String::new(),
            in_tool_call: false,
        }
    }

    /// Handle a single completion delta
    pub fn handle_delta(&mut self, delta: &CompletionDelta) {
        if self.first_chunk {
            println!("\n{}", style("Corvus:").blue().bold());
            self.first_chunk = false;
        }

        self.handle_message_delta(&delta.delta);
    }

    /// Handle a message delta
    fn handle_message_delta(&mut self, delta: &MessageDelta) {
        // Handle content
        if let Some(content) = &delta.content {
            if self.in_tool_call {
                // We were in a tool call, now back to content
                println!();
                self.in_tool_call = false;
            }
            self.content.push_str(content);
            self.print_content(content);
        }

        // Handle tool calls
        if let Some(tool_calls) = &delta.tool_calls {
            for tool_call_delta in tool_calls {
                self.handle_tool_call_delta(tool_call_delta);
            }
        }
    }

    /// Print content with proper buffering
    fn print_content(&mut self, content: &str) {
        let full_content = self.line_buffer.clone() + content;
        let mut lines = full_content.lines().peekable();

        while let Some(line) = lines.next() {
            if lines.peek().is_some() {
                // Complete line
                println!("{}", line);
            } else {
                // Partial line, buffer it
                self.line_buffer = line.to_string();
                // Print without newline
                print!("{}", line);
                std::io::Write::flush(&mut std::io::stdout()).ok();
                return;
            }
        }

        // If we get here, there's no partial line
        self.line_buffer.clear();
    }

    /// Handle a tool call delta
    fn handle_tool_call_delta(&mut self, delta: &ToolCallDelta) {
        let partial = self.tool_calls.entry(delta.index).or_insert_with(|| PartialToolCall {
            id: None,
            name: None,
            arguments: String::new(),
            shown: false,
        });

        if let Some(id) = &delta.id {
            partial.id = Some(id.clone());
        }

        if let Some(name) = &delta.name {
            partial.name = Some(name.clone());
            if !partial.shown {
                // First time we have both ID and name, show tool call indicator
                if !self.line_buffer.is_empty() {
                    println!();
                    self.line_buffer.clear();
                }
                println!("\n{} {}", style("⚙").yellow(), style(format!("Calling tool: {}", name)).yellow());
                self.in_tool_call = true;
                partial.shown = true;
            }
        }

        if let Some(args) = &delta.arguments {
            partial.arguments.push_str(args);
        }
    }

    /// Get the full accumulated content
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Check if we have accumulated any tool calls
    pub fn has_tool_calls(&self) -> bool {
        !self.tool_calls.is_empty() || !self.completed_tool_calls.is_empty()
    }

    /// Get the accumulated tool calls as complete ToolCall objects
    pub fn tool_calls(&self) -> Vec<ToolCall> {
        let mut result = self.completed_tool_calls.clone();

        for partial in self.tool_calls.values() {
            if let (Some(id), Some(name)) = (&partial.id, &partial.name) {
                if let Ok(args) = serde_json::from_str(&partial.arguments) {
                    result.push(ToolCall::new(id.clone(), name.clone(), args));
                }
            }
        }

        result
    }

    /// Mark tool calls as completed and prepare for next response
    pub fn complete_tool_calls(&mut self) {
        for (_, partial) in self.tool_calls.drain() {
            if let (Some(id), Some(name)) = (&partial.id, &partial.name) {
                if let Ok(args) = serde_json::from_str(&partial.arguments) {
                    self.completed_tool_calls.push(ToolCall::new(id.clone(), name.clone(), args));
                }
            }
        }
    }

    /// Show tool result
    pub fn show_tool_result(&mut self, tool_name: &str, result: &str) {
        println!("{} {} {}", style("✓").green(), style(format!("Tool {} completed", tool_name)).green(), style("↓").dim());

        // Truncate long results for display
        let display_result = if result.len() > 500 {
            format!("{}... (truncated)", &result[..500])
        } else {
            result.to_string()
        };

        for line in display_result.lines() {
            println!("  {}", style(line).dim());
        }
        println!();
        self.in_tool_call = false;
    }

    /// Finish the response and flush any remaining buffer
    pub fn finish(&mut self) {
        if !self.line_buffer.is_empty() {
            println!();
            self.line_buffer.clear();
        }
        println!();
    }

    /// Reset for a new response
    pub fn reset(&mut self) {
        self.content.clear();
        self.tool_calls.clear();
        self.completed_tool_calls.clear();
        self.first_chunk = true;
        self.line_buffer.clear();
        self.in_tool_call = false;
    }
}

impl Default for StreamingResponseHandler {
    fn default() -> Self {
        Self::new()
    }
}
