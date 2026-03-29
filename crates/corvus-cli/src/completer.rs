//! Tab completion for Corvus CLI

use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::Helper;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

/// Common chat commands that can be completed
pub const CHAT_COMMANDS: &[&str] = &[
    "exit", "quit", "help", "clear", "history", "save", "load", "model",
];

/// Common OpenAI models
pub const OPENAI_MODELS: &[&str] = &[
    "gpt-4o",
    "gpt-4o-mini",
    "gpt-4-turbo",
    "gpt-4",
    "gpt-3.5-turbo",
];

/// Common Ollama models
pub const OLLAMA_MODELS: &[&str] = &[
    "llama3:8b",
    "llama3:70b",
    "llama3.1:8b",
    "llama3.1:70b",
    "mistral:7b",
    "mistral",
    "gemma:2b",
    "gemma:7b",
    "codellama:7b",
    "codellama:13b",
    "nomic-embed-text",
    "mxbai-embed-large",
];

/// Corvus completer that provides intelligent suggestions
pub struct CorvusCompleter {
    /// Static commands that are always available
    commands: Vec<String>,
    /// Available models
    models: Vec<String>,
    /// Session IDs for completion
    session_ids: Arc<Mutex<Vec<String>>>,
    /// Custom words that have been seen in history
    custom_words: Arc<Mutex<HashSet<String>>>,
}

