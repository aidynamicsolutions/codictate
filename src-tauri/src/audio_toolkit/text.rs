use crate::settings::CustomWordEntry;
use std::sync::LazyLock;
use regex::Regex;
use rphonetic::{DoubleMetaphone, Encoder};
use std::collections::{HashMap, HashSet};
use strsim::{damerau_levenshtein, jaro_winkler};
use tracing::{debug, info};
use unicode_segmentation::UnicodeSegmentation;

/// Threshold for using Jaro-Winkler vs Damerau-Levenshtein
/// Jaro-Winkler is better for short strings due to prefix emphasis
const SHORT_WORD_THRESHOLD: usize = 6;
/// For short words with different lengths, require stronger Jaro-Winkler similarity.
/// This reduces false positives such as "mode" -> "modal" caused by prefix bias.
const SHORT_WORD_JARO_MISMATCH_MAX_DISTANCE: f64 = 0.12;

/// Minimum input length for fuzzy matching (inclusive).
/// Inputs with ≤ this many characters require exact match.
/// Mirrors Elasticsearch's `fuzziness: "AUTO"` which requires exact match for 0-2 char terms.
const MIN_FUZZY_LENGTH: usize = 3;

/// Minimum length ratio (shorter/longer) for fuzzy matching.
/// Reject candidates where the shorter string is less than this fraction of the longer.
/// Value 0.60 means shorter must be ≥ 60% of longer (≈1.67x max ratio).
/// Aligned with FuzzyWuzzy/RapidFuzz which auto-switch at 1.5x.
const MIN_LENGTH_RATIO: f64 = 0.60;

/// Default threshold for split-token fuzzy matching.
/// Stricter than the generic threshold to reduce false positives.
const DEFAULT_SPLIT_THRESHOLD: f64 = 0.14;

/// Minimum normalized length for split-token fuzzy candidates.
const MIN_SPLIT_FUZZY_LENGTH: usize = 5;

/// Maximum n-gram length for exact matching.
/// Keep this bounded to avoid pathological O(n*k*m) scans on very long dictation.
const MAX_EXACT_NGRAM_WORDS: usize = 8;

/// Maximum n-gram length for fuzzy matching.
/// Fuzzy matching on long phrases tends to increase false positives and cost.
const MAX_FUZZY_NGRAM_WORDS: usize = 3;

use crate::audio_toolkit::stop_words::{FUZZY_GUARD_WORDS, SELF_CORRECTION_PROTECTED_SHORT_WORDS};

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
    /// The source string used for matching (canonical input or alias).
    source_text: String,
    /// Number of words in `source_text`.
    source_word_count: usize,
    /// True when `source_text` is an alias.
    is_alias: bool,
    /// Phonetic codes of concatenated input (for fuzzy matching)
    phonetic: PhoneticCodes,
    /// Concatenated lowercase input for matching
    concatenated_input: String,
}

impl CustomPhrase {
    fn from_entry(entry: &CustomWordEntry, encoder: &DoubleMetaphone) -> Vec<Self> {
        let mut candidates: Vec<(String, bool)> = vec![(entry.input.clone(), false)];
        for alias in &entry.aliases {
            candidates.push((alias.clone(), true));
        }

        let mut seen = HashSet::new();
        candidates
            .into_iter()
            .filter_map(|(source_text, is_alias)| {
                Self::new(entry.clone(), source_text, is_alias, encoder).and_then(|phrase| {
                    if seen.insert(phrase.concatenated_input.clone()) {
                        Some(phrase)
                    } else {
                        None
                    }
                })
            })
            .collect()
    }

    fn new(
        entry: CustomWordEntry,
        source_text: String,
        is_alias: bool,
        encoder: &DoubleMetaphone,
    ) -> Option<Self> {
        let source_word_count = source_text.split_whitespace().count();
        if source_word_count == 0 {
            return None;
        }

        let concatenated_input = normalize_for_matching(&source_text);
        if concatenated_input.is_empty() {
            return None;
        }

        let phonetic = PhoneticCodes::new(encoder, &concatenated_input);

        Some(Self {
            entry,
            source_text,
            source_word_count,
            is_alias,
            phonetic,
            concatenated_input,
        })
    }
}

#[derive(Debug, Default)]
struct MatchStats {
    candidates_checked: usize,
    exact_hits: usize,
    split_fuzzy_hits: usize,
    standard_fuzzy_hits: usize,
    reject_counts: HashMap<&'static str, usize>,
}

impl MatchStats {
    fn reject(&mut self, reason: &'static str) {
        *self.reject_counts.entry(reason).or_insert(0) += 1;
    }
}



/// Builds an n-gram string by cleaning and concatenating words
///
/// Strips punctuation from each word, lowercases, and joins without spaces.
/// This allows matching "Charge B" against "ChargeBee".
fn build_ngram(words: &[&str]) -> String {
    normalize_for_matching(&words.join(" "))
}



/// Normalizes text into a comparison key for dictionary matching.
///
/// - Lowercases
/// - Expands common programming symbols (`+`, `#`, `&`) into spoken forms
/// - Drops punctuation and whitespace by collapsing to alphanumeric-only text
///
/// This keeps exact matching consistent between dictionary entries and transcript n-grams,
/// including punctuated terms like "don't", "node.js", and "c++".
fn normalize_for_matching(text: &str) -> String {
    let lower = text.to_lowercase();
    let mut expanded = String::with_capacity(lower.len() + 8);

    for ch in lower.chars() {
        match ch {
            '+' => expanded.push_str(" plus "),
            '#' => expanded.push_str(" sharp "),
            '&' => expanded.push_str(" and "),
            c if c.is_alphanumeric() || c.is_whitespace() => expanded.push(c),
            _ => expanded.push(' '),
        }
    }

    expanded.chars().filter(|c| c.is_alphanumeric()).collect()
}

