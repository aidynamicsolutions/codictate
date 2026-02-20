use crate::accessibility::TextInsertionContext;
use regex::Regex;
use std::sync::LazyLock;
use tracing::debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SmartInsertionProfile {
    CasedWhitespace,
    UncasedWhitespace,
    NoBoundarySpacing,
    Conservative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SentenceMarkKind {
    PeriodLike,
    QuestionLike,
    ExclamationLike,
}

fn normalize_selected_language(selected_language: &str) -> String {
    let normalized = selected_language.trim().replace('_', "-").to_lowercase();
    if normalized.is_empty() {
        return "auto".to_string();
    }

    if normalized == "auto" {
        return normalized;
    }

    if normalized.starts_with("zh-hans") {
        return "zh".to_string();
    }
    if normalized.starts_with("zh-hant")
        || normalized.starts_with("zh-tw")
        || normalized.starts_with("zh-hk")
        || normalized.starts_with("zh-mo")
    {
        return "zh-tw".to_string();
    }
    if normalized.starts_with("yue") {
        return "yue".to_string();
    }

    normalized
        .split('-')
        .next()
        .filter(|tag| !tag.is_empty())
        .unwrap_or("auto")
        .to_string()
}

fn resolve_smart_insertion_profile(
    normalized_language: &str,
) -> (SmartInsertionProfile, &'static str) {
    match normalized_language {
        "en" | "es" | "fr" | "de" | "it" | "pt" | "pl" | "cs" | "ru" | "uk" | "vi" => (
            SmartInsertionProfile::CasedWhitespace,
            "cased_whitespace_allowlist",
        ),
        "ar" | "fa" | "ur" | "he" | "ko" => (
            SmartInsertionProfile::UncasedWhitespace,
            "uncased_whitespace_allowlist",
        ),
        "zh" | "zh-tw" | "yue" | "ja" | "th" | "km" | "lo" | "my" | "bo" => (
            SmartInsertionProfile::NoBoundarySpacing,
            "no_boundary_spacing_allowlist",
        ),
        "auto" => (
            SmartInsertionProfile::Conservative,
            "auto_language_conservative_fallback",
        ),
        "tr" => (
            SmartInsertionProfile::Conservative,
            "turkish_conservative_fallback",
        ),
        _ => (
            SmartInsertionProfile::Conservative,
            "unknown_language_conservative_fallback",
        ),
    }
}

fn sentence_mark_kind(c: char, profile: SmartInsertionProfile) -> Option<SentenceMarkKind> {
    match profile {
        SmartInsertionProfile::CasedWhitespace => match c {
            '.' => Some(SentenceMarkKind::PeriodLike),
            '?' => Some(SentenceMarkKind::QuestionLike),
            '!' => Some(SentenceMarkKind::ExclamationLike),
            _ => None,
        },
        SmartInsertionProfile::UncasedWhitespace => match c {
            '.' | '۔' => Some(SentenceMarkKind::PeriodLike),
            '?' | '؟' => Some(SentenceMarkKind::QuestionLike),
            '!' => Some(SentenceMarkKind::ExclamationLike),
            _ => None,
        },
        SmartInsertionProfile::NoBoundarySpacing => match c {
            '.' | '。' => Some(SentenceMarkKind::PeriodLike),
            '?' | '？' => Some(SentenceMarkKind::QuestionLike),
            '!' | '！' => Some(SentenceMarkKind::ExclamationLike),
            _ => None,
        },
        SmartInsertionProfile::Conservative => None,
    }
}

fn is_sentence_terminator(c: char, profile: SmartInsertionProfile) -> bool {
    sentence_mark_kind(c, profile).is_some()
}

fn is_clause_boundary_punctuation(c: char, profile: SmartInsertionProfile) -> bool {
    match profile {
        SmartInsertionProfile::CasedWhitespace => {
            matches!(c, ',' | ';' | ':' | '，' | '；' | '：')
        }
        SmartInsertionProfile::UncasedWhitespace => {
            matches!(c, ',' | ';' | ':' | '،' | '؛' | '，' | '；' | '：')
        }
        SmartInsertionProfile::NoBoundarySpacing | SmartInsertionProfile::Conservative => false,
    }
}

/// Collapses punctuation artifacts that can emerge from spoken punctuation
/// conversion (e.g., ". ," -> "," and ". ." -> ".").
static SENTENCE_TO_CLAUSE_PUNCTUATION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[.!?]\s+([,;:])").expect("valid sentence-to-clause regex"));
static SPACED_DUPLICATE_COMMA_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r",(?:\s+,)+").expect("valid duplicate comma regex"));
static SPACED_DUPLICATE_PERIOD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\.(?:\s+\.)+").expect("valid duplicate period regex"));
static SPACED_DUPLICATE_SEMICOLON_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r";(?:\s+;)+").expect("valid duplicate semicolon regex"));
static SPACED_DUPLICATE_COLON_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r":(?:\s+:)+").expect("valid duplicate colon regex"));
static SPACED_DUPLICATE_EXCLAMATION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"!(?:\s+!)+").expect("valid duplicate exclamation regex"));
static SPACED_DUPLICATE_QUESTION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\?(?:\s+\?)+").expect("valid duplicate question regex"));
static SPACED_DUPLICATE_HYPHEN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(^|\s)-\s-(\s|$)").expect("valid duplicate hyphen regex"));
static SPACED_DUPLICATE_EN_DASH_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(^|\s)–\s–(\s|$)").expect("valid duplicate en dash regex"));
static SPACED_DUPLICATE_EM_DASH_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(^|\s)—\s—(\s|$)").expect("valid duplicate em dash regex"));
static SPACED_CONFLICTING_SENTENCE_PUNCTUATION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[.!?]\s+([.!?])").expect("valid conflicting sentence punctuation regex")
});

pub(crate) fn collapse_spaced_punctuation_artifacts(text: &str) -> String {
    let mut cleaned = text.to_string();

    // Iterate to a small fixed point to handle cascaded artifacts.
    for _ in 0..3 {
        let previous = cleaned.clone();
        cleaned = SENTENCE_TO_CLAUSE_PUNCTUATION_RE
            .replace_all(&cleaned, "$1")
            .into_owned();
        cleaned = SPACED_DUPLICATE_COMMA_RE
            .replace_all(&cleaned, ",")
            .into_owned();
        cleaned = SPACED_DUPLICATE_PERIOD_RE
            .replace_all(&cleaned, ".")
            .into_owned();
        cleaned = SPACED_DUPLICATE_SEMICOLON_RE
            .replace_all(&cleaned, ";")
            .into_owned();
        cleaned = SPACED_DUPLICATE_COLON_RE
            .replace_all(&cleaned, ":")
            .into_owned();
        cleaned = SPACED_DUPLICATE_EXCLAMATION_RE
            .replace_all(&cleaned, "!")
            .into_owned();
        cleaned = SPACED_DUPLICATE_QUESTION_RE
            .replace_all(&cleaned, "?")
            .into_owned();
        cleaned = SPACED_DUPLICATE_HYPHEN_RE
            .replace_all(&cleaned, "$1-$2")
            .into_owned();
        cleaned = SPACED_DUPLICATE_EN_DASH_RE
            .replace_all(&cleaned, "$1–$2")
            .into_owned();
        cleaned = SPACED_DUPLICATE_EM_DASH_RE
            .replace_all(&cleaned, "$1—$2")
            .into_owned();
        cleaned = SPACED_CONFLICTING_SENTENCE_PUNCTUATION_RE
            .replace_all(&cleaned, "$1")
            .into_owned();

        if cleaned == previous {
            break;
        }
    }

    cleaned
}