impl CorvusCompleter {
    /// Create a new Corvus completer
    pub fn new() -> Self {
        let mut commands = Vec::new();
        for cmd in CHAT_COMMANDS {
            commands.push(cmd.to_string());
        }

        let mut models = Vec::new();
        for model in OPENAI_MODELS {
            models.push(model.to_string());
        }
        for model in OLLAMA_MODELS {
            models.push(model.to_string());
        }

        Self {
            commands,
            models,
            session_ids: Arc::new(Mutex::new(Vec::new())),
            custom_words: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Update the list of available session IDs
    pub fn update_session_ids(&self, ids: Vec<String>) {
        if let Ok(mut session_ids) = self.session_ids.lock() {
            *session_ids = ids;
        }
    }

    /// Add a custom word to the completion dictionary
    pub fn add_custom_word(&self, word: &str) {
        if let Ok(mut words) = self.custom_words.lock() {
            words.insert(word.to_string());
        }
    }

    /// Add multiple words from a line of input
    pub fn add_words_from_line(&self, line: &str) {
        for word in line.split_whitespace() {
            if word.len() > 3 {
                self.add_custom_word(word);
            }
        }
    }

    /// Get completion candidates based on the input
    fn get_candidates(&self, line: &str, pos: usize) -> Vec<Pair> {
        let (_start, word) = extract_word_at_pos(line, pos);

        if word.is_empty() {
            return Vec::new();
        }

        let mut candidates = Vec::new();
        let lower_word = word.to_lowercase();

        // Check for command prefix
        if word.starts_with('/') || word.starts_with('!') {
            let cmd_part = &word[1..].to_lowercase();
            for cmd in &self.commands {
                if cmd.starts_with(cmd_part) {
                    candidates.push(Pair {
                        display: format!("/{}", cmd),
                        replacement: format!("/{}", cmd),
                    });
                }
            }
            return candidates;
        }

        // Check for model commands like "model:gpt-4"
        if let Some(colon_pos) = word.find(':') {
            let prefix = &word[..colon_pos].to_lowercase();
            let suffix = &word[colon_pos + 1..].to_lowercase();

            if prefix == "model" {
                for model in &self.models {
                    if model.starts_with(suffix) {
                        candidates.push(Pair {
                            display: format!("model:{}", model),
                            replacement: format!("model:{}", model),
                        });
                    }
                }
                return candidates;
            }
        }

        // Complete commands without prefix
        for cmd in &self.commands {
            if cmd.starts_with(&lower_word) {
                candidates.push(Pair {
                    display: cmd.clone(),
                    replacement: cmd.clone(),
                });
            }
        }

        // Complete model names
        for model in &self.models {
            if model.to_lowercase().starts_with(&lower_word) {
                candidates.push(Pair {
                    display: model.clone(),
                    replacement: model.clone(),
                });
            }
        }

        // Complete session IDs
        if let Ok(session_ids) = self.session_ids.lock() {
            for id in session_ids.iter() {
                if id.starts_with(&lower_word) {
                    candidates.push(Pair {
                        display: id.clone(),
                        replacement: id.clone(),
                    });
                }
            }
        }

        // Complete custom words
        if let Ok(custom_words) = self.custom_words.lock() {
            for word in custom_words.iter() {
                if word.to_lowercase().starts_with(&lower_word) {
                    candidates.push(Pair {
                        display: word.clone(),
                        replacement: word.clone(),
                    });
                }
            }
        }

        // Sort candidates by relevance
        candidates.sort_by(|a, b| {
            let a_starts = a.replacement.to_lowercase().starts_with(&lower_word);
            let b_starts = b.replacement.to_lowercase().starts_with(&lower_word);

            if a_starts && !b_starts {
                return std::cmp::Ordering::Less;
            }
            if !a_starts && b_starts {
                return std::cmp::Ordering::Greater;
            }

            a.replacement.len().cmp(&b.replacement.len())
        });

        candidates.dedup_by(|a, b| a.replacement == b.replacement);

        candidates
    }
}

impl Default for CorvusCompleter {
    fn default() -> Self {
        Self::new()
    }
}

impl Completer for CorvusCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> Result<(usize, Vec<Pair>), ReadlineError> {
        let (start, _) = extract_word_at_pos(line, pos);
        let candidates = self.get_candidates(line, pos);
        Ok((start, candidates))
    }
}

// Implement Helper trait for CorvusCompleter
impl Helper for CorvusCompleter {}

// Implement the required helper traits with default implementations
impl Hinter for CorvusCompleter {
    type Hint = String;
}

impl Highlighter for CorvusCompleter {}

impl Validator for CorvusCompleter {}

/// Extract the word at the given position in the line
fn extract_word_at_pos(line: &str, pos: usize) -> (usize, &str) {
    let line_bytes = line.as_bytes();
    let mut start = pos;

    while start > 0 {
        let c = line_bytes[start - 1] as char;
        if c.is_whitespace() || c == '(' || c == ')' || c == ',' || c == ';' || c == '"' {
            break;
        }
        start -= 1;
    }

    let mut end = pos;
    while end < line.len() {
        let c = line_bytes[end] as char;
        if c.is_whitespace() || c == '(' || c == ')' || c == ',' || c == ';' || c == '"' {
            break;
        }
        end += 1;
    }

    (start, &line[start..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_word_at_pos() {
        assert_eq!(extract_word_at_pos("hello world", 3), (0, "hello"));
        assert_eq!(extract_word_at_pos("hello world", 5), (0, "hello"));
        assert_eq!(extract_word_at_pos("hello world", 7), (6, "world"));
        assert_eq!(extract_word_at_pos("hello world", 10), (6, "world"));
    }

    #[test]
    fn test_completer_get_candidates_commands() {
        let completer = CorvusCompleter::new();
        let candidates = completer.get_candidates("ex", 2);

        assert!(!candidates.is_empty());
        assert!(candidates.iter().any(|c| c.replacement == "exit"));
    }

    #[test]
    fn test_completer_get_candidates_models() {
        let completer = CorvusCompleter::new();
        let candidates = completer.get_candidates("gpt-4", 5);

        assert!(!candidates.is_empty());
    }

    #[test]
    fn test_completer_get_candidates_session_ids() {
        let completer = CorvusCompleter::new();
        completer.update_session_ids(vec!["abc123".to_string(), "def456".to_string()]);

        let candidates = completer.get_candidates("abc", 3);

        assert!(candidates.iter().any(|c| c.replacement == "abc123"));
    }
}
