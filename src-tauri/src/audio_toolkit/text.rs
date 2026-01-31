use tracing::{debug, info};
use rphonetic::{DoubleMetaphone, Encoder};
use strsim::{damerau_levenshtein, jaro_winkler};
use unicode_segmentation::UnicodeSegmentation;

/// Threshold for using Jaro-Winkler vs Damerau-Levenshtein
/// Jaro-Winkler is better for short strings due to prefix emphasis
const SHORT_WORD_THRESHOLD: usize = 6;

/// Phonetic codes for a word (primary and alternate Double Metaphone codes)
struct PhoneticCodes {
    primary: String,
    alternate: String,
}

impl PhoneticCodes {
    fn new(encoder: &DoubleMetaphone, word: &str) -> Self {
        Self {
            primary: encoder.encode(word),
            alternate: encoder.encode_alternate(word),
        }
    }

    /// Check if any of our codes match any of the other word's codes
    fn matches(&self, other: &PhoneticCodes) -> bool {
        // Both codes being empty means no phonetic representation available
        if self.primary.is_empty() || other.primary.is_empty() {
            return false;
        }

        // Check all 4 possible combinations
        self.primary == other.primary
            || (!self.primary.is_empty()
                && !other.alternate.is_empty()
                && self.primary == other.alternate)
            || (!self.alternate.is_empty()
                && !other.primary.is_empty()
                && self.alternate == other.primary)
            || (!self.alternate.is_empty()
                && !other.alternate.is_empty()
                && self.alternate == other.alternate)
    }
}

/// Represents a custom word that may be a multi-word phrase
struct CustomPhrase {
    /// Original form as provided by user
    original: String,
    /// Phonetic codes of concatenated form (for "chat gpt" → "chatgpt")
    phonetic: PhoneticCodes,
    /// Number of words in the phrase
    word_count: usize,
    /// Concatenated lowercase form for matching
    concatenated: String,
}

impl CustomPhrase {
    fn new(original: &str, encoder: &DoubleMetaphone) -> Self {
        let normalized = original.to_lowercase();
        let words: Vec<String> = normalized.split_whitespace().map(String::from).collect();
        let concatenated: String = words.join("");
        let phonetic = PhoneticCodes::new(encoder, &concatenated);

        Self {
            original: original.to_string(),
            phonetic,
            word_count: words.len().max(1),
            concatenated,
        }
    }
}