fn continuation_allows_punctuation_strip(
    right_non_whitespace_char: char,
    profile: SmartInsertionProfile,
) -> bool {
    match profile {
        SmartInsertionProfile::CasedWhitespace => {
            right_non_whitespace_char.is_lowercase() || right_non_whitespace_char.is_numeric()
        }
        SmartInsertionProfile::UncasedWhitespace | SmartInsertionProfile::NoBoundarySpacing => {
            right_non_whitespace_char.is_alphabetic() || right_non_whitespace_char.is_numeric()
        }
        SmartInsertionProfile::Conservative => false,
    }
}

fn is_word_like(c: char) -> bool {
    c.is_alphanumeric()
}

fn first_non_whitespace_char(text: &str) -> Option<char> {
    text.chars().find(|c| !c.is_whitespace())
}

fn last_non_whitespace_char(text: &str) -> Option<char> {
    text.chars().rev().find(|c| !c.is_whitespace())
}

fn first_alphabetic_char_index(text: &str) -> Option<(usize, char)> {
    text.char_indices().find(|(_, c)| c.is_alphabetic())
}

fn replace_char_at(text: &str, index: usize, replacement: String) -> String {
    let mut output = String::with_capacity(text.len() + replacement.len());
    output.push_str(&text[..index]);
    let original_len = text[index..]
        .chars()
        .next()
        .map(|c| c.len_utf8())
        .unwrap_or(0);
    output.push_str(&replacement);
    output.push_str(&text[index + original_len..]);
    output
}

fn uppercase_first_alphabetic(text: &str) -> Option<String> {
    let (index, first_char) = first_alphabetic_char_index(text)?;
    if first_char.is_uppercase() {
        return None;
    }
    Some(replace_char_at(
        text,
        index,
        first_char.to_uppercase().collect(),
    ))
}

fn lowercase_first_alphabetic(text: &str) -> Option<String> {
    let (index, first_char) = first_alphabetic_char_index(text)?;
    if first_char.is_lowercase() {
        return None;
    }
    Some(replace_char_at(
        text,
        index,
        first_char.to_lowercase().collect(),
    ))
}

fn is_title_like_start(text: &str) -> bool {
    let Some(token) = text.trim_start().split_whitespace().next() else {
        return false;
    };

    let alpha_chars: Vec<char> = token.chars().filter(|c| c.is_alphabetic()).collect();
    if alpha_chars.len() < 2 {
        return false;
    }

    let starts_upper = alpha_chars[0].is_uppercase();
    let rest_all_upper = alpha_chars[1..].iter().all(|c| c.is_uppercase());
    let rest_has_lower = alpha_chars[1..].iter().any(|c| c.is_lowercase());

    starts_upper && !rest_all_upper && rest_has_lower
}

fn is_sentence_start(context: &TextInsertionContext, profile: SmartInsertionProfile) -> bool {
    match context.left_non_whitespace_char {
        None => true,
        Some(c) => is_sentence_terminator(c, profile),
    }
}

fn trailing_sentence_punctuation_char(text: &str, profile: SmartInsertionProfile) -> Option<char> {
    let trimmed_end = text.trim_end_matches(|c: char| c.is_whitespace());
    let last_char = trimmed_end.chars().next_back()?;
    if is_sentence_terminator(last_char, profile) {
        Some(last_char)
    } else {
        None
    }
}

fn strip_single_trailing_sentence_punctuation(text: &str) -> String {
    let trimmed_end = text.trim_end_matches(|c: char| c.is_whitespace());
    let trailing_whitespace = &text[trimmed_end.len()..];
    let Some(last_char) = trimmed_end.chars().next_back() else {
        return text.to_string();
    };
    let end_without_last = trimmed_end.len().saturating_sub(last_char.len_utf8());

    let mut output = String::with_capacity(text.len());
    output.push_str(&trimmed_end[..end_without_last]);
    output.push_str(trailing_whitespace);
    output
}

fn has_abbreviation_like_internal_dots(text: &str, profile: SmartInsertionProfile) -> bool {
    let Some(last_token) = text.trim_end().split_whitespace().last() else {
        return false;
    };

    let token_without_trailing =
        last_token.trim_end_matches(|c: char| is_sentence_terminator(c, profile));
    token_without_trailing.contains('.')
        && token_without_trailing.chars().any(|c| c.is_alphabetic())
}

fn sanitize_trailing_sentence_punctuation(
    text: &str,
    context: &TextInsertionContext,
    profile: SmartInsertionProfile,
) -> (String, bool, &'static str) {
    let Some(_candidate) = trailing_sentence_punctuation_char(text, profile) else {
        return (text.to_string(), false, "no_candidate_punctuation");
    };

    let Some(right_non_whitespace_char) = context.right_non_whitespace_char else {
        return (text.to_string(), false, "no_right_continuation");
    };

    if has_abbreviation_like_internal_dots(text, profile) {
        return (text.to_string(), false, "abbreviation_guard");
    }

    if continuation_allows_punctuation_strip(right_non_whitespace_char, profile) {
        let reason = if context.has_selection {
            "selection_continuation_allows_strip"
        } else {
            "non_selection_continuation_allows_strip"
        };
        return (
            strip_single_trailing_sentence_punctuation(text),
            true,
            reason,
        );
    }

    (
        text.to_string(),
        false,
        "right_continuation_disallows_strip",
    )
}

