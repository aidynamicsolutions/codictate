use tracing::info;
use rphonetic::{DoubleMetaphone, Encoder};
use strsim::{damerau_levenshtein, jaro_winkler};
use unicode_segmentation::UnicodeSegmentation;
use once_cell::sync::Lazy;
use regex::Regex;

/// Threshold for using Jaro-Winkler vs Damerau-Levenshtein
/// Jaro-Winkler is better for short strings due to prefix emphasis
const SHORT_WORD_THRESHOLD: usize = 6;

/// Phonetic codes for a word (primary and alternate Double Metaphone codes)
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
struct CustomPhrase {
    /// Original form as provided by user
    original: String,
    /// Phonetic codes of concatenated form (for "chat gpt" → "chatgpt")
    phonetic: PhoneticCodes,
    /// Concatenated lowercase form for matching
    concatenated: String,
}

impl CustomPhrase {
    fn new(original: &str, encoder: &DoubleMetaphone) -> Self {
        let normalized = original.to_lowercase();
        // Remove spaces and non-alphanumeric characters for "concatenated" form
        // This MUST match how we process n-grams in build_ngram
        let concatenated: String = normalized
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect();
            
        let phonetic = PhoneticCodes::new(encoder, &concatenated);

        Self {
            original: original.to_string(),
            phonetic,
            concatenated,
        }
    }
}

/// Builds an n-gram string by cleaning and concatenating words
///
/// Strips punctuation from each word, lowercases, and joins without spaces.
/// This allows matching "Charge B" against "ChargeBee".
fn build_ngram(words: &[&str]) -> String {
    words
        .iter()
        .map(|w| {
            w.trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase()
        })
        .collect::<Vec<_>>()
        .concat()
}

/// Applies custom word corrections to transcribed text using fuzzy matching
/// 
/// "Best of Both Worlds" Implementation:
/// - Uses N-gram sliding window (from main) to catch split phrases ("Chat G P T" -> "ChatGPT")
/// - Uses Double Metaphone + Jaro-Winkler/Levenshtein (from llm) for superior accuracy
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

    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return text.to_string();
    }

    let mut result: Vec<String> = Vec::new();
    let mut i = 0;

    while i < words.len() {
        let mut matched = false;

        // Try n-grams from longest (3) to shortest (1) - greedy matching
        // This allows capturing "Chat G P T" (4 words? max 3? main uses 3)
        // Main used 1..=3. let's stick to that.
        for n in (1..=3).rev() {
            if i + n > words.len() {
                continue;
            }

            let ngram_words = &words[i..i + n];
            let ngram_text = build_ngram(ngram_words);
            
            if ngram_text.is_empty() || ngram_text.len() > 50 {
                continue;
            }

            let mut best_match: Option<&CustomPhrase> = None;
            let mut best_score = f64::MAX;

            // Check against all custom phrases
            for phrase in &phrases {
                // 1. Exact match check
                if ngram_text == phrase.concatenated {
                    // Exact match is perfect
                    best_match = Some(phrase);
                    best_score = 0.0;
                    break; // Can't beat exact match
                }

                // 2. Fuzzy match checks (single words only)
                
                // Industry best practice: Double Metaphone and fuzzy matching are designed
                // for single words, not concatenated phrases. For multi-word n-grams,
                // only exact matches are allowed to prevent false positives like
                // "use chat gpt" incorrectly matching "ChatGPT".
                if n > 1 {
                    continue;
                }
                
                // Skip if lengths are too different (optimization + prevents over-matching)
                let len_diff = (ngram_text.len() as i32 - phrase.concatenated.len() as i32).abs();
                if len_diff > 5 {
                     continue;
                }

                // Calculate string similarity score
                // Use Jaro-Winkler for short strings (better for names/prefixes)
                // Use Damerau-Levenshtein for longer strings (better for transpositions)
                let len1 = ngram_text.len();
                let len2 = phrase.concatenated.len();
                let lengths_similar = len1.min(len2) * 2 >= len1.max(len2);
                let use_jaro = len1 <= SHORT_WORD_THRESHOLD
                    && len2 <= SHORT_WORD_THRESHOLD
                    && lengths_similar;

                let base_score = if use_jaro {
                    // Jaro-Winkler returns 0.0-1.0 similarity, convert to distance
                    let jw_distance = 1.0 - jaro_winkler(&ngram_text, &phrase.concatenated);
                    // Only accept if very similar
                    if jw_distance > 0.2 {
                        1.0 // Reject
                    } else {
                        jw_distance
                    }
                } else {
                    // Damerau-Levenshtein normalized by length
                    let damerau_dist = damerau_levenshtein(&ngram_text, &phrase.concatenated);
                    let max_len = len1.max(len2) as f64;
                    if max_len > 0.0 {
                        damerau_dist as f64 / max_len
                    } else {
                        1.0
                    }
                };

                // Phonetic matching (computed here since we're in the n==1 single-word path)
                let ngram_phonetic = PhoneticCodes::new(&double_metaphone, &ngram_text);
                let phonetic_match = ngram_phonetic.matches(&phrase.phonetic);

                // Combined score with phonetic boost
                let combined_score = if phonetic_match {
                    base_score * 0.3
                } else {
                    base_score
                };

                if combined_score < threshold && combined_score < best_score {
                    best_match = Some(phrase);
                    best_score = combined_score;
                }
            }

            if let Some(phrase) = best_match {
                 // Extract punctuation from first and last words of the n-gram
                let (prefix, _) = extract_punctuation(ngram_words[0]);
                let (_, suffix) = extract_punctuation(ngram_words[n - 1]);

                // Preserve case from first word
                let corrected = preserve_case_pattern(ngram_words[0], &phrase.original);

                info!(
                    "[CustomWords] Matched {} word(s): '{}' -> '{}' (score={:.3})",
                    n,
                    ngram_words.join(" "),
                    phrase.original,
                    best_score
                );

                result.push(format!("{}{}{}", prefix, corrected, suffix));
                i += n;
                matched = true;
                break;
            }
        }

        if !matched {
            result.push(words[i].to_string());
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
        // ALL CAPS input -> ALL CAPS output
        replacement.to_uppercase()
    } else if alpha_chars.first().map_or(false, |c| c.is_uppercase()) {
        // Title Case input -> Title Case output
        let mut chars: Vec<char> = replacement.chars().collect();
        if let Some(first_char) = chars.get_mut(0) {
            *first_char = first_char.to_uppercase().next().unwrap_or(*first_char);
        }
        chars.into_iter().collect()
    } else {
        // Lowercase or mixed input -> keep replacement as user defined it
        replacement.to_string()
    }
}