/// Applies custom word corrections to transcribed text using fuzzy matching
///
/// This function corrects words in the input text by finding the best matches
/// from a list of custom words using a combination of:
/// - Exact case-insensitive matching (checked first to avoid false positives)
/// - Multi-word phrase matching with sliding window (for "chat GPT" → "ChatGPT")
/// - Jaro-Winkler for short words (≤6 chars) - better prefix matching
/// - Damerau-Levenshtein for longer words - handles transpositions
/// - Double Metaphone phonetic matching for pronunciation similarity
///
/// # Arguments
/// * `text` - The input text to correct
/// * `custom_words` - List of custom words to match against
/// * `threshold` - Maximum similarity score to accept (0.0 = exact match, 1.0 = any match)
///
/// # Returns
/// The corrected text with custom words applied
pub fn apply_custom_words(text: &str, custom_words: &[String], threshold: f64) -> String {
    if custom_words.is_empty() {
        return text.to_string();
    }

    info!(
        "[CustomWords] Processing: '{}' with {} custom words, threshold={}",
        text,
        custom_words.len(),
        threshold
    );

    // Pre-compute phrase data
    let double_metaphone = DoubleMetaphone::default();
    let phrases: Vec<CustomPhrase> = custom_words
        .iter()
        .map(|w| CustomPhrase::new(w, &double_metaphone))
        .collect();

    // Max window size for multi-word phrase matching
    // Check up to 3-word combinations to handle cases like "chat GPT" → "ChatGPT"
    let phrase_max = phrases.iter().map(|p| p.word_count).max().unwrap_or(1);
    let max_window_size = phrase_max.max(3);

    // Split input into words, preserving original forms
    let input_words: Vec<&str> = text.split_whitespace().collect();

    if input_words.is_empty() {
        return text.to_string();
    }

    let mut result: Vec<String> = Vec::new();
    let mut i = 0;

    while i < input_words.len() {
        // First, try single-word matching (most common case)
        let single_word = input_words[i];
        let single_cleaned = single_word
            .chars()
            .filter(|c| c.is_alphabetic())
            .collect::<String>()
            .to_lowercase();

        if single_cleaned.is_empty() || single_cleaned.len() > 50 {
            result.push(single_word.to_string());
            i += 1;
            continue;
        }

        // Check for exact single-word match first
        let mut found_match: Option<(&CustomPhrase, usize, f64)> = None;

        for phrase in &phrases {
            if single_cleaned == phrase.concatenated {
                info!(
                    "[CustomWords] Exact single-word match: '{}' -> '{}'",
                    single_word, phrase.original
                );
                found_match = Some((phrase, 1, 0.0));
                break;
            }
        }

        // If no exact single-word match, try multi-word EXACT match
        // (only accept multi-word for exact matches to avoid over-matching)
        if found_match.is_none() {
            for window_size in (2..=max_window_size.min(input_words.len() - i)).rev() {
                let window_words: Vec<&str> = input_words[i..i + window_size].to_vec();
                let window_cleaned: String = window_words
                    .iter()
                    .map(|w| {
                        w.chars()
                            .filter(|c| c.is_alphabetic())
                            .collect::<String>()
                            .to_lowercase()
                    })
                    .collect::<Vec<_>>()
                    .join("");

                for phrase in &phrases {
                    // Only accept multi-word match if it's EXACT
                    if window_cleaned == phrase.concatenated {
                        let window_text = window_words.join(" ");
                        info!(
                            "[CustomWords] Exact multi-word match: '{}' -> '{}'",
                            window_text, phrase.original
                        );
                        found_match = Some((phrase, window_size, 0.0));
                        break;
                    }
                }

                if found_match.is_some() {
                    break;
                }
            }
        }

        // If still no exact match, try fuzzy single-word matching
        if found_match.is_none() {
            let word_codes = PhoneticCodes::new(&double_metaphone, &single_cleaned);
            let mut best_score = f64::MAX;

            for phrase in &phrases {
                // Skip if lengths are too different
                let len_diff =
                    (single_cleaned.len() as i32 - phrase.concatenated.len() as i32).abs();
                if len_diff > 5 {
                    continue;
                }

                // Calculate string similarity score
                // Use Jaro-Winkler for short strings (better for names/prefixes)
                // Use Damerau-Levenshtein for longer strings (better for transpositions)
                // Only use Jaro-Winkler when:
                // 1. BOTH strings are short (≤6 chars)
                // 2. They have similar lengths (at least half of each other)
                let len1 = single_cleaned.len();
                let len2 = phrase.concatenated.len();
                let lengths_similar = len1.min(len2) * 2 >= len1.max(len2);
                let use_jaro = len1 <= SHORT_WORD_THRESHOLD
                    && len2 <= SHORT_WORD_THRESHOLD
                    && lengths_similar;

                let base_score = if use_jaro {
                    // Jaro-Winkler returns 0.0-1.0 similarity, convert to distance
                    // Use stricter matching for short words to prevent false positives
                    let jw_distance = 1.0 - jaro_winkler(&single_cleaned, &phrase.concatenated);
                    // Only accept if very similar (Jaro-Winkler > 0.85 similarity = < 0.15 distance)
                    if jw_distance > 0.2 {
                        1.0 // Reject: not similar enough
                    } else {
                        jw_distance
                    }
                } else {
                    // Damerau-Levenshtein normalized by length
                    let damerau_dist = damerau_levenshtein(&single_cleaned, &phrase.concatenated);
                    let max_len = single_cleaned.len().max(phrase.concatenated.len()) as f64;
                    if max_len > 0.0 {
                        damerau_dist as f64 / max_len
                    } else {
                        1.0
                    }
                };

                // Phonetic matching
                let phonetic_match = word_codes.matches(&phrase.phonetic);

                // Combined score with phonetic boost
                let combined_score = if phonetic_match {
                    base_score * 0.3
                } else {
                    base_score
                };

                debug!(
                    "[CustomWords] '{}' vs '{}': base={:.3}, phonetic={}, combined={:.3}, algo={}",
                    single_cleaned, phrase.concatenated, base_score, phonetic_match, combined_score,
                    if use_jaro { "jaro-winkler" } else { "damerau-lev" }
                );

                if combined_score < threshold && combined_score < best_score {
                    found_match = Some((phrase, 1, combined_score));
                    best_score = combined_score;
                }
            }
        }

        // Apply the match or keep original
        if let Some((phrase, words_consumed, score)) = found_match {
            let first_word = input_words[i];
            let corrected = preserve_case_pattern(first_word, &phrase.original);
            let (prefix, _) = extract_punctuation(first_word);
            let (_, suffix) = extract_punctuation(input_words[i + words_consumed - 1]);

            info!(
                "[CustomWords] Matched {} word(s): '{}' -> '{}' (score={:.3})",
                words_consumed,
                input_words[i..i + words_consumed].join(" "),
                phrase.original,
                score
            );

            result.push(format!("{}{}{}", prefix, corrected, suffix));
            i += words_consumed;
        } else {
            result.push(single_word.to_string());
            i += 1;
        }
    }

    let output = result.join(" ");
    info!("[CustomWords] Result: '{}'", output);
    output
}