fn collapse_duplicate_boundary_sentence_punctuation(
    text: &str,
    context: &TextInsertionContext,
    profile: SmartInsertionProfile,
) -> (String, bool, &'static str) {
    let Some(inserted_mark) = trailing_sentence_punctuation_char(text, profile) else {
        return (
            text.to_string(),
            false,
            "no_inserted_terminal_sentence_punctuation",
        );
    };

    let Some(right_char) = context.right_char else {
        return (text.to_string(), false, "no_right_boundary_char");
    };

    if right_char != inserted_mark {
        let whitespace_profile = matches!(
            profile,
            SmartInsertionProfile::CasedWhitespace | SmartInsertionProfile::UncasedWhitespace
        );
        let right_is_sentence_mark = is_sentence_terminator(right_char, profile);

        if right_is_sentence_mark && whitespace_profile {
            return (
                strip_single_trailing_sentence_punctuation(text),
                true,
                "conflicting_sentence_boundary_mark_prefer_right_boundary",
            );
        }

        if whitespace_profile && is_clause_boundary_punctuation(right_char, profile) {
            if matches!(
                sentence_mark_kind(inserted_mark, profile),
                Some(SentenceMarkKind::PeriodLike)
            ) && has_abbreviation_like_internal_dots(text, profile)
            {
                return (
                    text.to_string(),
                    false,
                    "clause_boundary_abbreviation_guard",
                );
            }

            return (
                strip_single_trailing_sentence_punctuation(text),
                true,
                "conflicting_clause_boundary_mark_prefer_right_boundary",
            );
        }

        return (text.to_string(), false, "right_boundary_mark_differs");
    }

    let reason = match sentence_mark_kind(inserted_mark, profile) {
        Some(SentenceMarkKind::PeriodLike) => "duplicate_period_boundary",
        Some(SentenceMarkKind::QuestionLike) => "duplicate_question_boundary",
        Some(SentenceMarkKind::ExclamationLike) => "duplicate_exclamation_boundary",
        None => "duplicate_boundary",
    };

    (
        strip_single_trailing_sentence_punctuation(text),
        true,
        reason,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InternalSpaceCompactionStrategy {
    HanBoundaryOnly,
    JapaneseMixedScript,
}

fn internal_space_compaction_strategy(
    normalized_language: &str,
) -> Option<InternalSpaceCompactionStrategy> {
    match normalized_language {
        "zh" | "zh-tw" | "yue" => Some(InternalSpaceCompactionStrategy::HanBoundaryOnly),
        "ja" => Some(InternalSpaceCompactionStrategy::JapaneseMixedScript),
        _ => None,
    }
}

fn is_cjk_han(c: char) -> bool {
    let code = c as u32;
    matches!(
        code,
        0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xF900..=0xFAFF
            | 0x20000..=0x2A6DF
            | 0x2A700..=0x2B73F
            | 0x2B740..=0x2B81F
            | 0x2B820..=0x2CEAF
            | 0x2CEB0..=0x2EBEF
            | 0x30000..=0x3134F
    )
}

fn is_cjk_punctuation(c: char) -> bool {
    matches!(
        c,
        '。' | '！'
            | '？'
            | '，'
            | '、'
            | '；'
            | '：'
            | '「'
            | '」'
            | '『'
            | '』'
            | '（'
            | '）'
            | '《'
            | '》'
            | '〈'
            | '〉'
            | '【'
            | '】'
            | '〔'
            | '〕'
            | '〖'
            | '〗'
            | '〘'
            | '〙'
            | '〚'
            | '〛'
            | '…'
            | '．'
            | '｡'
            | '｢'
            | '｣'
            | '﹁'
            | '﹂'
            | '﹃'
            | '﹄'
            | '﹙'
            | '﹚'
            | '﹛'
            | '﹜'
            | '﹝'
            | '﹞'
            | '﹣'
            | '－'
            | '﹔'
            | '﹕'
            | '﹖'
            | '﹗'
            | '﹐'
            | '﹑'
            | '﹒'
    )
}

fn is_ascii_token_char(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(
            c,
            '_' | '-' | '.' | '/' | '\\' | ':' | '@' | '#' | '+' | '=' | '~' | '%' | '&' | '$'
        )
}

fn is_cjk_compaction_boundary_char(c: char) -> bool {
    is_cjk_han(c) || is_cjk_punctuation(c)
}

fn is_japanese_hiragana(c: char) -> bool {
    let code = c as u32;
    matches!(code, 0x3040..=0x309F)
}

fn is_japanese_katakana(c: char) -> bool {
    let code = c as u32;
    matches!(code, 0x30A0..=0x30FF)
}

fn is_japanese_katakana_extensions(c: char) -> bool {
    let code = c as u32;
    matches!(code, 0x31F0..=0x31FF)
}

fn is_japanese_halfwidth_katakana(c: char) -> bool {
    let code = c as u32;
    matches!(code, 0xFF66..=0xFF9F)
}

fn is_japanese_iteration_mark(c: char) -> bool {
    matches!(c, '々' | '〻')
}

fn is_japanese_script_char(c: char) -> bool {
    is_cjk_han(c)
        || is_japanese_hiragana(c)
        || is_japanese_katakana(c)
        || is_japanese_katakana_extensions(c)
        || is_japanese_halfwidth_katakana(c)
        || is_japanese_iteration_mark(c)
}

fn is_japanese_compaction_boundary_char(c: char) -> bool {
    is_japanese_script_char(c) || is_cjk_punctuation(c)
}

fn compact_internal_whitespace_for_cjk(
    text: &str,
    strategy: InternalSpaceCompactionStrategy,
) -> (String, bool, &'static str, usize) {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return (text.to_string(), false, "no_whitespace_runs", 0);
    }

    let mut output = String::with_capacity(text.len());
    let mut i = 0usize;
    let mut saw_whitespace_run = false;
    let mut removed_chars = 0usize;
    let mut applied = false;

    while i < chars.len() {
        if !chars[i].is_whitespace() {
            output.push(chars[i]);
            i += 1;
            continue;
        }

        let run_start = i;
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        let run_end = i;
        let run = &chars[run_start..run_end];
        saw_whitespace_run = true;

        let left = run_start.checked_sub(1).map(|idx| chars[idx]);
        let right = if run_end < chars.len() {
            Some(chars[run_end])
        } else {
            None
        };

        let contains_line_break = run.iter().any(|c| matches!(*c, '\n' | '\r' | '\t'));
        let all_compactable_spaces = run.iter().all(|c| matches!(*c, ' ' | '\u{3000}'));
        let preserve_ascii_phrase = left
            .zip(right)
            .map(|(l, r)| is_ascii_token_char(l) && is_ascii_token_char(r))
            .unwrap_or(false);
        let compactable_boundary = left
            .zip(right)
            .map(|(l, r)| match strategy {
                InternalSpaceCompactionStrategy::HanBoundaryOnly => {
                    is_cjk_compaction_boundary_char(l) && is_cjk_compaction_boundary_char(r)
                }
                InternalSpaceCompactionStrategy::JapaneseMixedScript => {
                    let left_ascii = is_ascii_token_char(l);
                    let right_ascii = is_ascii_token_char(r);
                    let left_japanese = is_japanese_compaction_boundary_char(l);
                    let right_japanese = is_japanese_compaction_boundary_char(r);

                    (left_japanese && (right_japanese || right_ascii))
                        || (right_japanese && (left_japanese || left_ascii))
                }
            })
            .unwrap_or(false);

        if compactable_boundary
            && all_compactable_spaces
            && !contains_line_break
            && !preserve_ascii_phrase
        {
            removed_chars += run.len();
            applied = true;
            continue;
        }

        for c in run {
            output.push(*c);
        }
    }

    let reason = if !saw_whitespace_run {
        "no_whitespace_runs"
    } else if applied {
        match strategy {
            InternalSpaceCompactionStrategy::HanBoundaryOnly => "cjk_boundary_space_removed",
            InternalSpaceCompactionStrategy::JapaneseMixedScript => {
                "japanese_boundary_space_removed"
            }
        }
    } else {
        match strategy {
            InternalSpaceCompactionStrategy::HanBoundaryOnly => "no_cjk_boundary_space_removed",
            InternalSpaceCompactionStrategy::JapaneseMixedScript => {
                "no_japanese_boundary_space_removed"
            }
        }
    };

    (output, applied, reason, removed_chars)
}

pub(crate) fn prepare_text_for_paste(
    text: &str,
    smart_insertion_enabled: bool,
    insertion_context: Option<TextInsertionContext>,
    selected_language: &str,
) -> String {
    if !smart_insertion_enabled {
        return text.to_string();
    }

    let normalized_language = normalize_selected_language(selected_language);
    let (profile, profile_reason) = resolve_smart_insertion_profile(&normalized_language);
    if profile == SmartInsertionProfile::Conservative {
        let context_available = insertion_context.is_some();
        let last_significant = last_non_whitespace_char(text);
        let ends_with_whitespace = text
            .chars()
            .last()
            .map(|c| c.is_whitespace())
            .unwrap_or(false);
        let trailing_space_added = insertion_context
            .as_ref()
            .map(|context| {
                context.right_char.map(is_word_like).unwrap_or(false)
                    && last_significant.map(is_word_like).unwrap_or(false)
                    && !ends_with_whitespace
            })
            .unwrap_or(true);
        let prepared = if trailing_space_added {
            format!("{} ", text)
        } else {
            text.to_string()
        };
        let conservative_spacing_reason = if !context_available {
            "legacy_no_context"
        } else if trailing_space_added {
            "word_boundary_trailing_space"
        } else {
            "no_trailing_space_needed"
        };
        debug!(
            normalized_language = normalized_language.as_str(),
            insertion_profile = ?profile,
            profile_reason,
            fallback_mode = "conservative_profile",
            context_available,
            conservative_spacing_reason,
            trailing_space_added,
            "Applied conservative smart insertion fallback"
        );
        return prepared;
    }

    let Some(context) = insertion_context else {
        debug!(
            normalized_language = normalized_language.as_str(),
            insertion_profile = ?profile,
            profile_reason,
            context_available = false,
            fallback_mode = "context_unavailable",
            "Smart insertion context unavailable; using legacy trailing-space fallback"
        );
        return format!("{} ", text);
    };

    let sentence_start = is_sentence_start(&context, profile);
    let casing_enabled = profile == SmartInsertionProfile::CasedWhitespace;
    let spacing_enabled = matches!(
        profile,
        SmartInsertionProfile::CasedWhitespace | SmartInsertionProfile::UncasedWhitespace
    );
    let (sanitized, punctuation_strip_applied, punctuation_strip_reason) =
        sanitize_trailing_sentence_punctuation(text, &context, profile);
    let (
        mut prepared,
        duplicate_punctuation_collapse_applied,
        duplicate_punctuation_collapse_reason,
    ) = collapse_duplicate_boundary_sentence_punctuation(&sanitized, &context, profile);
    let mut internal_space_compaction_applied = false;
    let mut internal_space_compaction_reason = "not_applicable";
    let mut internal_space_compaction_removed_chars = 0usize;

    if let Some(compaction_strategy) = internal_space_compaction_strategy(&normalized_language) {
        let (compacted, applied, reason, removed_chars) =
            compact_internal_whitespace_for_cjk(&prepared, compaction_strategy);
        prepared = compacted;
        internal_space_compaction_applied = applied;
        internal_space_compaction_reason = reason;
        internal_space_compaction_removed_chars = removed_chars;
    }

    let mut casing_action = "none";

    if casing_enabled && sentence_start {
        if let Some(capitalized) = uppercase_first_alphabetic(&prepared) {
            prepared = capitalized;
            casing_action = "capitalized";
        }
    } else if casing_enabled && is_title_like_start(&prepared) {
        if let Some(lowercased) = lowercase_first_alphabetic(&prepared) {
            prepared = lowercased;
            casing_action = "decapitalized";
        }
    }

    let first_significant = first_non_whitespace_char(&prepared);
    let last_significant = last_non_whitespace_char(&prepared);
    let starts_with_whitespace = prepared
        .chars()
        .next()
        .map(|c| c.is_whitespace())
        .unwrap_or(false);

    let needs_leading_space_word_boundary = spacing_enabled
        && context.left_char.map(is_word_like).unwrap_or(false)
        && first_significant.map(is_word_like).unwrap_or(false)
        && !starts_with_whitespace;
    let needs_leading_space_sentence_boundary = spacing_enabled
        && context
            .left_char
            .map(|c| is_sentence_terminator(c, profile))
            .unwrap_or(false)
        && first_significant.map(is_word_like).unwrap_or(false)
        && !starts_with_whitespace;
    let needs_leading_space =
        needs_leading_space_word_boundary || needs_leading_space_sentence_boundary;

    let leading_space_reason = if needs_leading_space_word_boundary {
        "word_boundary"
    } else if needs_leading_space_sentence_boundary {
        "sentence_punctuation_boundary"
    } else {
        "none"
    };

    if needs_leading_space {
        prepared.insert(0, ' ');
    }

    let ends_with_whitespace = prepared
        .chars()
        .last()
        .map(|c| c.is_whitespace())
        .unwrap_or(false);
    let needs_trailing_space_word_boundary = spacing_enabled
        && context.right_char.map(is_word_like).unwrap_or(false)
        && last_significant.map(is_word_like).unwrap_or(false)
        && !ends_with_whitespace;
    let needs_trailing_space_sentence_boundary = spacing_enabled
        && context.right_char.map(is_word_like).unwrap_or(false)
        && first_significant.map(is_word_like).unwrap_or(false)
        && last_significant
            .map(|c| is_sentence_terminator(c, profile))
            .unwrap_or(false)
        && !ends_with_whitespace;
    let needs_trailing_space =
        needs_trailing_space_word_boundary || needs_trailing_space_sentence_boundary;
    let trailing_space_reason = if needs_trailing_space_word_boundary {
        "word_boundary"
    } else if needs_trailing_space_sentence_boundary {
        "sentence_punctuation_boundary"
    } else {
        "none"
    };
    if needs_trailing_space {
        prepared.push(' ');
    }

    debug!(
        normalized_language = normalized_language.as_str(),
        insertion_profile = ?profile,
        profile_reason,
        context_available = true,
        fallback_mode = "none",
        casing_enabled,
        spacing_enabled,
        has_selection = context.has_selection,
        sentence_start,
        casing_action,
        punctuation_strip_applied,
        punctuation_strip_reason,
        duplicate_punctuation_collapse_applied,
        duplicate_punctuation_collapse_reason,
        internal_space_compaction_applied,
        internal_space_compaction_reason,
        internal_space_compaction_removed_chars,
        leading_space_added = needs_leading_space,
        leading_space_reason,
        trailing_space_reason,
        trailing_space_added = needs_trailing_space,
        "Applied smart transcript insertion formatting"
    );

    prepared
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accessibility::TextInsertionContext;

    fn context(
        left_char: Option<char>,
        left_non_whitespace_char: Option<char>,
        right_char: Option<char>,
        right_non_whitespace_char: Option<char>,
        has_selection: bool,
    ) -> TextInsertionContext {
        TextInsertionContext {
            left_char,
            left_non_whitespace_char,
            right_char,
            right_non_whitespace_char,
            has_selection,
        }
    }

    fn prepare_text_for_paste(
        text: &str,
        smart_insertion_enabled: bool,
        insertion_context: Option<TextInsertionContext>,
    ) -> String {
        super::prepare_text_for_paste(text, smart_insertion_enabled, insertion_context, "en")
    }

    fn prepare_text_for_paste_with_language(
        text: &str,
        smart_insertion_enabled: bool,
        insertion_context: Option<TextInsertionContext>,
        selected_language: &str,
    ) -> String {
        super::prepare_text_for_paste(
            text,
            smart_insertion_enabled,
            insertion_context,
            selected_language,
        )
    }

    #[test]
    fn collapse_spaced_punctuation_artifacts_fixes_sentence_to_comma() {
        let output = collapse_spaced_punctuation_artifacts(
            "Today there are five hundred things I want to do. , let's try to make it a good day.",
        );
        assert_eq!(
            output,
            "Today there are five hundred things I want to do, let's try to make it a good day."
        );
    }

    #[test]
    fn collapse_spaced_punctuation_artifacts_fixes_duplicate_full_stop() {
        let output = collapse_spaced_punctuation_artifacts("Let's try to make it a good day. .");
        assert_eq!(output, "Let's try to make it a good day.");
    }

    #[test]
    fn collapse_spaced_punctuation_artifacts_keeps_regular_text_unchanged() {
        let input = "Yesterday I spent $10. Today is better.";
        assert_eq!(collapse_spaced_punctuation_artifacts(input), input);
    }

    #[test]
    fn collapse_spaced_punctuation_artifacts_fixes_duplicate_hyphen_dash_variants() {
        let hyphen_output = collapse_spaced_punctuation_artifacts("alpha - - beta");
        let en_dash_output = collapse_spaced_punctuation_artifacts("alpha – – beta");
        let em_dash_output = collapse_spaced_punctuation_artifacts("alpha — — beta");

        assert_eq!(hyphen_output, "alpha - beta");
        assert_eq!(en_dash_output, "alpha – beta");
        assert_eq!(em_dash_output, "alpha — beta");
    }

    #[test]
    fn collapse_spaced_punctuation_artifacts_preserves_negative_number_expression() {
        let input = "x - -1";
        assert_eq!(collapse_spaced_punctuation_artifacts(input), input);
    }

    #[test]
    fn collapse_spaced_punctuation_artifacts_preserves_cli_double_dash() {
        let input = "run --help";
        assert_eq!(collapse_spaced_punctuation_artifacts(input), input);
    }

    #[test]
    fn collapse_spaced_punctuation_artifacts_handles_start_of_text_dash_duplicate() {
        let input = "- - alpha";
        assert_eq!(collapse_spaced_punctuation_artifacts(input), "- alpha");
    }

    #[test]
    fn collapse_spaced_punctuation_artifacts_handles_end_of_text_dash_duplicate() {
        let hyphen_output = collapse_spaced_punctuation_artifacts("alpha - -");
        let en_dash_output = collapse_spaced_punctuation_artifacts("alpha – –");
        let em_dash_output = collapse_spaced_punctuation_artifacts("alpha — —");

        assert_eq!(hyphen_output, "alpha -");
        assert_eq!(en_dash_output, "alpha –");
        assert_eq!(em_dash_output, "alpha —");
    }

    #[test]
    fn language_normalization_and_profile_resolution() {
        let cases = [
            ("en-US", "en", SmartInsertionProfile::CasedWhitespace),
            ("pt-BR", "pt", SmartInsertionProfile::CasedWhitespace),
            ("zh-Hans", "zh", SmartInsertionProfile::NoBoundarySpacing),
            ("zh-TW", "zh-tw", SmartInsertionProfile::NoBoundarySpacing),
            ("tr", "tr", SmartInsertionProfile::Conservative),
            ("AUTO", "auto", SmartInsertionProfile::Conservative),
        ];

        for (input, expected_normalized, expected_profile) in cases {
            let normalized = normalize_selected_language(input);
            assert_eq!(normalized, expected_normalized);
            let (profile, _reason) = resolve_smart_insertion_profile(&normalized);
            assert_eq!(profile, expected_profile);
        }
    }

    #[test]
    fn auto_language_uses_conservative_fallback() {
        let output = prepare_text_for_paste_with_language(
            "hello",
            true,
            Some(context(Some('o'), Some('o'), Some('w'), Some('w'), false)),
            "auto",
        );
        assert_eq!(output, "hello ");
    }

    #[test]
    fn unknown_language_uses_conservative_fallback() {
        let output = prepare_text_for_paste_with_language(
            "hello",
            true,
            Some(context(Some('o'), Some('o'), Some('w'), Some('w'), false)),
            "haw",
        );
        assert_eq!(output, "hello ");
    }

    #[test]
    fn conservative_profile_does_not_add_space_before_punctuation() {
        let output = prepare_text_for_paste_with_language(
            "hello",
            true,
            Some(context(Some('o'), Some('o'), Some(','), Some(','), false)),
            "auto",
        );
        assert_eq!(output, "hello");
    }

    #[test]
    fn conservative_profile_preserves_existing_trailing_space() {
        let output = prepare_text_for_paste_with_language(
            "hello ",
            true,
            Some(context(Some('o'), Some('o'), Some('w'), Some('w'), false)),
            "auto",
        );
        assert_eq!(output, "hello ");
    }

    #[test]
    fn conservative_profile_uses_legacy_trailing_space_when_context_unavailable() {
        let output = prepare_text_for_paste_with_language("hello", true, None, "auto");
        assert_eq!(output, "hello ");
    }

    #[test]
    fn turkish_language_uses_conservative_fallback() {
        let output = prepare_text_for_paste_with_language(
            "merhaba",
            true,
            Some(context(Some('a'), Some('a'), Some('d'), Some('d'), false)),
            "tr",
        );
        assert_eq!(output, "merhaba ");
    }

    #[test]
    fn unicode_digit_continuation_strips_trailing_punctuation() {
        let arabic_indic_output = prepare_text_for_paste(
            "hello?",
            true,
            Some(context(Some(' '), Some('e'), Some(' '), Some('٣'), false)),
        );
        assert_eq!(arabic_indic_output, "hello");

        let fullwidth_output = prepare_text_for_paste(
            "hello?",
            true,
            Some(context(Some(' '), Some('e'), Some(' '), Some('３'), false)),
        );
        assert_eq!(fullwidth_output, "hello");
    }

    #[test]
    fn mid_sentence_decapitalize() {
        let output = prepare_text_for_paste(
            "Word",
            true,
            Some(context(Some(' '), Some('e'), None, None, false)),
        );
        assert_eq!(output, "word");
    }

    #[test]
    fn new_sentence_capitalize() {
        let output = prepare_text_for_paste(
            "hello",
            true,
            Some(context(Some(' '), Some('.'), None, None, false)),
        );
        assert_eq!(output, "Hello");
    }

    #[test]
    fn leading_space_added_between_words() {
        let output = prepare_text_for_paste(
            "new",
            true,
            Some(context(Some('o'), Some('o'), None, None, false)),
        );
        assert_eq!(output, " new");
    }

    #[test]
    fn trailing_space_added_between_words() {
        let output = prepare_text_for_paste(
            "new",
            true,
            Some(context(Some(' '), Some('o'), Some('w'), Some('w'), false)),
        );
        assert_eq!(output, "new ");
    }

    #[test]
    fn no_space_before_punctuation() {
        let output = prepare_text_for_paste(
            ",",
            true,
            Some(context(Some('o'), Some('o'), Some('w'), Some('w'), false)),
        );
        assert_eq!(output, ",");
    }

    #[test]
    fn acronym_preserved() {
        let output = prepare_text_for_paste(
            "NASA",
            true,
            Some(context(Some(' '), Some('e'), None, None, false)),
        );
        assert_eq!(output, "NASA");
    }

    #[test]
    fn context_unavailable_legacy_fallback() {
        let output = prepare_text_for_paste("hello", true, None);
        assert_eq!(output, "hello ");
    }

    #[test]
    fn setting_disabled_no_transform() {
        let output = prepare_text_for_paste(
            "Hello",
            false,
            Some(context(Some('o'), Some('o'), Some('w'), Some('w'), false)),
        );
        assert_eq!(output, "Hello");
    }

    #[test]
    fn selection_strips_trailing_question_mark_on_lowercase_continuation() {
        let output = prepare_text_for_paste(
            "hello?",
            true,
            Some(context(Some(' '), Some('e'), Some(' '), Some('s'), true)),
        );
        assert_eq!(output, "hello");
    }

    #[test]
    fn selection_mid_sentence_strips_trailing_period() {
        let output = prepare_text_for_paste(
            "hello.",
            true,
            Some(context(Some(' '), Some('e'), Some(' '), Some('s'), true)),
        );
        assert_eq!(output, "hello");
    }

    #[test]
    fn selection_mid_sentence_strips_trailing_exclamation() {
        let output = prepare_text_for_paste(
            "hello!",
            true,
            Some(context(Some(' '), Some('e'), Some(' '), Some('s'), true)),
        );
        assert_eq!(output, "hello");
    }

    #[test]
    fn selection_preserves_trailing_punctuation_on_uppercase_continuation() {
        let output = prepare_text_for_paste(
            "hello?",
            true,
            Some(context(Some(' '), Some('e'), Some('S'), Some('S'), true)),
        );
        assert_eq!(output, "hello? ");
    }

    #[test]
    fn selection_preserves_abbreviation_like_internal_dot() {
        let output = prepare_text_for_paste(
            "e.g.",
            true,
            Some(context(Some(' '), Some('e'), Some('s'), Some('s'), true)),
        );
        assert_eq!(output, "e.g. ");
    }

    #[test]
    fn non_selection_strips_trailing_question_mark_on_lowercase_continuation() {
        let output = prepare_text_for_paste(
            "hello?",
            true,
            Some(context(Some(' '), Some('e'), Some(' '), Some('s'), false)),
        );
        assert_eq!(output, "hello");
    }

    #[test]
    fn non_selection_strips_trailing_period_on_digit_continuation() {
        let output = prepare_text_for_paste(
            "hello.",
            true,
            Some(context(Some(' '), Some('e'), Some(' '), Some('2'), false)),
        );
        assert_eq!(output, "hello");
    }

    #[test]
    fn non_selection_preserves_trailing_question_mark_on_uppercase_continuation() {
        let output = prepare_text_for_paste(
            "hello?",
            true,
            Some(context(Some(' '), Some('e'), Some('S'), Some('S'), false)),
        );
        assert_eq!(output, "hello? ");
    }

    #[test]
    fn non_selection_preserves_abbreviation_like_internal_dot() {
        let output = prepare_text_for_paste(
            "e.g.",
            true,
            Some(context(Some(' '), Some('e'), Some('s'), Some('s'), false)),
        );
        assert_eq!(output, "e.g. ");
    }

    #[test]
    fn punctuation_only_insert_does_not_add_sentence_boundary_trailing_space() {
        let output = prepare_text_for_paste(
            "?",
            true,
            Some(context(Some('d'), Some('d'), Some('S'), Some('S'), false)),
        );
        assert_eq!(output, "?");
    }

    #[test]
    fn no_trailing_space_when_right_boundary_is_whitespace_after_punctuation() {
        let output = prepare_text_for_paste(
            "hello?",
            true,
            Some(context(Some(' '), Some('e'), Some(' '), Some('S'), false)),
        );
        assert_eq!(output, "hello?");
    }

    #[test]
    fn collapse_duplicate_period_at_right_boundary() {
        let output = prepare_text_for_paste(
            "hello.",
            true,
            Some(context(Some(' '), Some('e'), Some('.'), Some('.'), false)),
        );
        assert_eq!(output, "hello");
    }

    #[test]
    fn collapse_duplicate_question_mark_at_right_boundary() {
        let output = prepare_text_for_paste(
            "hello?",
            true,
            Some(context(Some(' '), Some('e'), Some('?'), Some('?'), false)),
        );
        assert_eq!(output, "hello");
    }

    #[test]
    fn collapse_duplicate_exclamation_at_right_boundary() {
        let output = prepare_text_for_paste(
            "hello!",
            true,
            Some(context(Some(' '), Some('e'), Some('!'), Some('!'), false)),
        );
        assert_eq!(output, "hello");
    }

    #[test]
    fn collapse_conflicting_boundary_mark_prefers_right_boundary() {
        let output = prepare_text_for_paste(
            "hello?",
            true,
            Some(context(Some(' '), Some('e'), Some('.'), Some('.'), false)),
        );
        assert_eq!(output, "hello");
    }

    #[test]
    fn collapse_conflicting_exclamation_before_question_boundary() {
        let output = prepare_text_for_paste(
            "hello!",
            true,
            Some(context(Some(' '), Some('e'), Some('?'), Some('?'), false)),
        );
        assert_eq!(output, "hello");
    }

    #[test]
    fn collapse_conflicting_period_before_question_boundary() {
        let output = prepare_text_for_paste(
            "hello.",
            true,
            Some(context(Some(' '), Some('e'), Some('?'), Some('?'), false)),
        );
        assert_eq!(output, "hello");
    }

    #[test]
    fn collapse_sentence_period_before_comma_boundary_in_cased_profile() {
        let output = prepare_text_for_paste(
            "there.",
            true,
            Some(context(Some(' '), Some('o'), Some(','), Some(','), false)),
        );
        assert_eq!(output, "there");
    }

    #[test]
    fn collapse_sentence_question_before_comma_boundary_in_cased_profile() {
        let output = prepare_text_for_paste(
            "hello?",
            true,
            Some(context(Some(' '), Some('o'), Some(','), Some(','), false)),
        );
        assert_eq!(output, "hello");
    }

    #[test]
    fn collapse_sentence_exclamation_before_semicolon_boundary_in_cased_profile() {
        let output = prepare_text_for_paste(
            "hello!",
            true,
            Some(context(Some(' '), Some('o'), Some(';'), Some(';'), false)),
        );
        assert_eq!(output, "hello");
    }

    #[test]
    fn preserve_abbreviation_period_before_comma_boundary() {
        let output = prepare_text_for_paste(
            "e.g.",
            true,
            Some(context(Some(' '), Some('o'), Some(','), Some(','), false)),
        );
        assert_eq!(output, "e.g.");
    }

    #[test]
    fn preserve_abbreviation_period_before_colon_boundary() {
        let output = prepare_text_for_paste(
            "e.g.",
            true,
            Some(context(Some(' '), Some('o'), Some(':'), Some(':'), false)),
        );
        assert_eq!(output, "e.g.");
    }

    #[test]
    fn punctuation_only_insert_before_clause_boundary_strips_inserted_mark() {
        let output = prepare_text_for_paste(
            "?",
            true,
            Some(context(Some('o'), Some('o'), Some(','), Some(','), false)),
        );
        assert_eq!(output, "");
    }

    #[test]
    fn leading_space_added_after_period_boundary() {
        let output = prepare_text_for_paste(
            "what",
            true,
            Some(context(Some('.'), Some('.'), None, None, false)),
        );
        assert_eq!(output, " What");
    }

    #[test]
    fn leading_space_added_after_question_boundary() {
        let output = prepare_text_for_paste(
            "what",
            true,
            Some(context(Some('?'), Some('?'), None, None, false)),
        );
        assert_eq!(output, " What");
    }

    #[test]
    fn leading_space_added_after_exclamation_boundary() {
        let output = prepare_text_for_paste(
            "what",
            true,
            Some(context(Some('!'), Some('!'), None, None, false)),
        );
        assert_eq!(output, " What");
    }

    #[test]
    fn uncased_profile_skips_casing_transform() {
        let output = prepare_text_for_paste_with_language(
            "Word",
            true,
            Some(context(Some(' '), Some('.'), None, None, false)),
            "ko",
        );
        assert_eq!(output, "Word");
    }

    #[test]
    fn no_boundary_spacing_profile_avoids_word_spacing() {
        let output = prepare_text_for_paste_with_language(
            "世界",
            true,
            Some(context(
                Some('你'),
                Some('你'),
                Some('好'),
                Some('好'),
                false,
            )),
            "zh",
        );
        assert_eq!(output, "世界");
    }

    #[test]
    fn no_boundary_profile_still_has_no_boundary_spacing() {
        let output = prepare_text_for_paste_with_language(
            "世界。",
            true,
            Some(context(Some('你'), Some('你'), Some('!'), Some('!'), false)),
            "zh",
        );
        assert_eq!(output, "世界。");
    }

    #[test]
    fn zh_compacts_internal_han_space() {
        let output = prepare_text_for_paste_with_language(
            "是 請",
            true,
            Some(context(
                Some('你'),
                Some('你'),
                Some('好'),
                Some('好'),
                false,
            )),
            "zh",
        );
        assert_eq!(output, "是請");
    }

    #[test]
    fn zh_tw_compacts_internal_han_space() {
        let output = prepare_text_for_paste_with_language(
            "是 請",
            true,
            Some(context(
                Some('你'),
                Some('你'),
                Some('好'),
                Some('好'),
                false,
            )),
            "zh-Hant",
        );
        assert_eq!(output, "是請");
    }

    #[test]
    fn yue_compacts_internal_han_space() {
        let output = prepare_text_for_paste_with_language(
            "係 唔係",
            true,
            Some(context(
                Some('你'),
                Some('你'),
                Some('好'),
                Some('好'),
                false,
            )),
            "yue-HK",
        );
        assert_eq!(output, "係唔係");
    }

    #[test]
    fn zh_compacts_sentence_boundary_space() {
        let output = prepare_text_for_paste_with_language(
            "你好。 再见",
            true,
            Some(context(
                Some('你'),
                Some('你'),
                Some('好'),
                Some('好'),
                false,
            )),
            "zh-Hans",
        );
        assert_eq!(output, "你好。再见");
    }

    #[test]
    fn zh_preserves_ascii_phrase_space() {
        let output = prepare_text_for_paste_with_language(
            "Open AI",
            true,
            Some(context(
                Some('你'),
                Some('你'),
                Some('好'),
                Some('好'),
                false,
            )),
            "zh",
        );
        assert_eq!(output, "Open AI");
    }

    #[test]
    fn zh_preserves_url_like_spacing() {
        let output = prepare_text_for_paste_with_language(
            "cdn example.com",
            true,
            Some(context(
                Some('你'),
                Some('你'),
                Some('好'),
                Some('好'),
                false,
            )),
            "zh",
        );
        assert_eq!(output, "cdn example.com");
    }

    #[test]
    fn ja_compacts_japanese_mixed_script_spacing() {
        let output = prepare_text_for_paste_with_language(
            "私 は コーヒー を 飲みました。",
            true,
            Some(context(Some('こ'), Some('こ'), None, None, false)),
            "ja",
        );
        assert_eq!(output, "私はコーヒーを飲みました。");
    }

    #[test]
    fn ja_compacts_ascii_to_japanese_boundary_spacing() {
        let output = prepare_text_for_paste_with_language(
            "iPhone 16 を 買った",
            true,
            Some(context(Some('こ'), Some('こ'), None, None, false)),
            "ja",
        );
        assert_eq!(output, "iPhone 16を買った");
    }

    #[test]
    fn ja_compacts_numeric_to_japanese_spacing() {
        let output = prepare_text_for_paste_with_language(
            "3 時間 待ちました",
            true,
            Some(context(Some('こ'), Some('こ'), None, None, false)),
            "ja",
        );
        assert_eq!(output, "3時間待ちました");
    }

    #[test]
    fn ja_compacts_katakana_to_hiragana_spacing() {
        let output = prepare_text_for_paste_with_language(
            "テスト です",
            true,
            Some(context(Some('こ'), Some('こ'), None, None, false)),
            "ja",
        );
        assert_eq!(output, "テストです");
    }

    #[test]
    fn ja_compacts_kanji_to_katakana_spacing() {
        let output = prepare_text_for_paste_with_language(
            "東京 タワー",
            true,
            Some(context(Some('こ'), Some('こ'), None, None, false)),
            "ja",
        );
        assert_eq!(output, "東京タワー");
    }

    #[test]
    fn ja_compacts_single_ascii_tokens_around_particle() {
        let output = prepare_text_for_paste_with_language(
            "A と B",
            true,
            Some(context(Some('こ'), Some('こ'), None, None, false)),
            "ja",
        );
        assert_eq!(output, "AとB");
    }

    #[test]
    fn ja_preserves_ascii_phrase_but_compacts_japanese_boundaries() {
        let output = prepare_text_for_paste_with_language(
            "Open AI の API",
            true,
            Some(context(Some('こ'), Some('こ'), None, None, false)),
            "ja",
        );
        assert_eq!(output, "Open AIのAPI");
    }

    #[test]
    fn ja_preserves_line_breaks_in_internal_whitespace_runs() {
        let output = prepare_text_for_paste_with_language(
            "私は\nコーヒー",
            true,
            Some(context(Some('こ'), Some('こ'), None, None, false)),
            "ja",
        );
        assert_eq!(output, "私は\nコーヒー");
    }

    #[test]
    fn ja_preserves_emoji_spacing() {
        let output = prepare_text_for_paste_with_language(
            "了解 👍 です",
            true,
            Some(context(Some('こ'), Some('こ'), None, None, false)),
            "ja",
        );
        assert_eq!(output, "了解 👍 です");
    }

    #[test]
    fn ja_compacts_halfwidth_katakana_boundaries() {
        let output = prepare_text_for_paste_with_language(
            "ﾃｽﾄ です",
            true,
            Some(context(Some('こ'), Some('こ'), None, None, false)),
            "ja",
        );
        assert_eq!(output, "ﾃｽﾄです");
    }

    #[test]
    fn uncased_profile_handles_arabic_question_mark() {
        let output = prepare_text_for_paste_with_language(
            "مرحبا؟",
            true,
            Some(context(Some(' '), Some('ا'), Some(' '), Some('س'), false)),
            "ar",
        );
        assert_eq!(output, "مرحبا");
    }

    #[test]
    fn no_boundary_profile_collapses_duplicate_cjk_period() {
        let output = prepare_text_for_paste_with_language(
            "你好。",
            true,
            Some(context(
                Some(' '),
                Some('好'),
                Some('。'),
                Some('。'),
                false,
            )),
            "zh-Hans",
        );
        assert_eq!(output, "你好");
    }

    #[test]
    fn no_boundary_profile_preserves_abbreviation_like_internal_dot_with_cjk_terminal() {
        let output = prepare_text_for_paste_with_language(
            "U.S.A。",
            true,
            Some(context(Some(' '), Some('好'), Some(' '), Some('字'), false)),
            "zh-Hans",
        );
        assert_eq!(output, "U.S.A。");
    }

    #[test]
    fn uncased_profile_preserves_abbreviation_like_internal_dot_with_arabic_terminal() {
        let output = prepare_text_for_paste_with_language(
            "e.g؟",
            true,
            Some(context(Some(' '), Some('ا'), Some('س'), Some('س'), false)),
            "ar",
        );
        assert_eq!(output, "e.g؟ ");
    }

    #[test]
    fn uncased_profile_conflicting_mark_prefers_right_boundary() {
        let output = prepare_text_for_paste_with_language(
            "مرحبا؟",
            true,
            Some(context(Some(' '), Some('ا'), Some('.'), Some('.'), false)),
            "ar",
        );
        assert_eq!(output, "مرحبا");
    }

    #[test]
    fn uncased_profile_collapses_arabic_question_before_arabic_comma_boundary() {
        let output = prepare_text_for_paste_with_language(
            "كيف؟",
            true,
            Some(context(Some(' '), Some('ا'), Some('،'), Some('،'), false)),
            "ar",
        );
        assert_eq!(output, "كيف");
    }

    #[test]
    fn conservative_profile_clause_boundary_conflict_unchanged() {
        let output = prepare_text_for_paste_with_language(
            "hello.",
            true,
            Some(context(Some(' '), Some('o'), Some(','), Some(','), false)),
            "auto",
        );
        assert_eq!(output, "hello.");
    }

    #[test]
    fn no_boundary_profile_clause_boundary_conflict_unchanged() {
        let output = prepare_text_for_paste_with_language(
            "你好。",
            true,
            Some(context(
                Some('你'),
                Some('你'),
                Some('，'),
                Some('，'),
                false,
            )),
            "zh-Hans",
        );
        assert_eq!(output, "你好。");
    }

    #[test]
    fn no_boundary_profile_conflicting_mark_unchanged() {
        let output = prepare_text_for_paste_with_language(
            "你好？",
            true,
            Some(context(
                Some(' '),
                Some('好'),
                Some('。'),
                Some('。'),
                false,
            )),
            "zh-Hans",
        );
        assert_eq!(output, "你好？");
    }

    #[test]
    fn conservative_profile_unchanged_for_punctuation_endings() {
        let output = prepare_text_for_paste_with_language(
            "hello?",
            true,
            Some(context(Some(' '), Some('e'), Some('S'), Some('S'), false)),
            "auto",
        );
        assert_eq!(output, "hello?");
    }
}
