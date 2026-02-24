//! Correction Manager – orchestrates the AI voice correction pipeline.
//!
//! Flow: capture context → build prompt → send to LLM → emit result to overlay.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::collections::HashMap;
use tauri::{AppHandle, Manager};
use tracing::{debug, error, info, warn};

use crate::accessibility::{CapturedContext, CorrectionResult};
use crate::growth::{self, FeatureEntrypoint, FeatureName};
use crate::settings::{get_settings, AppSettings};
use regex::Regex;

use once_cell::sync::Lazy;
use std::collections::HashSet;

/// Hardcoded correction prompt — always used for the Fn+Z correction shortcut.
/// Embedded at compile time via `include_str!`. Build fails if file is missing.
/// Decoupled from user-configurable refine prompts.
const CORRECTION_PROMPT_TEMPLATE: &str = include_str!("../../../prompts/correct-text-v8.md");

static HOMOPHONES_MAP: Lazy<HashMap<String, Vec<String>>> = Lazy::new(|| {
    serde_json::from_str(include_str!("../../resources/homophones.json")).unwrap_or_default()
});

static PHONETIC_SLIPS_MAP: Lazy<HashMap<String, Vec<String>>> = Lazy::new(|| {
    serde_json::from_str(include_str!("../../resources/phonetic_slips.json")).unwrap_or_default()
});

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
use crate::managers::mlx::MlxModelManager;
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
use crate::settings::LOCAL_MLX_PROVIDER_ID;
use crate::settings::APPLE_INTELLIGENCE_PROVIDER_ID;

/// Manages the correction pipeline.
pub struct CorrectionManager {
    app_handle: AppHandle,
    /// Most recent correction result, stored for acceptance.
    last_result: Mutex<Option<CorrectionResult>>,
    /// Guard to prevent concurrent correction runs.
    in_progress: AtomicBool,
}

