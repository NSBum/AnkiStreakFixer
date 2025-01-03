
pub fn log(verbose: bool, message: &str) {
    if verbose {
        println!("[VERBOSE] {}", message);
    }
}

pub fn replace_deck_delimiter(deck_name: &str) -> String {
    deck_name.replace('\u{001F}', "::")
}

pub fn red_text(text: &str) -> String {
    format!("\x1b[31m{}\x1b[0m", text)
}

pub fn green_text(text: &str) -> String {
    format!("\x1b[32m{}\x1b[0m", text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_deck_delimiter_single_occurrence() {
        let input = "Deck\u{001F}SubDeck";
        let expected = "Deck::SubDeck";
        assert_eq!(replace_deck_delimiter(input), expected);
    }

    #[test]
    fn test_replace_deck_delimiter_multiple_occurrences() {
        let input = "Deck\u{001F}SubDeck\u{001F}SubSubDeck";
        let expected = "Deck::SubDeck::SubSubDeck";
        assert_eq!(replace_deck_delimiter(input), expected);
    }

    #[test]
    fn test_replace_deck_delimiter_no_occurrence() {
        let input = "Deck::SubDeck";
        let expected = "Deck::SubDeck";
        assert_eq!(replace_deck_delimiter(input), expected);
    }

    #[test]
    fn test_replace_deck_delimiter_empty_string() {
        let input = "";
        let expected = "";
        assert_eq!(replace_deck_delimiter(input), expected);
    }
}
