use unicode_segmentation::UnicodeSegmentation;
use unicode_script::{Script, UnicodeScript};  // Note the addition of Script

pub fn detect_concatenated_text(text: &str) -> bool {
    let graphemes: Vec<&str> = text.graphemes(true).collect();

    if graphemes.len() < 4 {
        return false;
    }

    let mut current_script_group = None;
    let mut script_changes = 0;
    let mut consecutive_same_script = 0;
    let mut seen_japanese = false;
    let mut seen_chinese = false;

    fn is_japanese_specific(script: Script) -> bool {
        matches!(script, Script::Hiragana | Script::Katakana)
    }

    for grapheme in graphemes {
        if let Some(ch) = grapheme.chars().next() {
            let script = ch.script();

            // Skip common punctuation and spaces
            if matches!(script, Script::Common | Script::Inherited) {
                continue;
            }

            // Track if we've seen Japanese-specific or Han characters
            if is_japanese_specific(script) {
                seen_japanese = true;
            } else if script == Script::Han {
                if seen_japanese {
                    // If we've seen Japanese-specific scripts, treat Han as part of Japanese
                    seen_chinese = false;
                } else {
                    // Otherwise, treat it as Chinese
                    seen_chinese = true;
                }
            }

            // Determine script group
            let script_group = if is_japanese_specific(script) || (script == Script::Han && seen_japanese) {
                Script::Hiragana  // Japanese script group
            } else if script == Script::Han && seen_chinese {
                Script::Han       // Chinese script group
            } else {
                script
            };

            match current_script_group {
                None => {
                    current_script_group = Some(script_group);
                    consecutive_same_script = 1;
                }
                Some(prev_group) if prev_group != script_group => {
                    if consecutive_same_script >= 3 {
                        script_changes += 1;
                        current_script_group = Some(script_group);
                        consecutive_same_script = 1;
                    }
                }
                Some(_) => {
                    consecutive_same_script += 1;
                }
            }
        }
    }

    script_changes > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_cyrillic_latin_with_separator() {
        let text = "Словарный запас\u{1F}Vocabulary added today";
        assert!(detect_concatenated_text(text));
    }

    #[test]
    fn detects_cyrillic_latin_without_separator() {
        let text = "Словарный запасVocabulary added today";
        assert!(detect_concatenated_text(text));
    }

    #[test]
    fn ignores_single_script_cyrillic() {
        let text = "Словарный запас";
        assert!(!detect_concatenated_text(text));
    }

    #[test]
    fn ignores_single_script_latin() {
        let text = "Vocabulary only";
        assert!(!detect_concatenated_text(text));
    }

    #[test]
    fn ignores_short_input() {
        let text = "Hi";
        assert!(!detect_concatenated_text(text));
    }

    #[test]
    fn handles_punctuation_between_scripts() {
        let text = "Словарный запас - Vocabulary";
        assert!(detect_concatenated_text(text));
    }

    #[test]
    fn handles_numbers_between_scripts() {
        let text = "Словарный 123 - Vocabulary";
        assert!(detect_concatenated_text(text));
    }

    #[test]
    fn handles_multiple_separators() {
        let text = "Словарный\u{1F}запас\u{1F}Vocabulary";
        assert!(detect_concatenated_text(text));
    }

    // Japanese tests
    #[test]
    fn detects_japanese_latin_with_separator() {
        let text = "こんにちは世界\u{1F}Hello world";
        assert!(detect_concatenated_text(text));
    }

    #[test]
    fn ignores_single_script_japanese() {
        let text = "こんにちは世界";
        assert!(!detect_concatenated_text(text));
    }

    #[test]
    fn detects_japanese_mixed_with_kanji() {
        // Mixed hiragana and kanji is normal in Japanese, should NOT detect as concatenated
        let text = "私は日本語を話します";
        assert!(!detect_concatenated_text(text));
    }

    // Chinese tests
    #[test]
    fn detects_chinese_latin_with_separator() {
        let text = "你好世界\u{1F}Hello world";
        assert!(detect_concatenated_text(text));
    }

    #[test]
    fn ignores_single_script_chinese() {
        let text = "你好世界";
        assert!(!detect_concatenated_text(text));
    }

    // Hebrew tests
    #[test]
    fn detects_hebrew_latin_with_separator() {
        let text = "שָׁלוֹם עוֹלָם\u{1F}Hello world";
        assert!(detect_concatenated_text(text));
    }

    #[test]
    fn ignores_single_script_hebrew() {
        let text = "שָׁלוֹם עוֹלָם";
        assert!(!detect_concatenated_text(text));
    }

    #[test]
    fn handles_hebrew_with_niqqud() {
        // Hebrew with niqqud (vowel points) should still count as single script
        let text = "שָׁלוֹם";
        assert!(!detect_concatenated_text(text));
    }

    // Mixed script tests
    #[test]
    fn detects_chinese_japanese_with_separator() {
        let text = "你好世界\u{1F}こんにちは";
        assert!(detect_concatenated_text(text));
    }

    #[test]
    fn detects_hebrew_japanese_with_separator() {
        let text = "שָׁלוֹם\u{1F}こんにちは";
        assert!(detect_concatenated_text(text));
    }
}