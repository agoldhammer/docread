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
        let n: usize = $n;
        match n.checked_sub(1) {
            Some(n_minus_1) => s
                .char_indices()
                .rev()
                .nth(n_minus_1)
                .map(|(i, _)| &s[i..])
                .unwrap_or(s),
            None => "",
        }
    }};
}

#[derive(Debug)]
pub(crate) struct MatchTriple {
    pub preamble: String,
    pub matched: String,
    pub postamble: String,
}

impl Display for MatchTriple {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}{}",
            self.preamble,
            self.matched.red(),
            self.postamble
        )
    }
}

/// Segment the given string `s` into a vector of `MatchTriple`s based on the matches of the
/// regular expression `re`. The first element of each `MatchTriple` is the text preceding the
/// match, the second element is the matched text itself, and the third element is the text
/// following the match. If the regular expression matches the beginning of the string, the first
/// element of the `MatchTriple` will be an empty string. If the regular expression matches the end
/// of the string, the third element of the `MatchTriple` will be an empty string.
pub(crate) fn segment_on_regex(s: &str, re: &Regex, context_len: usize) -> Vec<MatchTriple> {
    let matches: Vec<_> = re.find_iter(s).collect();
    let mut triples = Vec::with_capacity(matches.len());
    let mut gap_start = 0usize;
    for (i, m) in matches.iter().enumerate() {
        let gap_end = matches.get(i + 1).map_or(s.len(), |next| next.start());
        let preamble = last_n_chars!(&s[gap_start..m.start()], context_len).to_string();
        let postamble = first_n_chars!(&s[m.end()..gap_end], context_len).to_string();
        triples.push(MatchTriple::new(
            preamble,
            m.as_str().to_string(),
            postamble,
        ));
        gap_start = m.end();
    }
    triples
}

impl MatchTriple {
    fn new(preamble: String, matched: String, postamble: String) -> Self {
        MatchTriple {
            preamble,
            matched,
            postamble,
        }
    }
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
        assert_eq!(mtriples[0].preamble, "");
        assert_eq!(mtriples[0].matched, "Hello");
        assert_eq!(mtriples[0].postamble, ", world!");
    }

    // Tests to verify the macro works correctly

    #[test]
    fn test_segment_on_regex_multi() {
        let s = "This, that, and the other thing";
        let re = Regex::new(r"[Tt]h").unwrap();
        let mtriples = segment_on_regex(s, &re, 1000);
        println!("{:?}", mtriples);
        assert_eq!(mtriples.len(), 5);
        assert_eq!(mtriples[0].preamble, "");
        assert_eq!(mtriples[0].matched, "Th");
        assert_eq!(mtriples[0].postamble, "is, ");
        assert_eq!(mtriples[1].preamble, "is, ");
        assert_eq!(mtriples[1].matched, "th");
        assert_eq!(mtriples[1].postamble, "at, and ");
        assert_eq!(mtriples[2].preamble, "at, and ");
        assert_eq!(mtriples[2].matched, "th");
        assert_eq!(mtriples[2].postamble, "e o");
        assert_eq!(mtriples[3].preamble, "e o");
        assert_eq!(mtriples[3].matched, "th");
        assert_eq!(mtriples[3].postamble, "er ");
        assert_eq!(mtriples[4].preamble, "er ");
        assert_eq!(mtriples[4].matched, "th");
        assert_eq!(mtriples[4].postamble, "ing");
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

    #[test]
    fn test_last_n_chars_zero() {
        assert_eq!(last_n_chars!("Hello", 0), "");
        assert_eq!(first_n_chars!("Hello", 0), "");
    }

    #[test]
    fn test_segment_on_regex_zero_length_first_match() {
        // A zero-length first match desynchronised the old flat chunking.
        let s = "ab";
        let re = Regex::new("").unwrap();
        let mtriples = segment_on_regex(s, &re, 75);
        assert_eq!(mtriples.len(), 3);
        assert_eq!(mtriples[0].preamble, "");
        assert_eq!(mtriples[0].matched, "");
        assert_eq!(mtriples[0].postamble, "a");
        assert_eq!(mtriples[1].matched, "");
        assert_eq!(mtriples[2].matched, "");
    }

    #[test]
    fn test_segment_on_regex_truncated_context() {
        let s = "abcdefghij";
        let re = Regex::new("[ace]").unwrap();
        let mtriples = segment_on_regex(s, &re, 2);
        assert_eq!(mtriples.len(), 3);
        assert_eq!(mtriples[0].preamble, "");
        assert_eq!(mtriples[0].matched, "a");
        assert_eq!(mtriples[0].postamble, "b");
        assert_eq!(mtriples[1].preamble, "b");
        assert_eq!(mtriples[1].matched, "c");
        assert_eq!(mtriples[1].postamble, "d");
        assert_eq!(mtriples[2].preamble, "d");
        assert_eq!(mtriples[2].matched, "e");
        assert_eq!(mtriples[2].postamble, "fg");
    }
}