impl CorrectionManager {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            app_handle,
            last_result: Mutex::new(None),
            in_progress: AtomicBool::new(false),
        }
    }

    /// Get the last correction result (for acceptance flow).
    pub fn get_last_result(&self) -> Option<CorrectionResult> {
        self.last_result.lock().ok().and_then(|r| r.clone())
    }

    /// Run the full correction pipeline:
    /// 1. Capture context from the focused app
    /// 2. Build & interpolate correction prompt
    /// 3. Send to LLM
    /// 4. Compare original vs corrected
    pub async fn run_correction(&self) -> Result<CorrectionResult, String> {
        // Prevent concurrent correction runs (e.g. rapid Fn+Z keypresses)
        if self.in_progress.swap(true, Ordering::SeqCst) {
            warn!("Correction already in progress, ignoring duplicate trigger");
            return Err("correction_in_progress".to_string());
        }

        // Run the pipeline, ensuring in_progress is cleared on all exit paths
        let result = self.run_correction_inner().await;
        self.in_progress.store(false, Ordering::SeqCst);
        result
    }

    async fn run_correction_inner(&self) -> Result<CorrectionResult, String> {
        info!("Starting AI correction pipeline");

        // 1. Capture context
        let context = crate::accessibility::capture_context(&self.app_handle)?;
        
        let selected_text = context
            .selected_text
            .as_deref()
            .unwrap_or("")
            .to_string();

        if selected_text.is_empty() {
            info!("No text to correct (empty selection and no word at cursor)");
            return Err("no_text".to_string());
        }

        debug!(
            selected_len = selected_text.len(),
            context_len = context.context.len(),
            selected = %selected_text,
            context = %context.context,
            "Captured text for correction"
        );

        // Determine what to send to the LLM:
        // - If the context contains the selected text, send the full context so the LLM
        //   can see the word in its natural sentence (critical for homophones like their/they're)
        // - Otherwise, fall back to sending just the selected text
        let context_str = &context.context;
        let use_full_context = !context_str.is_empty() && context_str.contains(&selected_text);
        let text_for_llm = if use_full_context {
            context_str.clone()
        } else {
            selected_text.clone()
        };

        debug!(
            use_full_context = use_full_context,
            text_for_llm = %text_for_llm,
            "Text being sent to LLM"
        );

        // 2. Build prompt — send the full context as ${output} so LLM sees the sentence
        let settings = get_settings(&self.app_handle);
        let prompt = self.build_correction_prompt(&settings, &text_for_llm, &context)?;
        debug!(prompt_len = prompt.len(), prompt = %prompt, "Built correction prompt");

        // 3. Send to LLM
        let corrected_full = self.send_to_llm(&settings, &prompt).await?;
        let corrected_full = corrected_full.trim().to_string();
        
        // Parse JSON response if possible
        let corrected_text = match serde_json::from_str::<serde_json::Value>(&corrected_full) {
            Ok(json) => {
                if let Some(correction) = json.get("correction").and_then(|v| v.as_str()) {
                    debug!(thought = ?json.get("thought"), correction = %correction, "Parsed JSON correction");
                    correction.to_string()
                } else {
                    warn!("JSON response missing 'correction' field, using raw output");
                    corrected_full.clone()
                }
            }
            Err(_) => {
                // Try to clean markdown code blocks often returned by LLMs (```json ... ```)
                let cleaned = clean_json_md(&corrected_full);
                match serde_json::from_str::<serde_json::Value>(&cleaned) {
                    Ok(json) => {
                        if let Some(correction) = json.get("correction").and_then(|v| v.as_str()) {
                            debug!(thought = ?json.get("thought"), correction = %correction, "Parsed JSON correction (after cleanup)");
                            correction.to_string()
                        } else {
                             corrected_full.clone()
                        }
                    },
                    Err(_) => {
                        warn!("Failed to parse JSON response, falling back to raw output");
                        corrected_full.clone()
                    }
                }
            }
        };

        debug!(
            original = %text_for_llm,
            corrected = %corrected_text,
            "LLM returned correction"
        );

        // 4. Extract the correction for the selected text
        let (original_for_compare, mut corrected_for_result) = if use_full_context {
            // Find where the selected text appears in the original context,
            // then extract the corresponding region from the corrected output.
            extract_selected_correction(context_str, &selected_text, &corrected_text)
        } else {
            (selected_text.clone(), corrected_text.clone())
        };

        // 5. Strip trailing period that LLMs often add to mid-sentence corrections
        let has_suffix = if use_full_context {
            if let Some(sel_start) = context_str.find(&selected_text) {
                let sel_end = sel_start + selected_text.len();
                !context_str[sel_end..].trim().is_empty()
            } else {
                false
            }
        } else {
            false
        };
        corrected_for_result = strip_trailing_period(&original_for_compare, &corrected_for_result, has_suffix);

        // 6. Compare and build result
        let has_changes = corrected_for_result.trim() != original_for_compare.trim();
        let result = CorrectionResult {
            original: selected_text,
            corrected: corrected_for_result.trim().to_string(),
            has_changes,
        };

        info!(
            has_changes = result.has_changes,
            "Correction pipeline complete"
        );

        // Store for later acceptance
        if let Ok(mut last) = self.last_result.lock() {
            *last = Some(result.clone());
        }

        Ok(result)
    }

    /// Build the correction prompt by interpolating variables.
    ///
    /// Uses the hardcoded `CORRECTION_PROMPT_TEMPLATE` (v8), not the user-configurable
    /// refine prompts. This ensures the Fn+Z correction always uses the tested prompt.
    fn build_correction_prompt(
        &self,
        settings: &AppSettings,
        target_text: &str,
        context: &CapturedContext,
    ) -> Result<String, String> {
        let prompt_template = CORRECTION_PROMPT_TEMPLATE;

        let dict_total = settings.dictionary.len();
        let dict_used = dict_total.min(50);

        let mut hints_lines = Vec::new();
        for entry in settings.dictionary.iter().take(50) {
            // If it's a replacement (is_replacement=true), it's a strict fix.
            // If it's vocabulary (is_replacement=false), it's a biasing term.
            if entry.is_replacement {
                hints_lines.push(format!(
                    "Use '{}' instead of '{}'.",
                    entry.replacement, entry.input
                ));
            } else {
                // For vocabulary entries (e.g. "Kubernetes"), we might not have a specific wrong input.
                // If input matches replacement (case-insensitive), it's a general vocabulary hint.
                // We skip these for correction prompts to reduce noise, unless explicitly defined as input!=replacement.
                if !entry.input.eq_ignore_ascii_case(&entry.replacement) {
                    hints_lines.push(format!(
                        "Use '{}' contextually for '{}'.",
                        entry.replacement, entry.input
                    ));
                }
            }
        }

        // Inject deterministic homophone hints & phonetic slip-ups
        if let Some(selection) = context.selected_text.as_deref() {
            let hints_vec = generate_hints(selection);
            hints_lines.extend(hints_vec);
        }

        let hints = hints_lines.join("\n");

        debug!(
            dictionary_total = dict_total,
            dictionary_injected = dict_used,
            hints_preview = %if hints.len() > 200 { format!("{}...", &hints[..200]) } else { hints.clone() },
            "Dictionary hints for correction"
        );

        // Interpolate variables - JSON SAFE
        // We use serde_json::to_string to ensure strings are properly escaped for JSON prompts
        let safe_target = serde_json::to_string(target_text).unwrap_or(format!("\"{}\"", target_text));
        let safe_context = serde_json::to_string(&context.context).unwrap_or(format!("\"{}\"", context.context));
        let safe_selection = serde_json::to_string(context.selected_text.as_deref().unwrap_or("")).unwrap_or("\"\"".to_string());
        
        let prompt = interpolate_prompt(
            &prompt_template,
            &unquote_string(&safe_target),
            &unquote_string(&safe_context),
            &unquote_string(&safe_selection),
            &hints,
        );

        Ok(prompt)
    }

    /// Send the interpolated prompt to the configured LLM provider.
    async fn send_to_llm(&self, settings: &AppSettings, prompt: &str) -> Result<String, String> {
        let provider = settings
            .active_post_process_provider()
            .cloned()
            .ok_or("No post-process provider configured")?;

        let model = settings
            .post_process_models
            .get(&provider.id)
            .cloned()
            .unwrap_or_default();

        if model.trim().is_empty() {
            return Err(format!(
                "No model configured for provider '{}'",
                provider.id
            ));
        }

        debug!(
            provider = provider.id,
            model = model,
            "Sending correction to LLM"
        );

        // Handle MLX Local AI
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        if provider.id == LOCAL_MLX_PROVIDER_ID {
            let mlx_manager = self.app_handle.state::<Arc<MlxModelManager>>();
            return mlx_manager
                // Use greedy decoding (temp=0) for correction to ensure stability and deterministic output
                // v8 prompt is optimized for this.
                .process_text(prompt, Some(0.0), Some(1.0), Some(0.0))
                .await
                .map_err(|e| {
                    error!("MLX correction failed: {}", e);
                    format!("MLX processing failed: {}", e)
                });
        }

        // Handle Apple Intelligence
        if provider.id == APPLE_INTELLIGENCE_PROVIDER_ID {
            #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
            {
                if !crate::apple_intelligence::check_apple_intelligence_availability() {
                    return Err("Apple Intelligence not available".to_string());
                }
                let token_limit = model.trim().parse::<i32>().unwrap_or(0);
                return crate::apple_intelligence::process_text_with_system_prompt("", prompt, token_limit)
                    .map_err(|e| format!("Apple Intelligence failed: {}", e));
            }
            #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
            return Err("Apple Intelligence not supported on this platform".to_string());
        }

        // Handle remote API providers
        let api_key = settings
            .post_process_api_keys
            .get(&provider.id)
            .cloned()
            .unwrap_or_default();

        match crate::llm_client::send_chat_completion(&provider, api_key, &model, prompt.to_string())
            .await
        {
            Ok(Some(content)) => {
                // Strip invisible Unicode characters
                let content = content
                    .replace('\u{200B}', "")
                    .replace('\u{200C}', "")
                    .replace('\u{200D}', "")
                    .replace('\u{FEFF}', "");
                Ok(content)
            }
            Ok(None) => Err("LLM returned empty response".to_string()),
            Err(e) => {
                error!("LLM correction failed: {}", e);
                Err(format!("LLM failed: {}", e))
            }
        }
    }

    /// Accept a correction — replace text in the target app.
    pub fn accept_correction(
        &self,
        correction: &CorrectionResult,
        entrypoint: FeatureEntrypoint,
    ) -> Result<(), String> {
        info!(
            original_len = correction.original.len(),
            corrected_len = correction.corrected.len(),
            "Accepting correction"
        );
        if let Err(e) =
            crate::accessibility::replace_text_in_app(&self.app_handle, &correction.original, &correction.corrected)
        {
            error!("Failed to replace text: {}", e);
            crate::notification::show_error(&self.app_handle, "errors.correctionReplaceFailed");
            return Err(e);
        }

        growth::record_feature_success(&self.app_handle, FeatureName::CorrectText, entrypoint);
        Ok(())
    }

    /// Dismiss a correction — just hide the overlay.
    pub fn dismiss_correction(&self) {
        debug!("Correction dismissed by user");
        // Overlay hiding is handled by the caller
    }
}