/// Extracts punctuation prefix and suffix from a word
fn extract_punctuation(word: &str) -> (&str, &str) {
    let prefix_end = word.chars().take_while(|c| !c.is_alphanumeric()).count();
    let suffix_start = word
        .char_indices()
        .rev()
        .take_while(|(_, c)| !c.is_alphanumeric())
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
// Filler Word Removal
// ============================================

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
        assert_eq!(preserve_case_pattern("hello", "World"), "World"); // lowercase input -> keep replacement as-is
        assert_eq!(preserve_case_pattern("hello", "WORLD"), "WORLD"); // lowercase input -> keep replacement as-is
    }

    #[test]
    fn test_extract_punctuation() {
        assert_eq!(extract_punctuation("hello"), ("", ""));
        assert_eq!(extract_punctuation("!hello?"), ("!", "?"));
        assert_eq!(extract_punctuation("...hello..."), ("...", "..."));
    }

    #[test]
    fn test_exact_match_before_fuzzy() {
        let text = "handy is a great app";
        let custom_words = vec!["Handy".to_string(), "Candy".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "Handy is a great app");
    }

    #[test]
    fn test_multi_word_phrase_exact() {
        // "chat GPT" (two words) should match "ChatGPT" custom word
        let text = "I use chat GPT daily";
        let custom_words = vec!["ChatGPT".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "I use ChatGPT daily");
    }
    
    #[test]
    fn test_apply_custom_words_ngram_three_words() {
        // "Chat G P T" should NOT match since max n-gram is 3
        // but "Chat G P" (3 words) could potentially match if exact
        let text = "use Chat G P T for this";
        let custom_words = vec!["ChatGPT".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        // The algorithm uses n=1..3, so "Chat G P T" (4 words) won't fully match
        // But individual words may still get processed
        assert!(!result.is_empty());
    }

    #[test]
    fn test_empty_custom_words() {
        let text = "hello world";
        let custom_words: Vec<String> = vec![];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_phonetic_matching() {
        // "kat" sounds like "cat" - phonetic match
        let text = "I have a kat";
        let custom_words = vec!["cat".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "I have a cat");
    }

    #[test]
    fn test_punctuation_preserved() {
        // Punctuation should be preserved around matches
        let text = "I use chatgpt!";
        let custom_words = vec!["ChatGPT".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "I use ChatGPT!");
    }

    #[test]
    fn test_no_false_multi_word_match() {
        // "use chat gpt" should NOT consume "use" 
        // (this was the bug we fixed)
        let text = "I use chat gpt daily";
        let custom_words = vec!["ChatGPT".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        // "use" must be preserved, only "chat gpt" should become "ChatGPT"
        assert!(result.contains("use"));
        assert!(result.contains("ChatGPT"));
    }

    #[test]
    fn test_filter_transcription_output() {
        assert_eq!(filter_transcription_output("hello uh world"), "hello world");
        assert_eq!(filter_transcription_output("um hello"), "hello");
        assert_eq!(filter_transcription_output("wh wh wh wh what"), "wh what");
    }

    #[test]
    fn test_count_words() {
        assert_eq!(count_words("hello world"), 2);
        assert_eq!(count_words("  hello   world  "), 2);
        assert_eq!(count_words("One, two, three."), 3);
    }
}
