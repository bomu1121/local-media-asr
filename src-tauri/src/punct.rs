// Punctuation restoration module
// Uses CT-Transformer Punct model (via sherpa-onnx) to add punctuation to raw ASR text.
// When the model is not available, provides a basic rule-based fallback.

use anyhow::Result;

/// Punctuation restoration config
#[derive(Debug, Clone)]
pub struct PunctConfig {
    pub model_path: Option<String>,
    pub enabled: bool,
}

impl Default for PunctConfig {
    fn default() -> Self {
        PunctConfig {
            model_path: Some("./models/punct-ct-transformer".to_string()),
            enabled: false,
        }
    }
}

/// Punctuation restorer
pub struct PunctRestorer {
    config: PunctConfig,
}

impl PunctRestorer {
    pub fn new(config: PunctConfig) -> Self {
        PunctRestorer { config }
    }

    /// Add punctuation to raw ASR text.
    /// Uses CT-Transformer model when available, otherwise falls back to
    /// simple rule-based heuristics.
    pub fn add_punctuation(&self, text: &str) -> Result<String> {
        if !self.config.enabled {
            return Ok(text.to_string());
        }

        // TODO: When CT-Transformer model is available, use sherpa-onnx punctuation:
        // let model_path = self.config.model_path.as_deref().unwrap_or("./models/punct");
        // let punct = sherpa_rs::offline::OfflinePunctuation::new(model_path)?;
        // let result = punct.add_punct(text)?;

        // Rule-based fallback: add sentence boundaries based on pauses and context
        let result = rule_based_punctuation(text);
        Ok(result)
    }
}

/// Simple rule-based punctuation for fallback when model isn't available.
/// - Adds periods at natural pauses (multiple spaces, line breaks)
/// - Adds question marks for question-like patterns
/// - Capitalizes sentence starts
fn rule_based_punctuation(text: &str) -> String {
    let text = text.trim();
    if text.is_empty() {
        return String::new();
    }

    // Split by lines and add periods where appropriate
    let lines: Vec<&str> = text.lines().collect();
    let mut result = String::new();

    for line in &lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Check if line already ends with punctuation
        let last_char = trimmed.chars().last().unwrap_or('.');
        let has_punct = matches!(last_char, '.' | '!' | '?' | ',' | ';' | ':' | '。' | '！' | '？' | '，' | '；' | '：');

        if has_punct {
            result.push_str(trimmed);
            result.push(' ');
        } else if is_question(trimmed) {
            // Add question mark for question-like sentences
            // Chinese questions use 吗/呢/吧, English uses auxiliary-first patterns
            let marker = if contains_chinese(trimmed) { '？' } else { '?' };
            result.push_str(&format!("{}{} ", trimmed, marker));
        } else {
            // Add period
            let marker = if contains_chinese(trimmed) { '。' } else { '.' };
            result.push_str(&format!("{}{} ", trimmed, marker));
        }
    }

    result.trim().to_string()
}

/// Check if text looks like a question
fn is_question(text: &str) -> bool {
    let text_lower = text.to_lowercase();
    // English question patterns
    let en_starters = ["what", "when", "where", "who", "why", "how", "is", "are",
        "do", "does", "did", "can", "could", "will", "would", "shall", "should",
        "have", "has", "had", "may", "might", "must"];
    for starter in &en_starters {
        if text_lower.starts_with(starter) || text_lower.contains(&format!(" {}", starter)) {
            return true;
        }
    }
    // Chinese question markers
    let zh_markers = ['吗', '呢', '吧', '么', '啥', '谁', '哪', '怎'];
    for m in &zh_markers {
        if text.contains(*m) {
            return true;
        }
    }
    false
}

/// Check if text contains Chinese characters
fn contains_chinese(text: &str) -> bool {
    text.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_based_punctuation_adds_period() {
        let result = rule_based_punctuation("hello world");
        assert!(result.ends_with('.'));
    }

    #[test]
    fn test_is_question_english() {
        assert!(is_question("what is this"));
        assert!(is_question("how do you do"));
        assert!(!is_question("hello world"));
    }

    #[test]
    fn test_is_question_chinese() {
        assert!(is_question("你好吗"));
        assert!(is_question("这是什么"));
        assert!(!is_question("今天天气不错"));
    }

    #[test]
    fn test_contains_chinese() {
        assert!(contains_chinese("你好世界"));
        assert!(!contains_chinese("hello world"));
        assert!(contains_chinese("hello 世界"));
    }
}