fn is_one_char_prefix_extension(a: &str, b: &str) -> bool {
    let (shorter, longer) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    longer.len() == shorter.len() + 1 && longer.starts_with(shorter)
}

fn ngram_all_guard_words(words: &[&str]) -> bool {
    let mut saw_token = false;
    for token in words.iter().map(|w| normalize_for_matching(w)) {
        if token.is_empty() {
            continue;
        }
        saw_token = true;
        if !FUZZY_GUARD_WORDS.contains(token.as_str()) {
            return false;
        }
    }
    saw_token
}

fn is_cplusplus_like_phrase(phrase: &CustomPhrase) -> bool {
    phrase.entry.replacement.contains("++") || phrase.concatenated_input.ends_with("plusplus")
}

/// Consumes trailing "plus" tokens after a C++-style exact/fuzzy match when
/// they appear to be ASR overrun before a guard word or sentence end.
fn maybe_expand_cplusplus_match(
    words: &[&str],
    start_index: usize,
    matched_word_count: usize,
    phrase: &CustomPhrase,
) -> usize {
    if !is_cplusplus_like_phrase(phrase) {
        return matched_word_count;
    }

    let mut idx = start_index + matched_word_count;
    let mut plus_count = 0usize;
    while idx < words.len() {
        let token = normalize_for_matching(words[idx]);
        if token == "plus" {
            plus_count += 1;
            idx += 1;
        } else {
            break;
        }
    }

    if plus_count == 0 {
        return matched_word_count;
    }

    if idx == words.len() {
        return matched_word_count + plus_count;
    }

    let next = normalize_for_matching(words[idx]);
    if next.is_empty() || FUZZY_GUARD_WORDS.contains(next.as_str()) {
        matched_word_count + plus_count
    } else {
        matched_word_count
    }
}

/// Applies custom word corrections to transcribed text using fuzzy or strict matching
///
/// "Best of Both Worlds" Implementation:
/// - Uses N-gram sliding window (from main) to catch split phrases ("Chat G P T" -> "ChatGPT")
/// - Uses Double Metaphone + Jaro-Winkler/Levenshtein (from llm) for superior accuracy
pub fn apply_custom_words(text: &str, custom_words: &[CustomWordEntry], threshold: f64) -> String {
    apply_custom_words_with_thresholds(text, custom_words, threshold, DEFAULT_SPLIT_THRESHOLD)
}

pub fn apply_custom_words_with_thresholds(
    text: &str,
    custom_words: &[CustomWordEntry],
    threshold: f64,
    split_threshold: f64,
) -> String {
    if custom_words.is_empty() {
        return text.to_string();
    }

    // TODO: extend to N-best rescoring when ASR engines expose alternatives
    apply_custom_words_to_hypothesis(text, custom_words, threshold, split_threshold)
}