/// Preserves the case pattern of the original word when applying a replacement
fn preserve_case_pattern(original: &str, replacement: &str) -> String {
    let alpha_chars: Vec<char> = original.chars().filter(|c| c.is_alphabetic()).collect();

    if alpha_chars.is_empty() {
        return replacement.to_string();
    }

    if alpha_chars.iter().all(|c| c.is_uppercase()) {
        replacement.to_uppercase()
    } else if alpha_chars.first().map_or(false, |c| c.is_uppercase()) {
        let mut chars: Vec<char> = replacement.chars().collect();
        if let Some(first_char) = chars.get_mut(0) {
            *first_char = first_char.to_uppercase().next().unwrap_or(*first_char);
        }
        chars.into_iter().collect()
    } else {
        replacement.to_string()
    }
}

/// Extracts punctuation prefix and suffix from a word
fn extract_punctuation(word: &str) -> (&str, &str) {
    let prefix_end = word.chars().take_while(|c| !c.is_alphabetic()).count();
    let suffix_start = word
        .char_indices()
        .rev()
        .take_while(|(_, c)| !c.is_alphabetic())
        .count();

    let prefix = if prefix_end > 0 {
        &word[..prefix_end]
    } else {
        ""
    };

    let suffix = if suffix_start > 0 {
        &word[word.len() - suffix_start..]
    } else {
        ""
    };

    (prefix, suffix)
}

// ============================================
// Filler Word Removal (from main branch)
// ============================================

use once_cell::sync::Lazy;
use regex::Regex;

/// Filler words to remove from transcriptions
const FILLER_WORDS: &[&str] = &[
    "uh", "um", "uhm", "umm", "uhh", "uhhh", "ah", "eh", "hmm", "hm", "mmm", "mm", "mh", "ha",
    "ehh",
];

static MULTI_SPACE_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s{2,}").unwrap());

/// Collapses repeated 1-2 letter words (3+ repetitions) to a single instance.
/// E.g., "wh wh wh wh" -> "wh", "I I I I" -> "I"
fn collapse_stutters(text: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return text.to_string();
    }

    let mut result: Vec<&str> = Vec::new();
    let mut i = 0;

    while i < words.len() {
        let word = words[i];
        let word_lower = word.to_lowercase();

        // Only process 1-2 letter words
        if word_lower.len() <= 2 && word_lower.chars().all(|c| c.is_alphabetic()) {
            // Count consecutive repetitions (case-insensitive)
            let mut count = 1;
            while i + count < words.len() && words[i + count].to_lowercase() == word_lower {
                count += 1;
            }

            // If 3+ repetitions, collapse to single instance
            if count >= 3 {
                result.push(word);
                i += count;
            } else {
                result.push(word);
                i += 1;
            }
        } else {
            result.push(word);
            i += 1;
        }
    }

    result.join(" ")
}

/// Pre-compiled filler word patterns (built lazily)
static FILLER_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    FILLER_WORDS
        .iter()
        .map(|word| {
            // Match filler word with word boundaries, optionally followed by comma or period
            Regex::new(&format!(r"(?i)\b{}\b[,.]?", regex::escape(word))).unwrap()
        })
        .collect()
});

/// Filters transcription output by removing filler words and stutter artifacts.
///
/// This function cleans up raw transcription text by:
/// 1. Removing filler words (uh, um, hmm, etc.)
/// 2. Collapsing repeated 1-2 letter stutters (e.g., "wh wh wh" -> "wh")
/// 3. Cleaning up excess whitespace
///
/// # Arguments
/// * `text` - The raw transcription text to filter
///
/// # Returns
/// The filtered text with filler words and stutters removed
pub fn filter_transcription_output(text: &str) -> String {
    let mut filtered = text.to_string();

    // Remove filler words
    for pattern in FILLER_PATTERNS.iter() {
        filtered = pattern.replace_all(&filtered, "").to_string();
    }

    // Collapse repeated 1-2 letter words (stutter artifacts like "wh wh wh wh")
    filtered = collapse_stutters(&filtered);

    // Clean up multiple spaces to single space
    filtered = MULTI_SPACE_PATTERN.replace_all(&filtered, " ").to_string();

    // Trim leading/trailing whitespace
    filtered.trim().to_string()
}

