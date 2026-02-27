/// Normalize dictionary text into a matching key.
///
/// This mirrors matcher normalization so settings migration and runtime matching
/// use exactly the same canonical form for length and equality checks.
pub fn normalize_for_dictionary_matching(text: &str) -> String {
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

pub fn normalized_dictionary_len(text: &str) -> usize {
    normalize_for_dictionary_matching(text).chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_symbols_and_punctuation() {
        assert_eq!(normalize_for_dictionary_matching("C++"), "cplusplus");
        assert_eq!(normalize_for_dictionary_matching("Node.js"), "nodejs");
        assert_eq!(normalize_for_dictionary_matching("R&D"), "randd");
    }

    #[test]
    fn normalized_length_counts_characters_not_utf8_bytes() {
        assert_eq!(normalized_dictionary_len("C++"), 9);
        assert_eq!(normalized_dictionary_len("猫咪"), 2);
    }
}
