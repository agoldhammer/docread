use colored::Colorize;
use regex::Regex;
use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
#[allow(dead_code)]
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
pub(crate) fn segment_on_regex(s: &str, re: &Regex) -> Vec<MatchTriple> {
    let mut segments = Vec::new();
    let mut start = 0;
    let mut end;
    let mut end_of_prev_match = 0usize;
    for m in re.find_iter(s) {
        end = m.start();
        if end_of_prev_match > 0 {
            segments.push(s[end_of_prev_match..end].to_string());
        }
        segments.push(s[start..end].to_string());
        let matched = m.as_str().to_string();
        end_of_prev_match = m.end();
        start = end + matched.len();
        segments.push(matched);
    }
    if start < s.len() {
        segments.push(s[start..].to_string());
    }
    let mut triples: Vec<MatchTriple> = Vec::new();
    segments.chunks(3).for_each(|chunk| {
        let triple: Vec<String> = chunk.iter().map(|s| s.to_owned()).collect();
        let mtriple = MatchTriple::from_iter(triple);
        triples.push(mtriple);
    });
    triples
}

#[macro_export]
macro_rules! truncate_str {
    ($s:expr, $n:expr) => {{
        let s: &str = $s;
        if s.chars().count() <= $n {
            s.to_string()
        } else {
            let chars: Vec<char> = s.chars().collect();
            let mut last_space = None;
            let mut char_count = 0;

            // Find the last space within the limit
            for (i, &c) in chars.iter().enumerate() {
                if char_count >= $n {
                    break;
                }
                if c.is_whitespace() {
                    last_space = Some(i);
                }
                char_count += 1;
            }

            // If we found a space, truncate there; otherwise take full characters
            match last_space {
                Some(pos) => chars[..pos].iter().collect::<String>(),
                None => chars.iter().take($n).collect::<String>(),
            }
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segment_on_regex() {
        let s = "Hello, world!";
        let re = Regex::new(r"[Hh]ello").unwrap();
        let mtriples = segment_on_regex(s, &re);
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
        let mtriples = segment_on_regex(s, &re);
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
    fn test_truncate() {
        let s = "Hello, world!";
        // let mt = "";
        let x = "🦀 Rust is awesome";
        assert_eq!(&s[..5], "Hello");
        assert_eq!(&s[6..], " world!");
        assert_eq!(&s[13..], "");
        assert_eq!(&x[..6], "🦀 Rust");
        // assert_eq!(&s[..50], "Hello, world!");
        // assert_eq!(&mt[..10], "");
        // assert_eq!(&mt[..10], "");
    }

    #[test]
    fn test_truncate_str() {
        // Basic truncation
        assert_eq!(truncate_str!("Hello, world!", 5), "Hello");
        assert_eq!(truncate_str!("Hello", 10), "Hello");

        // Word boundary tests
        assert_eq!(truncate_str!("Hello beautiful world", 10), "Hello");
        assert_eq!(truncate_str!("Hello-beautiful world", 10), "Hello-beau");
        assert_eq!(truncate_str!("ThisIsAVeryLongWord", 10), "ThisIsAVer");

        // Unicode tests
        // assert_eq!(truncate_str!("🦀 Rust is awesome", 6), "🦀 Rust");
        // assert_eq!(truncate_str!("🦀 Rust", 2), "🦀");

        // Edge cases
        assert_eq!(truncate_str!("", 5), "");
        assert_eq!(truncate_str!("   ", 2), "");
        assert_eq!(truncate_str!("NoSpaces", 3), "NoS");
    }
}