/// Counts words in text using Unicode segmentation rules.
/// This handles CJK languages correctly where words are not separated by spaces,
/// as well as standard space-separated languages.
///
/// # Arguments
/// * `text` - The text to count words in
///
/// # Returns
/// The number of words in the text
pub fn count_words(text: &str) -> usize {
    text.unicode_words().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_custom_words_exact_match() {
        let text = "hello world";
        let custom_words = vec!["Hello".to_string(), "World".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_apply_custom_words_fuzzy_match() {
        let text = "helo wrold";
        let custom_words = vec!["hello".to_string(), "world".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_preserve_case_pattern() {
        assert_eq!(preserve_case_pattern("HELLO", "world"), "WORLD");
        assert_eq!(preserve_case_pattern("Hello", "world"), "World");
        assert_eq!(preserve_case_pattern("hello", "WORLD"), "WORLD");
    }

    #[test]
    fn test_extract_punctuation() {
        assert_eq!(extract_punctuation("hello"), ("", ""));
        assert_eq!(extract_punctuation("!hello?"), ("!", "?"));
        assert_eq!(extract_punctuation("...hello..."), ("...", "..."));
    }

    #[test]
    fn test_empty_custom_words() {
        let text = "hello world";
        let custom_words = vec![];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_exact_match_before_fuzzy() {
        let text = "handy is a great app";
        let custom_words = vec!["Handy".to_string(), "Candy".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "Handy is a great app");
    }

    #[test]
    fn test_transposition_handling() {
        let text = "teh quick brown fox";
        let custom_words = vec!["the".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "the quick brown fox");
    }

    #[test]
    fn test_double_metaphone_phonetic() {
        let text = "I have a kat";
        let custom_words = vec!["cat".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "I have a cat");
    }

    #[test]
    fn test_phonetic_name_matching() {
        let text = "I work at Anthrapik";
        let custom_words = vec!["Anthropic".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "I work at Anthropic");
    }

    // Multi-word phrase matching tests
    #[test]
    fn test_multi_word_phrase_exact() {
        // "chat GPT" (two words) should match "ChatGPT" custom word
        let text = "I use chat GPT daily";
        let custom_words = vec!["ChatGPT".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "I use ChatGPT daily");
    }

    #[test]
    fn test_multi_word_phrase_case_variations() {
        let custom_words = vec!["ChatGPT".to_string()];

        assert_eq!(
            apply_custom_words("chat gpt is cool", &custom_words, 0.5),
            "ChatGPT is cool"
        );
        assert_eq!(
            apply_custom_words("Chat GPT is cool", &custom_words, 0.5),
            "ChatGPT is cool"
        );
        assert_eq!(
            apply_custom_words("CHAT GPT is cool", &custom_words, 0.5),
            "CHATGPT is cool"
        );
    }

    #[test]
    fn test_single_word_still_works() {
        // Single word "chatgpt" should still match
        let text = "I use chatgpt daily";
        let custom_words = vec!["ChatGPT".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "I use ChatGPT daily");
    }

    #[test]
    fn test_three_word_phrase() {
        // Test with a three-word phrase
        let text = "I love new york city";
        let custom_words = vec!["New York City".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "I love New York City");
    }

    #[test]
    fn test_no_false_multi_word_match() {
        // "at Anthropic" should NOT match as a phrase
        let text = "I work at Anthropic";
        let custom_words = vec!["Anthropic".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        // Should match "Anthropic" as single word, not consume "at Anthropic"
        assert_eq!(result, "I work at Anthropic");
    }

    #[test]
    fn test_count_words_english() {
        assert_eq!(count_words("Hello world"), 2);
        assert_eq!(count_words("  Hello   world  "), 2);
        assert_eq!(count_words("One, two, three."), 3);
    }

    #[test]
    fn test_count_words_cjk() {
        // Chinese: "你好嗎?" (How are you?) -> "你", "好", "嗎" (3 words)
        // Note: unicode segmentation standard might treat this differently depending on exact rules,
        // but typically CJK characters are often treated as individual words or segmented by dictionary if available.
        // The unicode-segmentation crate follows UAX#29.
        // For "你好嗎?":
        // "你" (You)
        // "好" (Good)
        // "嗎" (Question particle)
        // "?" (Punctuation - usually ignored or separate depending on rules, but here count is 3 words)
        // Let's rely on the library's behavior which is better than split_whitespace (1 word).
        
        // Chinese: "你好嗎?" (How are you?) -> "你", "好", "嗎" (3 words)
        let chinese = "你好嗎?";
        let count = count_words(chinese);
        assert_eq!(count, 3, "Should identify 3 words in Chinese '你好嗎?', got {}", count);
        
        let mixed = "Hello 你好";
        assert_eq!(count_words(mixed), 3); // "Hello", "你", "好" (likely)
    }

    #[test]
    fn test_count_words_mixed_punctuation() {
        assert_eq!(count_words("Hello, world!"), 2);
        assert_eq!(count_words("It's a beautiful day."), 4); // "It's", "a", "beautiful", "day"
    }
}
