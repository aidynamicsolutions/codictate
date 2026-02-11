use tracing::{info, debug};
use rphonetic::{DoubleMetaphone, Encoder};
use strsim::{damerau_levenshtein, jaro_winkler};
use unicode_segmentation::UnicodeSegmentation;
use once_cell::sync::Lazy;
use regex::Regex;
use crate::settings::CustomWordEntry;

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

/// Represents a custom word entry prepared for matching
#[derive(Debug, Clone)]
struct CustomPhrase {
    entry: CustomWordEntry,
    /// Phonetic codes of concatenated input (for fuzzy matching)
    phonetic: PhoneticCodes,
    /// Concatenated lowercase input for matching
    concatenated_input: String,
}

impl CustomPhrase {
    fn new(entry: CustomWordEntry, encoder: &DoubleMetaphone) -> Self {
        // Use input for matching. If input is empty (shouldn't happen ideally), fallback to replacement
        let match_text = if entry.input.trim().is_empty() {
            &entry.replacement
        } else {
            &entry.input
        };

        let normalized = match_text.to_lowercase();
        // Remove spaces and non-alphanumeric characters for "concatenated" form
        let concatenated_input: String = normalized
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect();
            
        let phonetic = PhoneticCodes::new(encoder, &concatenated_input);

        Self {
            entry,
            phonetic,
            concatenated_input,
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

/// Applies custom word corrections to transcribed text using fuzzy or strict matching
/// 
/// "Best of Both Worlds" Implementation:
/// - Uses N-gram sliding window (from main) to catch split phrases ("Chat G P T" -> "ChatGPT")
/// - Uses Double Metaphone + Jaro-Winkler/Levenshtein (from llm) for superior accuracy
pub fn apply_custom_words(text: &str, custom_words: &[CustomWordEntry], threshold: f64) -> String {
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
        .map(|w| CustomPhrase::new(w.clone(), &double_metaphone))
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
        for n in (1..=3).rev() {
            if i + n > words.len() {
                continue;
            }

            let ngram_words = &words[i..i + n];
            let ngram_text = build_ngram(ngram_words);
            
            if ngram_text.is_empty() || ngram_text.len() > 50 {
                continue;
            }

            debug!("[CustomWords] Checking n-gram: '{}' (n={})", ngram_words.join(" "), n);

            let mut best_match: Option<&CustomPhrase> = None;
            let mut best_score = f64::MAX;

            // Check against all custom phrases
            for phrase in &phrases {
                // 1. Exact match check
                // For is_replacement=true: case-insensitive (btw, Btw, BTW all match)
                // For vocabulary entries: case-sensitive exact match
                let is_exact_match = if phrase.entry.is_replacement {
                    ngram_text.eq_ignore_ascii_case(&phrase.concatenated_input)
                } else {
                    ngram_text == phrase.concatenated_input
                };
                
                if is_exact_match {
                    // Exact match is perfect
                    best_match = Some(phrase);
                    best_score = 0.0;
                    break; // Can't beat exact match
                }

                // 2. Fuzzy match checks (Skip if is_replacement is TRUE - exact only)
                if phrase.entry.is_replacement {
                    debug!("[CustomWords] Skipping fuzzy: is_replacement=true for '{}'", phrase.entry.input);
                    continue;
                }

                // Allow fuzzy matching for multi-word n-grams ONLY if the entry itself
                // is also multi-word (and word counts match). This prevents single-word
                // entries like "ChatGPT" from incorrectly matching multi-word phrases
                // like "use chat gpt", while allowing corrections like "super wisper".
                let entry_word_count = phrase.entry.input.split_whitespace().count();
                if n != entry_word_count {
                    debug!("[CustomWords] Skipping fuzzy: word count {} != {}", n, entry_word_count);
                    continue;
                }
                
                // Skip if lengths are too different (optimization + prevents over-matching)
                let len_diff = (ngram_text.len() as i32 - phrase.concatenated_input.len() as i32).abs();
                if len_diff > 5 {
                     continue;
                }

                // Calculate string similarity score
                // Use Jaro-Winkler for short strings (better for names/prefixes)
                // Use Damerau-Levenshtein for longer strings (better for transpositions)
                let len1 = ngram_text.len();
                let len2 = phrase.concatenated_input.len();
                let lengths_similar = len1.min(len2) * 2 >= len1.max(len2);
                let use_jaro = len1 <= SHORT_WORD_THRESHOLD
                    && len2 <= SHORT_WORD_THRESHOLD
                    && lengths_similar;

                let base_score = if use_jaro {
                    // Jaro-Winkler returns 0.0-1.0 similarity, convert to distance
                    let jw_distance = 1.0 - jaro_winkler(&ngram_text, &phrase.concatenated_input);
                    // Only accept if very similar
                    if jw_distance > 0.2 {
                        1.0 // Reject
                    } else {
                        jw_distance
                    }
                } else {
                    // Damerau-Levenshtein normalized by length
                    let damerau_dist = damerau_levenshtein(&ngram_text, &phrase.concatenated_input);
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

                debug!(
                    "[CustomWords] Score for '{}' vs '{}': base={:.3}, phonetic={}, combined={:.3}",
                    ngram_text, phrase.entry.input, base_score, phonetic_match, combined_score
                );

                if combined_score < threshold && combined_score < best_score {
                    best_match = Some(phrase);
                    best_score = combined_score;
                }
            }

            if let Some(phrase) = best_match {
                 // Extract punctuation from first and last words of the n-gram
                let (prefix, _) = extract_punctuation(ngram_words[0]);
                let (_, suffix) = extract_punctuation(ngram_words[n - 1]);

                // Determine replacement text
                // If is_replacement=true: Use exact replacement (e.g. "btw" -> "by the way")
                // If is_replacement=false (vocabulary/fuzzy): preserve case pattern from input
                
                let corrected = if phrase.entry.is_replacement {
                    // Replacement mode: Adapt case based on input pattern
                    // btw → by the way, Btw → By the way, BTW → BY THE WAY
                    let input_combined: String = ngram_words.join("");
                    let first_char = input_combined.chars().next();
                    let is_all_upper = input_combined.chars().all(|c| !c.is_alphabetic() || c.is_uppercase());
                    let is_title_case = first_char.map_or(false, |c| c.is_uppercase()) && !is_all_upper;
                    
                    if is_all_upper && input_combined.chars().any(|c| c.is_alphabetic()) {
                        // ALL CAPS input → ALL CAPS output
                        phrase.entry.replacement.to_uppercase()
                    } else if is_title_case {
                        // Title Case input → Capitalize first letter of output
                        let mut chars: Vec<char> = phrase.entry.replacement.chars().collect();
                        if let Some(first_char) = chars.get_mut(0) {
                            *first_char = first_char.to_uppercase().next().unwrap_or(*first_char);
                        }
                        chars.into_iter().collect()
                    } else {
                        // lowercase input → use replacement as-is
                        phrase.entry.replacement.clone()
                    }
                } else {
                    // Vocabulary/Fuzzy mode: preserve case pattern from input
                    preserve_case_pattern(ngram_words[0], &phrase.entry.replacement)
                };

                info!(
                    "[CustomWords] Matched {} word(s): '{}' -> '{}' (score={:.3})",
                    n,
                    ngram_words.join(" "),
                    phrase.entry.replacement,
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

/// Strips leading and trailing punctuation from a word, returning only the
/// alphanumeric core. Used so that "call," compares equal to "call".
fn strip_punctuation(word: &str) -> &str {
    word.trim_matches(|c: char| !c.is_alphanumeric())
}

/// Collapse 3+ consecutive identical words (case-insensitive).
/// If the next word starts with the repeated word (e.g. "cont" → "continue"),
/// all copies are dropped since the next word is the completed form.
/// Otherwise, one copy is kept.
fn collapse_stutters(text: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return String::new();
    }

    let mut result: Vec<&str> = Vec::new();
    let mut i = 0;

    while i < words.len() {
        let word = words[i];
        let word_lower = strip_punctuation(word).to_lowercase();

        // Count consecutive repetitions (case-insensitive, ignoring punctuation)
        let mut count = 1;
        while i + count < words.len()
            && strip_punctuation(words[i + count]).to_lowercase() == word_lower
        {
            count += 1;
        }

        if count >= 3 {
            // Check if the next word after the repeated sequence starts with the
            // repeated word — if so, it's the "completed" form and we drop all copies
            let next_idx = i + count;
            if next_idx < words.len() {
                let next_lower = strip_punctuation(words[next_idx]).to_lowercase();
                if next_lower.starts_with(&word_lower) && next_lower != word_lower {
                    // Drop all copies; the next word is the completed form
                } else {
                    // Keep one copy
                    result.push(word);
                }
            } else {
                // Keep one copy
                result.push(word);
            }
            i += count;
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

/// Filters transcription output by removing filler words only.
/// Stutter/repetition handling is in `filter_hallucinations()`.
pub fn filter_transcription_output(text: &str) -> String {
    let mut filtered = text.to_string();

    // Remove filler words
    for pattern in FILLER_PATTERNS.iter() {
        filtered = pattern.replace_all(&filtered, "").to_string();
    }

    // Clean up multiple spaces to single space
    filtered = MULTI_SPACE_PATTERN.replace_all(&filtered, " ").to_string();

    // Trim leading/trailing whitespace
    filtered.trim().to_string()
}

/// Collapse progressive self-correction patterns where consecutive short fragments
/// build up to a longer target word.
/// e.g., "dr f fu fuzzy" → "fuzzy" (speaker's multiple attempts at saying a word)
///
/// Algorithm: for each long word (≥ 4 chars), look backwards for consecutive short
/// words (≤ 2 chars). If 2+ of those short words are prefixes of the long word,
/// drop the entire fragment run. This avoids capturing unrelated short words that
/// aren't adjacent to the target.
fn collapse_self_corrections(text: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() < 3 {
        return text.to_string();
    }

    const MAX_FRAGMENT_LEN: usize = 2;
    const MIN_TARGET_LEN: usize = 4;
    const MIN_PREFIX_MATCHES: usize = 2;
    // We scan backwards for contiguous prefix matches. 
    // This replaces MAX_LOOKBACK with a dynamic window that only includes valid fragments.

    // Mark which words should be dropped
    let mut drop = vec![false; words.len()];

    for j in 0..words.len() {
        // Only consider long words as potential targets (strip punctuation for length check)
        let target_stripped = strip_punctuation(words[j]);
        if target_stripped.len() < MIN_TARGET_LEN {
            continue;
        }

        let target_lower = target_stripped.to_lowercase();
        
        // Walk backwards looking for contiguous prefix matches
        let mut k = j;
        let mut matches = Vec::new();
        
        while k > 0 {
            let prev_idx = k - 1;
            let fragment = strip_punctuation(words[prev_idx]);
            
            // Check constraints:
            // 1. Must be short
            if fragment.len() > MAX_FRAGMENT_LEN {
                break; 
            }
            // 2. Must be a prefix
            if !target_lower.starts_with(&fragment.to_lowercase()) {
                break;
            }
            // 3. Must be a "fresh" word (not already dropped by a previous target catch)
            // Although scanning strict contiguous blocks usually avoids overlap issues.
            
            matches.push(prev_idx);
            k -= 1;
        }

        if matches.len() >= MIN_PREFIX_MATCHES {
            for match_idx in matches {
                drop[match_idx] = true;
            }
        }
    }

    let result: Vec<&str> = words
        .iter()
        .enumerate()
        .filter(|(i, _)| !drop[*i])
        .map(|(_, w)| *w)
        .collect();

    result.join(" ")
}

/// Filters hallucination artifacts:
/// 1. Collapses 3+ consecutive identical repeated words
/// 2. Collapses progressive self-correction fragments (e.g., "dr f fu fuzzy" → "fuzzy")
pub fn filter_hallucinations(text: &str) -> String {
    let mut filtered = collapse_stutters(text);
    filtered = collapse_self_corrections(&filtered);

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

    /// Vocabulary entry: fuzzy matching enabled (is_replacement=false)
    fn vocabulary(input: &str, replacement: &str) -> CustomWordEntry {
        CustomWordEntry {
            input: input.to_string(),
            replacement: replacement.to_string(),
            is_replacement: false, // Fuzzy matching enabled
        }
    }

    /// Exact replacement entry: exact match only (is_replacement=true)
    fn exact_replacement(input: &str, replacement: &str) -> CustomWordEntry {
        CustomWordEntry {
            input: input.to_string(),
            replacement: replacement.to_string(),
            is_replacement: true, // Exact match only
        }
    }

    #[test]
    fn test_apply_custom_words_exact_match() {
        let text = "hello world";
        let custom_words = vec![
            vocabulary("Hello", "Hello"),
            vocabulary("World", "World"),
        ];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_apply_custom_words_fuzzy_match() {
        let text = "helo wrold";
        let custom_words = vec![
            vocabulary("hello", "hello"),
            vocabulary("world", "world"),
        ];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_strict_replacement() {
        let text = "btw I am busy";
        let custom_words = vec![
            exact_replacement("btw", "by the way"),
        ];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "by the way I am busy");
    }

    #[test]
    fn test_strict_replacement_case_handling() {
        // "Btw" -> "By the way" (Title Case)
        let text = "Btw I am busy";
        let custom_words = vec![
            exact_replacement("btw", "by the way"),
        ];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "By the way I am busy");
    }

    #[test]
    fn test_strict_replacement_all_caps() {
        // "BTW" -> "BY THE WAY" (ALL CAPS)
        let text = "BTW I am busy";
        let custom_words = vec![
            exact_replacement("btw", "by the way"),
        ];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "BY THE WAY I am busy");
    }

    #[test]
    fn test_strict_replacement_no_fuzzy() {
        // "bt" should NOT match "btw" strictly
        let text = "bt I am busy";
        let custom_words = vec![
            exact_replacement("btw", "by the way"),
        ];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "bt I am busy");
    }

    #[test]
    fn test_correction_fuzzy() {
        // "super wisper" -> "SuperWhisper"
        let text = "I use super wisper";
        let custom_words = vec![
            vocabulary("super whisper", "SuperWhisper"),
        ];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "I use SuperWhisper");
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
        let custom_words = vec![
            vocabulary("Handy", "Handy"), 
            vocabulary("Candy", "Candy")
        ];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "Handy is a great app");
    }

    #[test]
    fn test_multi_word_phrase_exact() {
        // "chat GPT" (two words) should match "ChatGPT" custom word
        let text = "I use chat GPT daily";
        let custom_words = vec![
            vocabulary("ChatGPT", "ChatGPT"),
        ];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "I use ChatGPT daily");
    }

    #[test]
    fn test_multi_word_replacement() {
        // UI example: "chat gpt" → "ChatGPT" with "Replace with different text" toggle ON
        // This is exact replacement mode (is_replacement=true), not fuzzy vocabulary mode
        let text = "I use chat gpt daily";
        let custom_words = vec![
            exact_replacement("chat gpt", "ChatGPT"),
        ];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "I use ChatGPT daily");
    }
    
    #[test]
    fn test_apply_custom_words_ngram_three_words() {
        // "ChatGPT" is a single-word entry, so fuzzy matching only works for 1-word n-grams.
        // "Chat" is close enough to "ChatGPT" for a fuzzy match, but "G", "P", "T" are not.
        // Multi-word n-gram fuzzy matching is blocked because word counts don't match (3 != 1).
        let text = "use Chat G P T for this";
        let custom_words = vec![
            vocabulary("ChatGPT", "ChatGPT"),
        ];
        let result = apply_custom_words(text, &custom_words, 0.5);
        // Only "Chat" gets matched to "ChatGPT", "G P T" remain unchanged
        assert_eq!(result, "use ChatGPT G P T for this");
    }

    #[test]
    fn test_empty_custom_words() {
        let text = "hello world";
        let custom_words: Vec<CustomWordEntry> = vec![];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_phonetic_matching() {
        // "kat" sounds like "cat" - phonetic match
        let text = "I have a kat";
        let custom_words = vec![
            vocabulary("cat", "cat"),
        ];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "I have a cat");
    }

    #[test]
    fn test_punctuation_preserved() {
        // Punctuation should be preserved around matches
        let text = "I use chatgpt!";
        let custom_words = vec![
            vocabulary("ChatGPT", "ChatGPT"),
        ];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "I use ChatGPT!");
    }

    #[test]
    fn test_no_false_multi_word_match() {
        // "use chat gpt" should NOT consume "use" 
        // (this was the bug we fixed)
        let text = "I use chat gpt daily";
        let custom_words = vec![
            vocabulary("ChatGPT", "ChatGPT"),
        ];
        let result = apply_custom_words(text, &custom_words, 0.5);
        // "use" must be preserved, only "chat gpt" should become "ChatGPT"
        assert!(result.contains("use"));
        assert!(result.contains("ChatGPT"));
    }

    #[test]
    fn test_filter_transcription_output() {
        assert_eq!(filter_transcription_output("hello uh world"), "hello world");
        assert_eq!(filter_transcription_output("um hello"), "hello");
    }

    #[test]
    fn test_filter_hallucinations_example1() {
        // "cont cont cont ... continue" → "continue" (prefix match: all copies dropped)
        assert_eq!(
            filter_hallucinations("so I can cont cont cont cont cont cont cont cont cont continue debugging"),
            "so I can continue debugging"
        );
    }

    #[test]
    fn test_filter_hallucinations_example2() {
        // "dr f fu fuzzy" → "dr fuzzy" (progressive self-correction: matching fragments collapse)
        // "dr" is NOT a prefix of "fuzzy", so it is preserved.
        assert_eq!(
            filter_hallucinations("the correction is dr f fu fuzzy matching"),
            "the correction is dr fuzzy matching"
        );
    }

    #[test]
    fn test_filter_hallucinations_example3() {
        // "sim sim sim ... similar" → "similar" (prefix match: all copies dropped)
        assert_eq!(
            filter_hallucinations("similar sim sim sim sim sim sim sim sim sim similar to how"),
            "similar similar to how"
        );
    }

    #[test]
    fn test_filter_hallucinations_example4() {
        // "call call call call call call call call call call call it" -> "call it"
        assert_eq!(
            filter_hallucinations("sidebar, call call call call call call call call call call call it dictionary?"),
            "sidebar, call it dictionary?"
        );
    }

    #[test]
    fn test_filter_hallucinations_example5() {
        // "three three three three three three three three three three three" -> "three"
        assert_eq!(
            filter_hallucinations("one three three three three three three three three three three three"),
            "one three"
        );
    }

    #[test]
    fn test_filter_hallucinations_short_stutter() {
        // "wh wh wh wh what" → "what" (prefix match: "wh" starts "what")
        assert_eq!(
            filter_hallucinations("wh wh wh wh what"),
            "what"
        );
    }

    #[test]
    fn test_filter_hallucinations_no_repeats() {
        // No repeated words - text should remain unchanged
        assert_eq!(
            filter_hallucinations("hello world how are you"),
            "hello world how are you"
        );
    }

    #[test]
    fn test_filter_hallucinations_exactly_two_repeats() {
        // Only 2 repeats - should NOT collapse (threshold is 3)
        assert_eq!(
            filter_hallucinations("the the cat"),
            "the the cat"
        );
    }

    #[test]
    fn test_filter_hallucinations_case_insensitive() {
        // Mixed case, next word "thing" does NOT start with "the" → keep one
        assert_eq!(
            filter_hallucinations("The the THE the thing"),
            "The thing"
        );
    }

    #[test]
    fn test_self_correction_normal_short_words_preserved() {
        // "I am" before "amazing" — only 1 prefix match ("am"), threshold is 2
        assert_eq!(
            filter_hallucinations("I am amazing"),
            "I am amazing"
        );
    }

    #[test]
    fn test_self_correction_single_fragment_no_collapse() {
        // Only 1 short word before target — need at least 2
        assert_eq!(
            filter_hallucinations("to tomorrow"),
            "to tomorrow"
        );
    }

    #[test]
    fn test_self_correction_progressive_prefix() {
        // "b bu buzz" — both short words are prefixes of "buzz"
        assert_eq!(
            filter_hallucinations("say b bu buzz"),
            "say buzz"
        );
    }

    #[test]
    fn test_self_correction_mixed_with_repetition() {
        // Identical repetition + self-correction in same text
        assert_eq!(
            filter_hallucinations("I cont cont cont continue and dr f fu fuzzy works"),
            "I continue and dr fuzzy works"
        );
    }

    #[test]
    fn test_bug_fix_unrelated_short_words() {
        // "I am a f fu fuzzy" -> "I am a fuzzy"
        // "a" is a short word but NOT a prefix of "fuzzy", so it must be preserved.
        // The old buggy implementation deleted "a".
        // Also verify contiguous prefix logic: "f fu" are dropped, "a" breaks the chain.
        assert_eq!(
            filter_hallucinations("I am a f fu fuzzy bear"),
            "I am a fuzzy bear"
        );
    }

    #[test]
    fn test_self_correction_target_too_short() {
        // Target word "and" is only 3 chars (< 4 minimum) — no collapse
        assert_eq!(
            filter_hallucinations("a an and"),
            "a an and"
        );
    }

    #[test]
    fn test_count_words() {
        assert_eq!(count_words("hello world"), 2);
        assert_eq!(count_words("  hello   world  "), 2);
        assert_eq!(count_words("One, two, three."), 3);
    }
}