/// Helper to clean markdown code blocks from JSON response
fn clean_json_md(text: &str) -> String {
    let text = text.trim();
    if let Some(stripped) = text.strip_prefix("```json") {
        stripped.trim_end_matches("```").trim().to_string()
    } else if let Some(stripped) = text.strip_prefix("```") {
        stripped.trim_end_matches("```").trim().to_string()
    } else {
        text.to_string()
    }
}

/// Helper to safely unquote JSON strings (removes surrounding quotes but keeps internal escapes)
fn unquote_string(s: &str) -> String {
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

/// Interpolate prompt variables: ${output}, ${context}, ${selection}, ${hints}/${dictionary}.
pub fn interpolate_prompt(
    template: &str,
    output: &str,
    context: &str,
    selection: &str,
    hints: &str,
) -> String {
    template
        .replace("${output}", output)
        .replace("${context}", context)
        .replace("${selection}", selection)
        .replace("${hints}", hints)
        // Support legacy/alternate variable name for compatibility
        .replace("${dictionary}", hints)
}

/// Extract the correction for a selected word/phrase from a corrected full sentence.
///
/// When we send the full context to the LLM (so it can see the word in context),
/// we need to figure out what word(s) in the corrected output correspond to the
/// user's original selection.
///
/// Strategy: find the word-offset of the selection in the original context,
/// then extract that many words from the corrected output. If the LLM added/removed
/// words, fall back to comparing the corrected output to the original to find diffs.
fn extract_selected_correction(
    original_context: &str,
    selected_text: &str,
    corrected_context: &str,
) -> (String, String) {
    // Find where the selected text starts in the original context
    let Some(sel_start) = original_context.find(selected_text) else {
        // Selection not found in context — can't extract, compare directly
        debug!("Selected text not found in context, comparing directly");
        return (selected_text.to_string(), corrected_context.to_string());
    };
    let sel_end = sel_start + selected_text.len();

    // Split original into: prefix + selected + suffix
    let prefix = &original_context[..sel_start];
    let suffix = &original_context[sel_end..];

    // Count words in prefix and selected text
    let prefix_word_count = prefix.split_whitespace().count();
    let selected_word_count = selected_text.split_whitespace().count();

    // Split corrected context into words
    let corrected_words: Vec<&str> = corrected_context.split_whitespace().collect();

    // Extract the corresponding words from the corrected output
    // Strategy: Prioritize matching the surrounding context (suffix/prefix) in the corrected output.
    // relying on word counting is fragile when the LLM expands or contracts the selection length.

    let prefix_char_end = find_byte_after_n_words(corrected_context, prefix_word_count);

    // 1. Try Suffix Matching
    if !suffix.trim().is_empty() {
        if let Some(suffix_pos) = corrected_context.find(suffix.trim()) {
            // Check if suffix appears *after* prefix end
            if prefix_char_end <= suffix_pos {
                let corrected_selected = corrected_context
                    [prefix_char_end..suffix_pos]
                    .trim()
                    .to_string();
                debug!(
                    corrected_selected = %corrected_selected,
                    suffix_match = true,
                    "Extracted correction via suffix matching"
                );
                return (selected_text.to_string(), corrected_selected);
            }
        }
    } else {
        // 2. Suffix is empty (end of sentence)
        // Take everything after the prefix end
        if prefix_char_end <= corrected_context.len() {
             let corrected_selected = corrected_context[prefix_char_end..].trim().to_string();
             debug!(
                corrected_selected = %corrected_selected,
                end_of_sentence = true,
                "Extracted correction via end-of-sentence logic"
            );
            return (selected_text.to_string(), corrected_selected);
        }
    }

    // 3. Fallback to Word Count Matching
    // Only if suffix matching fails (e.g., LLM changed the suffix text too)
    if prefix_word_count + selected_word_count <= corrected_words.len() {
        let corrected_selected: String = corrected_words
            [prefix_word_count..prefix_word_count + selected_word_count]
            .join(" ");

        debug!(
            prefix_words = prefix_word_count,
            selected_words = selected_word_count,
            corrected_selected = %corrected_selected,
            "Extracted correction via word count fallback"
        );

        (selected_text.to_string(), corrected_selected)
    } else {
        // Last resort: return the full corrected context
        debug!("Could not extract selected region, using full corrected output");
        (selected_text.to_string(), corrected_context.to_string())
    }
}

/// Find the byte position in `text` after the first `n` whitespace-separated words.
/// Returns the offset of leading whitespace if n == 0. Returns text.len() if fewer than n words.
fn find_byte_after_n_words(text: &str, n: usize) -> usize {
    if n == 0 {
        // Skip leading whitespace so we start at the first word
        return text.len() - text.trim_start().len();
    }
    let mut count = 0;
    let mut in_word = false;
    for (i, ch) in text.char_indices() {
        if ch.is_whitespace() {
            if in_word {
                count += 1;
                if count == n {
                    return i;
                }
                in_word = false;
            }
        } else {
            in_word = true;
        }
    }
    text.len()
}

/// Strip trailing period that LLMs often add to corrections.
///
/// Only strips if:
/// 1. The corrected text ends with a period
/// 2. The original text did NOT end with a period
/// 3. There is text after the selection in the context (mid-sentence)
fn strip_trailing_period(original: &str, corrected: &str, has_suffix: bool) -> String {
    let corrected_trimmed = corrected.trim_end();
    if corrected_trimmed.ends_with('.')
        && !original.trim_end().ends_with('.')
        && has_suffix
    {
        let stripped = corrected_trimmed[..corrected_trimmed.len() - 1].to_string();
        debug!(
            original = %original,
            corrected = %corrected,
            stripped = %stripped,
            "Stripped trailing period from mid-sentence correction"
        );
        stripped
    } else {
        corrected.to_string()
    }
}

/// Generate hints by scanning the selection for known homophones and slips.
fn generate_hints(selection: &str) -> Vec<String> {
    let selection_lower = selection.trim().to_lowercase();
    let mut unique_hints = std::collections::HashSet::new();
    let mut hints_list = Vec::new();

    // Stopwords to ignore for high-frequency homophones to reduce prompt noise
    let stopwords: HashSet<&str> = ["i", "a", "the", "is", "it", "in", "to", "for", "of", "on", "at", "be", "do", "we", "he", "by", "or", "an", "no", "so"].into();

    // 1. Phonetic Slip-ups
    for (pattern, candidates) in PHONETIC_SLIPS_MAP.iter() {
        if selection_lower.contains(pattern) {
            let pattern_escaped = regex::escape(pattern);
            let re_str = format!(r"\b{}\b", pattern_escaped);
            if let Ok(re) = Regex::new(&re_str) {
                if re.is_match(&selection_lower) {
                    let candidates_str = candidates.join("', '");
                    // Simplified format: "'pattern' is likely supposed to be 'candidate'."
                    let hint = format!("'{}' is likely supposed to be '{}'.", pattern, candidates_str);
                    if unique_hints.insert(hint.clone()) {
                        hints_list.push(hint);
                    }
                }
            }
        }
    }

    // 2. Homophones
    // Use regex to split by non-word characters to get tokens
    let re_words = Regex::new(r"\w+").unwrap();
    for cap in re_words.captures_iter(&selection_lower) {
        let word = &cap[0];
        // Skip common stopwords to avoid noise (e.g. "to" -> "too", "two")
        if stopwords.contains(word) {
            continue;
        }

        if let Some(candidates) = HOMOPHONES_MAP.get(word) {
            let candidates_str = candidates.join("', '");
            // Simplified format: "'word' might be meant as 'candidate'."
            let hint = format!("'{}' might be meant as '{}'.", word, candidates_str);
            if unique_hints.insert(hint.clone()) {
                hints_list.push(hint);
            }
        }
    }
    
    hints_list
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_hints_sentence() {
        // Test case from user report
        let selection = "He read the book allowed to the class";
        let hints = generate_hints(selection);
        
        // Should contain hint for "allowed" -> "aloud"
        let found = hints.iter().any(|h| h.contains("'allowed' might be meant as 'aloud'"));
        assert!(found, "Expected hint for 'allowed' in sentence: {:?}", hints);
    }
    
    #[test]
    fn test_generate_hints_multi_word_slip() {
        // "ten to" was removed from phonetic_slips.json as it was a valid phrase.
        // Let's use "could of" -> "could have", which is a definitive error.
        
        let selection = "I could of gone";
        let hints = generate_hints(selection);
        
        // Expected format: 'could of' is likely supposed to be 'could have'.
        let found_slip = hints.iter().any(|h| h.contains("'could of' is likely supposed to be 'could have'"));
        assert!(found_slip, "Expected hint for 'could of'. Hints: {:?}", hints);
    }

    #[test]
    fn test_generate_hints_stopwords() {
        // "to" is a stopword, should not generate hint for "too"/"two"
        // "allowed" is NOT a stopword, should generate hint
        let selection = "I allowed him to go";
        let hints = generate_hints(selection);
        
        let has_allowed = hints.iter().any(|h| h.contains("'allowed' might be meant as 'aloud'"));
        let has_to = hints.iter().any(|h| h.contains("'to' might be meant as"));
        
        assert!(has_allowed, "Should contain hint for 'allowed'");
        assert!(!has_to, "Should NOT contain hint for stopword 'to'. Hints: {:?}", hints);
    }
    
    #[test]
    fn test_unquote_string() {
        assert_eq!(unquote_string(r#""hello""#), "hello");
        // Escaped quotes inside should be preserved
        assert_eq!(unquote_string(r#""he said \"hello\"""#), r#"he said \"hello\""#);
        // No quotes
        assert_eq!(unquote_string("hello"), "hello");
        // Empty quotes
        assert_eq!(unquote_string(r#""""#), "");
    }
    
    #[test]
    fn test_clean_json_md_edge_cases() {
        // Nested backticks logic using strip_prefix
        assert_eq!(clean_json_md("```json\n{\"a\":1}\n```"), "{\"a\":1}");
        // Malformed backticks - strip_prefix handles exact matches
        assert_eq!(clean_json_md("``json\n{\"a\":1}\n``"), "``json\n{\"a\":1}\n``");
        // Extra whitespace
        assert_eq!(clean_json_md("   ```json   \n  {\"a\": 1}  \n   ```   "), "{\"a\": 1}");
    }

    #[test]
    fn test_interpolate_prompt_all_vars() {
        let template = "Context: ${context}\nSelected: ${selection}\nText: ${output}\nHints: ${hints}";
        let result = interpolate_prompt(template, "hello", "surrounding text", "the word", "- use X");
        assert_eq!(
            result,
            "Context: surrounding text\nSelected: the word\nText: hello\nHints: - use X"
        );
    }

    #[test]
    fn test_interpolate_prompt_no_vars() {
        let template = "Fix this text please.";
        let result = interpolate_prompt(template, "text", "ctx", "sel", "");
        assert_eq!(result, "Fix this text please.");
    }

    #[test]
    fn test_interpolate_prompt_output_only() {
        let template = "Fix: ${output}";
        let result = interpolate_prompt(template, "there going", "", "", "");
        assert_eq!(result, "Fix: there going");
    }

    #[test]
    fn test_interpolate_prompt_dictionary_alias() {
        let template = "Dict: ${dictionary}";
        let result = interpolate_prompt(template, "", "", "", "- hint");
        assert_eq!(result, "Dict: - hint");
    }

    // ── extract_selected_correction tests ──────────────────────────

    #[test]
    fn test_extract_word_aligned_single_word() {
        // "their" → "they're" in "they said their going"
        let (orig, corrected) = extract_selected_correction(
            "they said their going",
            "their",
            "they said they're going",
        );
        assert_eq!(orig, "their");
        assert_eq!(corrected, "they're");
    }

    #[test]
    fn test_extract_word_aligned_first_word() {
        let (orig, corrected) = extract_selected_correction(
            "their going to the store",
            "their",
            "they're going to the store",
        );
        assert_eq!(orig, "their");
        assert_eq!(corrected, "they're");
    }

    #[test]
    fn test_extract_word_aligned_last_word() {
        let (orig, corrected) = extract_selected_correction(
            "I went over their",
            "their",
            "I went over there",
        );
        assert_eq!(orig, "their");
        assert_eq!(corrected, "there");
    }

    #[test]
    fn test_extract_multi_word_selection() {
        let (orig, corrected) = extract_selected_correction(
            "we should of went there",
            "should of",
            "we should have went there",
        );
        assert_eq!(orig, "should of");
        assert_eq!(corrected, "should have");
    }

    #[test]
    fn test_extract_selection_not_in_context() {
        // When selection is not found in context, fall back to full comparison
        let (orig, corrected) = extract_selected_correction(
            "hello world",
            "missing",
            "hello universe",
        );
        assert_eq!(orig, "missing");
        assert_eq!(corrected, "hello universe");
    }

    #[test]
    fn test_extract_suffix_fallback_when_llm_changes_word_count() {
        // LLM expands "cant" to "can not" (1 word → 2 words) but suffix matches
        let (orig, corrected) = extract_selected_correction(
            "I cant believe it happened",
            "cant",
            "I can not believe it happened",
        );
        assert_eq!(orig, "cant");
        assert_eq!(corrected, "can not");
    }

    #[test]
    fn test_extract_suffix_fallback_word_count_exceeded() {
        // LLM returns fewer words than expected
        let (orig, corrected) = extract_selected_correction(
            "the quick brown fox jumps",
            "brown fox",
            "the fast fox jumps",
        );
        // Correct behavior: "brown fox" -> "fox"
        // Incorrect (old) behavior: "brown fox" -> "fox jumps"
        assert_eq!(orig, "brown fox");
        assert_eq!(corrected, "fox");
    }

    #[test]
    fn test_extract_no_changes() {
        let (orig, corrected) = extract_selected_correction(
            "hello world today",
            "world",
            "hello world today",
        );
        assert_eq!(orig, "world");
        assert_eq!(corrected, "world");
    }

    // ── find_byte_after_n_words tests ──────────────────────────────

    #[test]
    fn test_find_byte_after_0_words() {
        assert_eq!(find_byte_after_n_words("hello world", 0), 0);
        assert_eq!(find_byte_after_n_words("  hello world", 0), 2);
    }

    #[test]
    fn test_find_byte_after_1_word() {
        // "hello world" → after "hello" is at position 5 (the space)
        assert_eq!(find_byte_after_n_words("hello world", 1), 5);
    }

    #[test]
    fn test_find_byte_after_2_words() {
        assert_eq!(find_byte_after_n_words("hello big world", 2), 9);
    }

    #[test]
    fn test_find_byte_after_n_words_exceeds() {
        // More words requested than available
        assert_eq!(find_byte_after_n_words("hello", 2), 5);
    }

    #[test]
    fn test_find_byte_after_n_words_unicode() {
        // Unicode: "café latte" — "é" is 2 bytes
        let text = "café latte";
        let pos = find_byte_after_n_words(text, 1);
        // "café" is 5 bytes (c=1, a=1, f=1, é=2), space at byte 5
        assert_eq!(pos, 5);
        assert_eq!(&text[pos..].trim_start(), &"latte");
    }

    // ── strip_trailing_period tests ────────────────────────────────

    #[test]
    fn test_strip_period_mid_sentence() {
        let result = strip_trailing_period("their", "they're.", true);
        assert_eq!(result, "they're");
    }

    #[test]
    fn test_preserve_period_end_of_sentence() {
        // No suffix → end of sentence, keep the period
        let result = strip_trailing_period("their", "they're.", false);
        assert_eq!(result, "they're.");
    }

    #[test]
    fn test_preserve_period_when_original_had_period() {
        // Original already had a period
        let result = strip_trailing_period("their.", "they're.", true);
        assert_eq!(result, "they're.");
    }

    #[test]
    fn test_no_strip_when_no_period() {
        let result = strip_trailing_period("their", "they're", true);
        assert_eq!(result, "they're");
    }

    #[test]
    fn test_strip_period_with_trailing_whitespace() {
        let result = strip_trailing_period("their", "they're.  ", true);
        assert_eq!(result, "they're");
    }

    // ── clean_json_md tests ────────────────────────────────────────

    #[test]
    fn test_clean_json_md_no_md() {
        let input = r#"{"correction": "test"}"#;
        assert_eq!(clean_json_md(input), input);
    }

    #[test]
    fn test_clean_json_md_with_json_block() {
        let input = r#"```json
{"correction": "test"}
```"#;
        assert_eq!(clean_json_md(input), r#"{"correction": "test"}"#);
    }

    #[test]
    fn test_clean_json_md_with_generic_block() {
        let input = r#"```
{"correction": "test"}
```"#;
        assert_eq!(clean_json_md(input), r#"{"correction": "test"}"#);
    }

    #[test]
    fn test_extract_correction_expansion() {
        // "firstable" (1 word) -> "first of all" (3 words)
        // Original: "This is firstable wrong"
        // Corrected: "This is first of all wrong"
        let (orig, corrected) = extract_selected_correction(
            "This is firstable wrong",
            "firstable",
            "This is first of all wrong",
        );
        assert_eq!(orig, "firstable");
        assert_eq!(corrected, "first of all");
    }

    #[test]
    fn test_extract_correction_compression() {
        // "old timers disease" (3 words) -> "Alzheimer's disease" (2 words)
        let (orig, corrected) = extract_selected_correction(
            "He has old timers disease now",
            "old timers disease",
            "He has Alzheimer's disease now",
        );
        assert_eq!(orig, "old timers disease");
        assert_eq!(corrected, "Alzheimer's disease");
    }
}
