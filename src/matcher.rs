use colored::Colorize;
use regex::Regex;
use std::fmt::{self, Display, Formatter};

#[macro_export]
/// Truncate a string to the first `n` characters, or return the string if it is shorter than `n`.
macro_rules! first_n_chars {
    ($s:expr, $n:expr) => {{
        let s: &str = $s;
        s.char_indices().nth($n).map(|(i, _)| &s[..i]).unwrap_or(s)
    }};
}

#[macro_export]
/// Truncate a string to the last `n` characters, or return the string if it is shorter than `n`.
macro_rules! last_n_chars {
    ($s:expr, $n:expr) => {{
        let s: &str = $s;
        let len = s.len();
        s.char_indices()
            .rev()
            .nth($n - 1)
            .map(|(i, _)| &s[i..len])
            .unwrap_or(s)
    }};
}

#[derive(Debug)]
pub(crate) struct MatchTriple(
    String, //preamble
    String, //matched
    String, //postamble
);

impl FromIterator<String> for MatchTriple {
    /// Creates a new `MatchTriple` from an iterator of `String`s.
    ///
    /// The first element of the iterator becomes the preamble, the second element
    /// becomes the matched text, and the third element becomes the postamble.
    ///
    /// If the iterator does not contain enough elements, empty strings are used for
    /// any missing elements.
    ///
    /// # Example
    ///
    ///
    fn from_iter<T: IntoIterator<Item = String>>(iter: T) -> Self {
        let mut iter = iter.into_iter();
        MatchTriple(
            iter.next().unwrap_or_default(),
            iter.next().unwrap_or_default(),
            iter.next().unwrap_or_default(),
        )
    }
}

impl Display for MatchTriple {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}{}", self.0, self.1.red(), self.2)
    }
}

/// Segment the given string `s` into a vector of `MatchTriple`s based on the matches of the
/// regular expression `re`. The first element of each `MatchTriple` is the text preceding the
/// match, the second element is the matched text itself, and the third element is the text
/// following the match. If the regular expression matches the beginning of the string, the first
/// element of the `MatchTriple` will be an empty string. If the regular expression matches the end
/// of the string, the third element of the `MatchTriple` will be an empty string.
pub(crate) fn segment_on_regex(s: &str, re: &Regex, context_len: usize) -> Vec<MatchTriple> {
    let mut segments = Vec::new();
    let mut start = 0;
    let mut end;
    let mut end_of_prev_match = 0usize;
    for m in re.find_iter(s) {
        end = m.start();
        // push postamble if there is any
        if end_of_prev_match > 0 {
            segments.push(first_n_chars!(&s[end_of_prev_match..end], context_len).to_string());
        }
        // push preamble
        segments.push(last_n_chars!(&s[start..end], context_len).to_string()); // push preamble.push(s[start..end].to_string());
        let matched = m.as_str().to_string();
        end_of_prev_match = m.end();
        start = end + matched.len();
        // push match itself
        segments.push(matched);
    }
    if start < s.len() {
        // push postamble of last match
        segments.push(first_n_chars!(&s[start..], context_len).to_string()); // segments.push(s[start..].to_string());
    }
    let mut triples: Vec<MatchTriple> = Vec::new();
    segments.chunks(3).for_each(|chunk| {
        // !ReMOVE this line: let triple: Vec<String> = chunk.iter().map(|s| s.to_owned()).collect();
        let mtriple = MatchTriple::from_iter(chunk.to_owned());
        triples.push(mtriple);
    });
    triples
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segment_on_regex() {
        let s = "Hello, world!";
        let re = Regex::new(r"[Hh]ello").unwrap();
        let mtriples = segment_on_regex(s, &re, 1000);
        println!("{:?}", mtriples);
        assert_eq!(mtriples.len(), 1);
        assert_eq!(mtriples[0].0, "");
        assert_eq!(mtriples[0].1, "Hello");
        assert_eq!(mtriples[0].2, ", world!");
    }

    // Tests to verify the macro works correctly

    #[test]
    fn test_segment_on_regex_multi() {
        let s = "This, that, and the other thing";
        let re = Regex::new(r"[Tt]h").unwrap();
        let mtriples = segment_on_regex(s, &re, 1000);
        println!("{:?}", mtriples);
        assert_eq!(mtriples.len(), 5);
        assert_eq!(mtriples[0].0, "");
        assert_eq!(mtriples[0].1, "Th");
        assert_eq!(mtriples[0].2, "is, ");
        assert_eq!(mtriples[1].0, "is, ");
        assert_eq!(mtriples[1].1, "th");
        assert_eq!(mtriples[1].2, "at, and ");
        assert_eq!(mtriples[2].0, "at, and ");
        assert_eq!(mtriples[2].1, "th");
        assert_eq!(mtriples[2].2, "e o");
        assert_eq!(mtriples[3].0, "e o");
        assert_eq!(mtriples[3].1, "th");
        assert_eq!(mtriples[3].2, "er ");
        assert_eq!(mtriples[4].0, "er ");
        assert_eq!(mtriples[4].1, "th");
        assert_eq!(mtriples[4].2, "ing");
    }

    #[test]
    fn test_first_n_chars() {
        // Basic truncation
        assert_eq!(first_n_chars!("Hello, world!", 5), "Hello");
        assert_eq!(first_n_chars!("Hello", 10), "Hello");

        // Word boundary tests
        assert_eq!(first_n_chars!("Hello beautiful world", 10), "Hello beau");
        assert_eq!(first_n_chars!("Hello-beautiful world", 10), "Hello-beau");
        assert_eq!(first_n_chars!("ThisIsAVeryLongWord", 10), "ThisIsAVer");

        // Unicode tests
        assert_eq!(first_n_chars!("🦀 Rust is awesome", 6), "🦀 Rust");
        assert_eq!(first_n_chars!("🦀 Rust", 2), "🦀 ");

        // Edge cases
        assert_eq!(first_n_chars!("", 5), "");
        assert_eq!(first_n_chars!("   ", 2), "  ");
        assert_eq!(first_n_chars!("NoSpaces", 3), "NoS");
        assert_eq!(first_n_chars!("Célimène", 3), "Cél");
        assert_eq!(first_n_chars!("Célimène", 50), "Célimène");
    }

    #[test]
    fn test_last_n_chars() {
        assert_eq!(last_n_chars!("Hello, world!", 5), "orld!");
        assert_eq!(last_n_chars!("Hello", 10), "Hello");
        assert_eq!(last_n_chars!("Hello beautiful world", 10), "iful world");
        assert_eq!(last_n_chars!("", 10), "");
        assert_eq!(last_n_chars!("   ", 2), "  ");
        assert_eq!(last_n_chars!("NoSpaces", 3), "ces");
        assert_eq!(last_n_chars!("Célimène", 3), "ène");
    }
}