fn apply_custom_words_to_hypothesis(
    text: &str,
    custom_words: &[CustomWordEntry],
    threshold: f64,
    split_threshold: f64,
) -> String {
    info!(
        "[CustomWords] Processing: '{}' with {} custom words, threshold={}, split_threshold={}",
        text,
        custom_words.len(),
        threshold,
        split_threshold
    );

    // Pre-compute phrase data
    let double_metaphone = DoubleMetaphone::default();
    let phrases: Vec<CustomPhrase> = custom_words
        .iter()
        .flat_map(|w| CustomPhrase::from_entry(w, &double_metaphone))
        .collect();
    let mut stats = MatchStats::default();

    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return text.to_string();
    }

    let max_exact_ngram = MAX_EXACT_NGRAM_WORDS.min(words.len());
    let max_fuzzy_ngram = MAX_FUZZY_NGRAM_WORDS.min(max_exact_ngram);

    let mut result: Vec<String> = Vec::new();
    let mut i = 0;

    while i < words.len() {
        let mut matched = false;

        // Exact matching is always evaluated before any fuzzy path.
        let mut exact_match: Option<(usize, &CustomPhrase, &'static str)> = None;
        for n in (1..=max_exact_ngram).rev() {
            if i + n > words.len() {
                continue;
            }

            let ngram_words = &words[i..i + n];
            let ngram_text = build_ngram(ngram_words);
            if ngram_text.is_empty() || ngram_text.len() > 50 {
                continue;
            }

            for phrase in &phrases {
                stats.candidates_checked += 1;
                let is_exact_match = if phrase.entry.is_replacement {
                    ngram_text.eq_ignore_ascii_case(&phrase.concatenated_input)
                } else {
                    ngram_text == phrase.concatenated_input
                };

                if is_exact_match {
                    let reason = if phrase.is_alias {
                        "exact_alias_match"
                    } else {
                        "exact_canonical_match"
                    };
                    exact_match = Some((n, phrase, reason));
                    break;
                }
            }

            if exact_match.is_some() {
                break;
            }
        }

        if let Some((n, phrase, reason)) = exact_match {
            let ngram_words = &words[i..i + n];
            let consumed_words = maybe_expand_cplusplus_match(&words, i, n, phrase);
            let consumed_segment = &words[i..i + consumed_words];
            let (prefix, _) = extract_punctuation(consumed_segment[0]);
            let (_, suffix) = extract_punctuation(consumed_segment[consumed_words - 1]);
            let corrected = corrected_text_for_match(phrase, ngram_words);

            stats.exact_hits += 1;
            info!(
                reason = reason,
                path = "exact",
                ngram = %ngram_words.join(" "),
                n,
                entry_input = %phrase.entry.input,
                entry_alias = %phrase.source_text,
                score = 0.0,
                threshold = 0.0,
                "[CustomWords] Matched {} word(s): '{}' -> '{}' (score={:.3})",
                n,
                ngram_words.join(" "),
                phrase.entry.replacement,
                0.0
            );

            result.push(format!("{}{}{}", prefix, corrected, suffix));
            i += consumed_words;
            matched = true;
        }

        if matched {
            continue;
        }

        // Try n-grams from longest (3) to shortest (1) - greedy matching
        for n in (1..=max_fuzzy_ngram).rev() {
            if i + n > words.len() {
                continue;
            }

            let ngram_words = &words[i..i + n];
            let ngram_text = build_ngram(ngram_words);

            if ngram_text.is_empty() || ngram_text.len() > 50 {
                continue;
            }

            debug!(
                "[CustomWords] Checking n-gram: '{}' (n={})",
                ngram_words.join(" "),
                n
            );

            let mut best_match: Option<&CustomPhrase> = None;
            let mut best_score = f64::MAX;
            let mut best_reason = "reject_score";
            let mut best_path = "none";
            let mut best_threshold = threshold;

            // Check against all custom phrases
            for phrase in &phrases {
                stats.candidates_checked += 1;

                // Fuzzy match checks (skip replacement entries, exact-only).
                if phrase.entry.is_replacement {
                    stats.reject("skip_exact_only_replacement");
                    debug!(
                        reason = "skip_exact_only_replacement",
                        path = "fuzzy",
                        ngram = %ngram_text,
                        n,
                        entry_input = %phrase.entry.input,
                        entry_alias = %phrase.source_text,
                        "[CustomWords] Rejected candidate"
                    );
                    continue;
                }

                // Guard: skip fuzzy for very short inputs (Elasticsearch AUTO style)
                if ngram_text.len() <= MIN_FUZZY_LENGTH {
                    stats.reject("skip_short_input");
                    debug!(
                        reason = "skip_short_input",
                        path = "fuzzy",
                        ngram = %ngram_text,
                        n,
                        entry_input = %phrase.entry.input,
                        entry_alias = %phrase.source_text,
                        "[CustomWords] Rejected candidate"
                    );
                    continue;
                }

                let is_split_path = (2..=3).contains(&n) && phrase.source_word_count == 1;
                let active_threshold = if is_split_path {
                    split_threshold
                } else {
                    threshold
                };
                let active_path = if is_split_path {
                    "split_fuzzy"
                } else {
                    "standard_fuzzy"
                };

                if is_split_path {
                    if ngram_text.len() < MIN_SPLIT_FUZZY_LENGTH {
                        stats.reject("skip_short_input");
                        debug!(
                            reason = "skip_short_input",
                            path = active_path,
                            ngram = %ngram_text,
                            n,
                            entry_input = %phrase.entry.input,
                            entry_alias = %phrase.source_text,
                            "[CustomWords] Rejected candidate"
                        );
                        continue;
                    }
                    if ngram_all_guard_words(ngram_words) {
                        stats.reject("skip_guard_word");
                        debug!(
                            reason = "skip_guard_word",
                            path = active_path,
                            ngram = %ngram_text,
                            n,
                            entry_input = %phrase.entry.input,
                            entry_alias = %phrase.source_text,
                            "[CustomWords] Rejected candidate"
                        );
                        continue;
                    }
                    // Split-token fuzzy should target naturally split forms like
                    // "shat cn" or "chat g p", not generic phrases like "a phoen".
                    let has_short_tail_token = ngram_words
                        .iter()
                        .skip(1)
                        .map(|w| normalize_for_matching(w))
                        .any(|token| !token.is_empty() && token.len() <= 2);
                    if !has_short_tail_token {
                        stats.reject("skip_word_count");
                        debug!(
                            reason = "skip_word_count",
                            path = active_path,
                            ngram = %ngram_text,
                            n,
                            entry_input = %phrase.entry.input,
                            entry_alias = %phrase.source_text,
                            "[CustomWords] Rejected candidate"
                        );
                        continue;
                    }
                } else if n != phrase.source_word_count {
                    stats.reject("skip_word_count");
                    debug!(
                        reason = "skip_word_count",
                        path = active_path,
                        ngram = %ngram_text,
                        n,
                        entry_input = %phrase.entry.input,
                        entry_alias = %phrase.source_text,
                        source_word_count = phrase.source_word_count,
                        "[CustomWords] Rejected candidate"
                    );
                    continue;
                } else if FUZZY_GUARD_WORDS.contains(ngram_text.as_str()) {
                    stats.reject("skip_guard_word");
                    debug!(
                        reason = "skip_guard_word",
                        path = active_path,
                        ngram = %ngram_text,
                        n,
                        entry_input = %phrase.entry.input,
                        entry_alias = %phrase.source_text,
                        "[CustomWords] Rejected candidate"
                    );
                    continue;
                }

                // Guard: proportional length ratio check
                // Reject when shorter string < 60% of longer (replaces fixed len_diff > 5)
                let shorter = ngram_text.len().min(phrase.concatenated_input.len());
                let longer = ngram_text.len().max(phrase.concatenated_input.len());
                if (shorter as f64) < (longer as f64) * MIN_LENGTH_RATIO {
                    stats.reject("skip_length_ratio");
                    debug!(
                        reason = "skip_length_ratio",
                        path = active_path,
                        ngram = %ngram_text,
                        n,
                        entry_input = %phrase.entry.input,
                        entry_alias = %phrase.source_text,
                        shorter,
                        longer,
                        "[CustomWords] Rejected candidate"
                    );
                    continue;
                }

                // Calculate string similarity score
                // Use Jaro-Winkler for short strings (better for names/prefixes)
                // Use Damerau-Levenshtein for longer strings (better for transpositions)
                let len1 = ngram_text.len();
                let len2 = phrase.concatenated_input.len();
                let lengths_similar = len1.min(len2) * 2 >= len1.max(len2);
                let use_jaro =
                    len1 <= SHORT_WORD_THRESHOLD && len2 <= SHORT_WORD_THRESHOLD && lengths_similar;

                let base_score = if use_jaro {
                    // Jaro-Winkler returns 0.0-1.0 similarity, convert to distance
                    let jw_distance = 1.0 - jaro_winkler(&ngram_text, &phrase.concatenated_input);
                    // Only accept if very similar
                    let max_jw_distance = if len1 == len2 {
                        0.2
                    } else {
                        SHORT_WORD_JARO_MISMATCH_MAX_DISTANCE
                    };
                    if jw_distance > max_jw_distance {
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
                let combined_score = if phonetic_match && !is_split_path {
                    base_score * 0.3
                } else {
                    base_score
                };

                debug!(
                    path = active_path,
                    ngram = %ngram_text,
                    n,
                    entry_input = %phrase.entry.input,
                    entry_alias = %phrase.source_text,
                    base_score = base_score,
                    phonetic_match = phonetic_match,
                    combined_score = combined_score,
                    threshold = active_threshold,
                    "[CustomWords] Candidate score"
                );

                // Guard against over-aggressive one-char suffix expansions
                // on short single-word candidates (e.g. "word" -> "words").
                if !is_split_path
                    && n == 1
                    && phrase.source_word_count == 1
                    && ngram_text.len().min(phrase.concatenated_input.len()) <= SHORT_WORD_THRESHOLD
                    && is_one_char_prefix_extension(&ngram_text, &phrase.concatenated_input)
                {
                    stats.reject("skip_one_char_prefix_extension");
                    debug!(
                        reason = "skip_one_char_prefix_extension",
                        path = active_path,
                        ngram = %ngram_text,
                        n,
                        entry_input = %phrase.entry.input,
                        entry_alias = %phrase.source_text,
                        "[CustomWords] Rejected candidate"
                    );
                    continue;
                }

                if combined_score < active_threshold && combined_score < best_score {
                    best_match = Some(phrase);
                    best_score = combined_score;
                    best_path = active_path;
                    best_threshold = active_threshold;
                    best_reason = if is_split_path {
                        "accept_split_fuzzy"
                    } else {
                        "accept_standard_fuzzy"
                    };
                } else {
                    stats.reject("reject_score");
                    debug!(
                        reason = "reject_score",
                        path = active_path,
                        ngram = %ngram_text,
                        n,
                        entry_input = %phrase.entry.input,
                        entry_alias = %phrase.source_text,
                        score = combined_score,
                        threshold = active_threshold,
                        "[CustomWords] Rejected candidate"
                    );
                }
            }

            if let Some(phrase) = best_match {
                // Extract punctuation from first and last words of the n-gram
                let consumed_words = maybe_expand_cplusplus_match(&words, i, n, phrase);
                let consumed_segment = &words[i..i + consumed_words];
                let (prefix, _) = extract_punctuation(consumed_segment[0]);
                let (_, suffix) = extract_punctuation(consumed_segment[consumed_words - 1]);
                let corrected = corrected_text_for_match(phrase, ngram_words);

                match best_reason {
                    "accept_split_fuzzy" => stats.split_fuzzy_hits += 1,
                    "accept_standard_fuzzy" => stats.standard_fuzzy_hits += 1,
                    _ => {}
                }

                info!(
                    reason = best_reason,
                    path = best_path,
                    ngram = %ngram_words.join(" "),
                    n,
                    entry_input = %phrase.entry.input,
                    entry_alias = %phrase.source_text,
                    score = best_score,
                    threshold = best_threshold,
                    "[CustomWords] Matched {} word(s): '{}' -> '{}' (score={:.3})",
                    n,
                    ngram_words.join(" "),
                    phrase.entry.replacement,
                    best_score
                );

                result.push(format!("{}{}{}", prefix, corrected, suffix));
                i += consumed_words;
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
    info!(
        candidates_checked = stats.candidates_checked,
        exact_hits = stats.exact_hits,
        split_fuzzy_hits = stats.split_fuzzy_hits,
        standard_fuzzy_hits = stats.standard_fuzzy_hits,
        reject_counts = ?stats.reject_counts,
        "[CustomWords] Summary"
    );
    info!("[CustomWords] Result: '{}'", output);
    output
}

fn corrected_text_for_match(phrase: &CustomPhrase, ngram_words: &[&str]) -> String {
    if phrase.entry.is_replacement {
        // Replacement mode: Adapt case based on input pattern
        // btw -> by the way, Btw -> By the way, BTW -> BY THE WAY
        let input_combined: String = ngram_words.join("");
        let first_char = input_combined.chars().next();
        let is_all_upper = input_combined
            .chars()
            .all(|c| !c.is_alphabetic() || c.is_uppercase());
        let is_title_case = first_char.map_or(false, |c| c.is_uppercase()) && !is_all_upper;

        if is_all_upper && input_combined.chars().any(|c| c.is_alphabetic()) {
            // ALL CAPS input -> ALL CAPS output
            phrase.entry.replacement.to_uppercase()
        } else if is_title_case {
            // Title Case input -> Capitalize first letter of output
            let mut chars: Vec<char> = phrase.entry.replacement.chars().collect();
            if let Some(first_char) = chars.get_mut(0) {
                *first_char = first_char.to_uppercase().next().unwrap_or(*first_char);
            }
            chars.into_iter().collect()
        } else {
            // lowercase input -> use replacement as-is
            phrase.entry.replacement.clone()
        }
    } else {
        // Vocabulary/Fuzzy mode: preserve case pattern from input for plain words,
        // but keep symbol-rich replacements literal as user-configured (e.g. "c++", "node.js").
        if should_preserve_literal_symbol_case(&phrase.entry.replacement) {
            phrase.entry.replacement.clone()
        } else {
            preserve_case_pattern(ngram_words[0], &phrase.entry.replacement)
        }
    }
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

/// For symbol-rich terms, preserve user-defined case exactly.
/// This avoids forced title/upper casing from dictated token case, which is common for
/// spelled letters in ASR (e.g. "C plus plus" starting with uppercase "C").
fn should_preserve_literal_symbol_case(replacement: &str) -> bool {
    replacement
        .chars()
        .any(|c| !c.is_alphanumeric() && !c.is_whitespace())
}

/// Extracts punctuation prefix and suffix from a word
fn extract_punctuation(word: &str) -> (&str, &str) {
    if !word.chars().any(|c| c.is_alphanumeric()) {
        return (word, "");
    }

    let mut prefix_end = 0usize;
    for (idx, ch) in word.char_indices() {
        if ch.is_alphanumeric() {
            break;
        }
        prefix_end = idx + ch.len_utf8();
    }

    let mut suffix_start = word.len();
    for (idx, ch) in word.char_indices().rev() {
        if ch.is_alphanumeric() {
            break;
        }
        suffix_start = idx;
    }

    let prefix = &word[..prefix_end];
    let suffix = &word[suffix_start..];

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

static MULTI_SPACE_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s{2,}").unwrap());

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
static FILLER_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    FILLER_WORDS
        .iter()
        .map(|word| {
            // Match filler word with word boundaries, optionally followed by comma or period
            Regex::new(&format!(r"(?i)\b{}\b[,.]?", regex::escape(word))).unwrap()
        })
        .collect()
});

/// Filters filler words and returns (filtered_text, removed_count).
/// This is the core implementation used by both `filter_transcription_output`
/// and callers that need the removal count (e.g., stats tracking).
pub fn filter_and_count_filler_words(text: &str) -> (String, usize) {
    let mut filtered = text.to_string();
    let mut count = 0;

    // Count and remove filler words
    for pattern in FILLER_PATTERNS.iter() {
        count += pattern.find_iter(&filtered).count();
        filtered = pattern.replace_all(&filtered, "").to_string();
    }

    // Clean up multiple spaces to single space
    filtered = MULTI_SPACE_PATTERN.replace_all(&filtered, " ").to_string();

    // Trim leading/trailing whitespace
    (filtered.trim().to_string(), count)
}

/// Filters transcription output by removing filler words only.
/// Stutter/repetition handling is in `filter_hallucinations()`.
pub fn filter_transcription_output(text: &str) -> String {
    filter_and_count_filler_words(text).0
}

/// Collapse progressive self-correction patterns where consecutive short fragments
/// build up to a longer target word.
/// e.g., "dr f fu fuzzy" → "fuzzy" (speaker's multiple attempts at saying a word)
/// e.g., "sh ... showing" → "showing" (distant prefix match)
///
/// Algorithm:
/// 1. Iterate through words.
/// 2. For each "target" word (len >= 3), look back up to 3 words.
/// 3. If a previous word is a short prefix (len <= 2) of the target, and NOT a protected word, drop it.
fn collapse_self_corrections(text: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() < 2 {
        return text.to_string();
    }

    let mut drop = vec![false; words.len()];

    for i in 0..words.len() {
        // Only look for corrections relative to words that are not themselves dropped
        if drop[i] {
            continue;
        }

        let target_stripped = strip_punctuation(words[i]);
        // Target must be substantive enough to be the "real" word
        if target_stripped.len() < 3 {
            continue;
        }

        let target_lower = target_stripped.to_lowercase();

        // Look back up to 3 words for hesitation fragments
        // range: 1 to 3
        for back in 1..=3 {
            if i < back {
                break;
            }
            let prev_idx = i - back;

            if drop[prev_idx] {
                continue;
            } // Already dropped

            let fragment_stripped = strip_punctuation(words[prev_idx]);

            // Constraint 1: Fragment must be short
            // "n", "f" (len 1), "sh", "dr" (len 2)
            if fragment_stripped.len() > 2 {
                continue;
            }
            if fragment_stripped.is_empty() {
                continue;
            }

            let fragment_lower = fragment_stripped.to_lowercase();

            // Constraint 2: Fragment must be a prefix of the target
            if !target_lower.starts_with(&fragment_lower) {
                continue;
            }

            // Constraint 3: Fragment must NOT be a valid preserved short word.
            // "a" -> "apple" (a is preserved). SKIP.
            // "go" -> "going" (go is preserved). SKIP.
            // "n" -> "new" (n is not preserved). DROP.
            let is_preserved =
                SELF_CORRECTION_PROTECTED_SHORT_WORDS.contains(fragment_lower.as_str());

            if is_preserved {
                continue;
            }

            // If we satisfied all constraints, mark for dropping
            drop[prev_idx] = true;
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
            aliases: Vec::new(),
            replacement: replacement.to_string(),
            is_replacement: false, // Fuzzy matching enabled
        }
    }

    /// Exact replacement entry: exact match only (is_replacement=true)
    fn exact_replacement(input: &str, replacement: &str) -> CustomWordEntry {
        CustomWordEntry {
            input: input.to_string(),
            aliases: Vec::new(),
            replacement: replacement.to_string(),
            is_replacement: true, // Exact match only
        }
    }

    fn exact_replacement_with_aliases(
        input: &str,
        aliases: &[&str],
        replacement: &str,
    ) -> CustomWordEntry {
        CustomWordEntry {
            input: input.to_string(),
            aliases: aliases.iter().map(|alias| alias.to_string()).collect(),
            replacement: replacement.to_string(),
            is_replacement: true, // Exact match only
        }
    }

    fn vocabulary_with_aliases(
        input: &str,
        aliases: &[&str],
        replacement: &str,
    ) -> CustomWordEntry {
        CustomWordEntry {
            input: input.to_string(),
            aliases: aliases.iter().map(|alias| alias.to_string()).collect(),
            replacement: replacement.to_string(),
            is_replacement: false,
        }
    }

    #[test]
    fn test_apply_custom_words_exact_match() {
        let text = "hello world";
        let custom_words = vec![vocabulary("Hello", "Hello"), vocabulary("World", "World")];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_apply_custom_words_fuzzy_match() {
        let text = "helo wrold";
        let custom_words = vec![vocabulary("hello", "hello"), vocabulary("world", "world")];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_strict_replacement() {
        let text = "btw I am busy";
        let custom_words = vec![exact_replacement("btw", "by the way")];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "by the way I am busy");
    }

    #[test]
    fn test_strict_replacement_case_handling() {
        // "Btw" -> "By the way" (Title Case)
        let text = "Btw I am busy";
        let custom_words = vec![exact_replacement("btw", "by the way")];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "By the way I am busy");
    }

    #[test]
    fn test_strict_replacement_with_apostrophe() {
        let text = "Don't worry about it.";
        let custom_words = vec![exact_replacement("don't", "do not")];
        let result = apply_custom_words(text, &custom_words, 0.18);
        assert_eq!(result, "Do not worry about it.");
    }

    #[test]
    fn test_strict_replacement_with_curly_apostrophe() {
        let text = "Don’t worry about it.";
        let custom_words = vec![exact_replacement("don't", "do not")];
        let result = apply_custom_words(text, &custom_words, 0.18);
        assert_eq!(result, "Do not worry about it.");
    }

    #[test]
    fn test_strict_replacement_all_caps() {
        // "BTW" -> "BY THE WAY" (ALL CAPS)
        let text = "BTW I am busy";
        let custom_words = vec![exact_replacement("btw", "by the way")];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "BY THE WAY I am busy");
    }

    #[test]
    fn test_strict_replacement_no_fuzzy() {
        // "bt" should NOT match "btw" strictly
        let text = "bt I am busy";
        let custom_words = vec![exact_replacement("btw", "by the way")];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "bt I am busy");
    }

    #[test]
    fn test_correction_fuzzy() {
        // "super wisper" -> "SuperWhisper"
        let text = "I use super wisper";
        let custom_words = vec![vocabulary("super whisper", "SuperWhisper")];
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
        let custom_words = vec![vocabulary("Handy", "Handy"), vocabulary("Candy", "Candy")];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "Handy is a great app");
    }

    #[test]
    fn test_multi_word_phrase_exact() {
        // "chat GPT" (two words) should match "ChatGPT" custom word
        let text = "I use chat GPT daily";
        let custom_words = vec![vocabulary("ChatGPT", "ChatGPT")];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "I use ChatGPT daily");
    }

    #[test]
    fn test_multi_word_replacement() {
        // UI example: "chat gpt" → "ChatGPT" with "Replace with different text" toggle ON
        // This is exact replacement mode (is_replacement=true), not fuzzy vocabulary mode
        let text = "I use chat gpt daily";
        let custom_words = vec![exact_replacement("chat gpt", "ChatGPT")];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "I use ChatGPT daily");
    }

    #[test]
    fn test_alias_exact_match() {
        let text = "Please use shad cn for this";
        let custom_words = vec![vocabulary_with_aliases(
            "shadcn",
            &["shad cn", "shad c n"],
            "shadcn",
        )];
        let result = apply_custom_words(text, &custom_words, 0.18);
        assert_eq!(result, "Please use shadcn for this");
    }

    #[test]
    fn test_alias_exact_match_preserves_punctuation() {
        let text = "Please use shad c n?";
        let custom_words = vec![vocabulary_with_aliases("shadcn", &["shad c n"], "shadcn")];
        let result = apply_custom_words(text, &custom_words, 0.18);
        assert_eq!(result, "Please use shadcn?");
    }

    #[test]
    fn test_exact_replacement_alias_longer_than_three_words() {
        let text = "Use my personal work email.";
        let custom_words = vec![exact_replacement_with_aliases(
            "my email",
            &["my personal work email"],
            "john@example.com",
        )];
        let result = apply_custom_words(text, &custom_words, 0.18);
        assert_eq!(result, "Use john@example.com.");
    }

    #[test]
    fn test_cplusplus_spoken_form_exact_match() {
        let text = "Use C plus plus for this project.";
        let custom_words = vec![vocabulary("c++", "c++")];
        let result = apply_custom_words(text, &custom_words, 0.18);
        assert_eq!(result, "Use c++ for this project.");
    }

    #[test]
    fn test_cplusplus_does_not_match_single_c() {
        let text = "Use C for now.";
        let custom_words = vec![vocabulary("c++", "c++")];
        let result = apply_custom_words(text, &custom_words, 0.18);
        assert_eq!(result, "Use C for now.");
    }

    #[test]
    fn test_cplusplus_collapses_trailing_plus_before_guard_word() {
        let text = "Use C plus plus plus for this project.";
        let custom_words = vec![vocabulary("c++", "c++")];
        let result = apply_custom_words(text, &custom_words, 0.18);
        assert_eq!(result, "Use c++ for this project.");
    }

    #[test]
    fn test_cplusplus_keeps_plus_when_followed_by_content_word() {
        let text = "Use C plus plus plus macros here.";
        let custom_words = vec![vocabulary("c++", "c++")];
        let result = apply_custom_words(text, &custom_words, 0.18);
        assert_eq!(result, "Use c++ plus macros here.");
    }

    #[test]
    fn test_split_token_fuzzy_success() {
        let text = "Create a new Shat CN component";
        let custom_words = vec![vocabulary("shadcn", "shadcn")];
        let result = apply_custom_words_with_thresholds(text, &custom_words, 0.18, 0.2);
        assert_eq!(result, "Create a new Shadcn component");
    }

    #[test]
    fn test_split_token_fuzzy_reject_dissimilar() {
        let text = "Create a new Chef CN component";
        let custom_words = vec![vocabulary("shadcn", "shadcn")];
        let result = apply_custom_words_with_thresholds(text, &custom_words, 0.18, 0.2);
        assert_eq!(result, "Create a new Chef CN component");
    }

    #[test]
    fn test_apply_custom_words_ngram_four_words_exact() {
        // Exact stage scans up to MAX_EXACT_NGRAM_WORDS and can recognize
        // fully split spell-out forms ("Chat G P T") as "ChatGPT".
        let text = "use Chat G P T for this";
        let custom_words = vec![vocabulary("ChatGPT", "ChatGPT")];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "use ChatGPT for this");
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
        // "phoen" sounds like "phone" - phonetic match with 5-char input
        // Note: 3-char inputs like "kat" are blocked by MIN_FUZZY_LENGTH guard
        let text = "I need a phoen";
        let custom_words = vec![vocabulary("phone", "phone")];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "I need a phone");
    }

    #[test]
    fn test_punctuation_preserved() {
        // Punctuation should be preserved around matches
        let text = "I use chatgpt!";
        let custom_words = vec![vocabulary("ChatGPT", "ChatGPT")];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "I use ChatGPT!");
    }

    #[test]
    fn test_no_false_multi_word_match() {
        // "use chat gpt" should NOT consume "use"
        // (this was the bug we fixed)
        let text = "I use chat gpt daily";
        let custom_words = vec![vocabulary("ChatGPT", "ChatGPT")];
        let result = apply_custom_words(text, &custom_words, 0.5);
        // "use" must be preserved, only "chat gpt" should become "ChatGPT"
        assert!(result.contains("use"));
        assert!(result.contains("ChatGPT"));
    }

    // ── False positive regression tests ─────────────────────────────

    #[test]
    fn test_no_false_positive_the_to_theyre() {
        // "the" must NOT be fuzzy-matched to "they're" — blocked by stop word + min-length guards
        let text = "the app should work";
        let custom_words = vec![vocabulary("they're", "they're")];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "the app should work");
    }

    #[test]
    fn test_no_false_positive_where_to_theyre() {
        // "where" must NOT be fuzzy-matched to "they're" — blocked by stop word guard
        let text = "go where we want";
        let custom_words = vec![vocabulary("they're", "they're")];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "go where we want");
    }

    #[test]
    fn test_no_false_positive_they_to_theyre() {
        // "they" must NOT be fuzzy-matched to "they're" — blocked by stop word guard
        let text = "they said hello";
        let custom_words = vec![vocabulary("they're", "they're")];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "they said hello");
    }

    #[test]
    fn test_no_false_positive_common_short_words() {
        // Common short words must never be fuzzy-matched
        let text = "we in to it is";
        let custom_words = vec![
            vocabulary("they're", "they're"),
            vocabulary("shadcn", "shadcn"),
        ];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "we in to it is");
    }

    #[test]
    fn test_no_false_positive_your_to_youre() {
        // "your" is high-frequency and should not fuzzily mutate to "you're"
        let text = "your code works";
        let custom_words = vec![vocabulary("you're", "you're")];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "your code works");
    }

    #[test]
    fn test_no_false_positive_then_to_than() {
        // "then" is high-frequency and should not fuzzily mutate to "than"
        let text = "then we test";
        let custom_words = vec![vocabulary("than", "than")];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "then we test");
    }

    #[test]
    fn test_no_false_positive_about_to_abort() {
        // "about" is high-frequency and should not fuzzily mutate to "abort"
        let text = "about this behavior";
        let custom_words = vec![vocabulary("abort", "abort")];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "about this behavior");
    }

    #[test]
    fn test_no_false_positive_mode_to_modal() {
        // "mode" should not fuzzily mutate to "modal" by one-char suffix extension.
        let text = "Create a new mode to see if it's actually working.";
        let custom_words = vec![vocabulary("modal", "modal")];
        let result = apply_custom_words(text, &custom_words, 0.18);
        assert_eq!(result, "Create a new mode to see if it's actually working.");
    }

    #[test]
    fn test_no_false_positive_model_to_modal() {
        // Similar short words with different semantics should not be rewritten.
        let text = "Create a new model to test.";
        let custom_words = vec![vocabulary("modal", "modal")];
        let result = apply_custom_words(text, &custom_words, 0.18);
        assert_eq!(result, "Create a new model to test.");
    }

    #[test]
    fn test_no_false_positive_cache_to_cash() {
        // Homophone-like pairs should not rewrite common product terms accidentally.
        let text = "Clear the cache before retrying.";
        let custom_words = vec![vocabulary("cash", "cash")];
        let result = apply_custom_words(text, &custom_words, 0.18);
        assert_eq!(result, "Clear the cache before retrying.");
    }

    #[test]
    fn test_no_false_positive_word_to_words() {
        // One-char suffix extensions should not rewrite standalone short words.
        let text = "This word should stay as word.";
        let custom_words = vec![vocabulary("words", "words")];
        let result = apply_custom_words(text, &custom_words, 0.18);
        assert_eq!(result, "This word should stay as word.");
    }

    #[test]
    fn test_short_exact_match_still_works() {
        // Short words can still be exact-matched (e.g. abbreviation replacement)
        let text = "btw I am busy";
        let custom_words = vec![exact_replacement("btw", "by the way")];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "by the way I am busy");
    }

    #[test]
    fn test_legitimate_fuzzy_still_works() {
        // Longer words should still be fuzzy-matched correctly
        let text = "I use Anthrapik daily";
        let custom_words = vec![vocabulary("Anthropic", "Anthropic")];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "I use Anthropic daily");
    }

    #[test]
    fn test_filter_transcription_output() {
        assert_eq!(filter_transcription_output("hello uh world"), "hello world");
        assert_eq!(filter_transcription_output("um hello"), "hello");
    }

    #[test]
    fn test_filter_and_count_single_filler() {
        let (text, count) = filter_and_count_filler_words("hello uh world");
        assert_eq!(text, "hello world");
        assert_eq!(count, 1);
    }

    #[test]
    fn test_filter_and_count_multiple_fillers() {
        let (text, count) = filter_and_count_filler_words("um like uh hello umm world");
        assert_eq!(text, "like hello world");
        assert_eq!(count, 3); // um, uh, umm
    }

    #[test]
    fn test_filter_and_count_no_fillers() {
        let (text, count) = filter_and_count_filler_words("clean text here");
        assert_eq!(text, "clean text here");
        assert_eq!(count, 0);
    }

    #[test]
    fn test_filter_and_count_empty_input() {
        let (text, count) = filter_and_count_filler_words("");
        assert_eq!(text, "");
        assert_eq!(count, 0);
    }

    #[test]
    fn test_filter_hallucinations_example1() {
        // "cont cont cont ... continue" → "continue" (prefix match: all copies dropped)
        assert_eq!(
            filter_hallucinations(
                "so I can cont cont cont cont cont cont cont cont cont continue debugging"
            ),
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
            filter_hallucinations(
                "sidebar, call call call call call call call call call call call it dictionary?"
            ),
            "sidebar, call it dictionary?"
        );
    }

    #[test]
    fn test_filter_hallucinations_example5() {
        // "three three three three three three three three three three three" -> "three"
        assert_eq!(
            filter_hallucinations(
                "one three three three three three three three three three three three"
            ),
            "one three"
        );
    }

    #[test]
    fn test_filter_hallucinations_short_stutter() {
        // "wh wh wh wh what" → "what" (prefix match: "wh" starts "what")
        assert_eq!(filter_hallucinations("wh wh wh wh what"), "what");
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
        assert_eq!(filter_hallucinations("the the cat"), "the the cat");
    }

    #[test]
    fn test_filter_hallucinations_case_insensitive() {
        // Mixed case, next word "thing" does NOT start with "the" → keep one
        assert_eq!(filter_hallucinations("The the THE the thing"), "The thing");
    }

    #[test]
    fn test_self_correction_normal_short_words_preserved() {
        // "I am" before "amazing" — "am" is in protected short words, so it is kept
        assert_eq!(filter_hallucinations("I am amazing"), "I am amazing");
    }

    #[test]
    fn test_self_correction_single_fragment_no_collapse() {
        // "to" is in protected short words, so it is kept
        assert_eq!(filter_hallucinations("to tomorrow"), "to tomorrow");
    }

    #[test]
    fn test_self_correction_progressive_prefix() {
        // "b bu buzz" — both short words are prefixes of "buzz"
        assert_eq!(filter_hallucinations("say b bu buzz"), "say buzz");
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
        // Target word "and" is short (3 chars), but "a" and "an" are protected short words
        assert_eq!(filter_hallucinations("a an and"), "a an and");
    }

    #[test]
    fn test_count_words() {
        assert_eq!(count_words("hello world"), 2);
        assert_eq!(count_words("  hello   world  "), 2);
        assert_eq!(count_words("One, two, three."), 3);
    }

    #[test]
    fn test_cleanup_n_new() {
        // "Would this n new phonetic slip up work with the correction file?"
        let input = "Would this n new phonetic slip up work with the correction file?";
        let expected = "Would this new phonetic slip up work with the correction file?";
        assert_eq!(filter_hallucinations(input), expected);
    }

    #[test]
    fn test_cleanup_f_from() {
        // "As in f from the sentence meaning..."
        let input = "As in f from the sentence meaning";
        let expected = "As in from the sentence meaning";
        assert_eq!(filter_hallucinations(input), expected);
    }

    #[test]
    fn test_cleanup_sh_showing() {
        // "...tile sh is still showing the street only."
        let input = "tile sh is still showing the street";
        let expected = "tile is still showing the street";
        assert_eq!(filter_hallucinations(input), expected);
    }

    #[test]
    fn test_cleanup_preserve_apple() {
        // "a" is protected, should NOT be removed
        let input = "I am a apple";
        assert_eq!(filter_hallucinations(input), "I am a apple");
    }

    #[test]
    fn test_cleanup_preserve_i_identify() {
        // "i" is protected, so it should not be removed before "identify"
        let input = "I i identify this";
        assert_eq!(filter_hallucinations(input), "I i identify this");
    }

    #[test]
    fn test_cleanup_preserve_go_going() {
        // "go" is a valid word, should NOT be removed
        let input = "I go going home";
        assert_eq!(filter_hallucinations(input), "I go going home");
    }

    #[test]
    fn test_cleanup_preserve_up_upload() {
        // "up" is a valid word, should be preserved
        let input = "Click up upload";
        assert_eq!(filter_hallucinations(input), "Click up upload");
    }

    #[test]
    fn test_cleanup_preserve_ok_okay() {
        // "ok" is protected, should NOT be removed
        let input = "It is ok okay";
        assert_eq!(filter_hallucinations(input), "It is ok okay");
    }

    #[test]
    fn test_cleanup_multi_fragment() {
        // "dr f fu fuzzy" -> "dr fuzzy"
        // 1. "dr" (len 2) is NOT a prefix of "fuzzy", so it is preserved.
        // 2. "f" (len 1) IS a prefix of "fuzzy", so it is dropped.
        // 3. "fu" (len 2) IS a prefix of "fuzzy", so it is dropped.
        assert_eq!(filter_hallucinations("dr f fu fuzzy"), "dr fuzzy");
    }

    #[test]
    fn test_regression_common_short_words_false_positives() {
        // These should NOT be collapsed because the short words are valid common words
        // properly used in context, even though they are prefixes of the next word.

        // "bus" is a prefix of "busy" -> should be preserved
        assert_eq!(
            filter_hallucinations("The bus busy schedule"),
            "The bus busy schedule"
        );

        // "car" is prefix of "carpet" -> should be preserved
        assert_eq!(
            filter_hallucinations("The car carpet is dirty"),
            "The car carpet is dirty"
        );

        // "pen" is prefix of "pencil" -> should be preserved
        assert_eq!(
            filter_hallucinations("Use a pen pencil or marker"),
            "Use a pen pencil or marker"
        );

        // "net" is prefix of "network" -> should be preserved
        assert_eq!(
            filter_hallucinations("The net network speed"),
            "The net network speed"
        );

        // "man" is prefix of "manage" -> should be preserved
        assert_eq!(
            filter_hallucinations("can you man manage this?"),
            "can you man manage this?"
        );
    }
}
