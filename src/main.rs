use clap::Parser;
use colored::Colorize;
use docx_rs::*;
use glob::glob;
use rayon::prelude::*;
use regex::Regex;
use serde_json::Value;
use std::collections::VecDeque;
use std::fmt::{self, Display, Formatter};
use std::io::Read;
use std::sync::Arc;

type Run = String;
type Runs = Vec<Run>;
struct SearchResult {
    file_name: String,
    maybe_result: anyhow::Result<Runs>,
}
#[derive(Debug)]
#[allow(dead_code)]
struct MatchTriple(
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

// modified from https://betterprogramming.pub/how-to-parse-microsoft-word-documents-docx-in-rust-d62a4f56ba94

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    regex: String,
}

/// Parses a DOCX file specified by `file_name` and extracts text that matches the given regular
/// expression `search_re`.
///
/// # Arguments
///
/// * `file_name` - A reference to the name of the DOCX file to be parsed.
/// * `search_re` - A reference to the regular expression used to find matching text within the DOCX file.
///
/// # Returns
///
/// * `anyhow::Result<Runs>` - A result containing a vector of text runs that match the regular expression,
///   or an error if the parsing or reading process fails.
fn parse_docx(file_name: &str, search_re: &Regex) -> anyhow::Result<Runs> {
    let data: Value = serde_json::from_str(&read_docx(&read_to_vec(file_name)?)?.json())?;
    let matched_runs = xtract_text_from_doctree(&data, search_re);
    Ok(matched_runs)
}

/// Recursively traverse the JSON representation of a DOCX file, extracting all text runs that match
/// the given regular expression `search_re`.
///
/// # Arguments
///
/// * `root` - The JSON representation of the DOCX file, as a `serde_json::Value`.
/// * `search_re` - A reference to the regular expression used to find matching text within the DOCX file.
///
/// # Returns
///
/// * `Runs` - A vector of text runs that match the regular expression.
fn xtract_text_from_doctree(root: &Value, search_re: &Regex) -> Runs {
    let mut queue = VecDeque::new();
    let mut matching_runs = Vec::new();
    if let Some(children) = root["document"]["children"].as_array() {
        for child in children {
            queue.push_back(child);
        }
    }
    while let Some(child) = queue.pop_front() {
        if child["type"] == "text" {
            let text = child["data"]["text"].as_str().unwrap();
            if search_re.is_match(text) {
                matching_runs.push(text.to_string());
            }
        } else if let Some(children) = child["data"]["children"].as_array() {
            for child in children {
                queue.push_back(child);
            }
        }
    }
    matching_runs
}

/// Reads the contents of a file at the given `path` into a vector of bytes.
///
/// # Errors
///
/// Will return an error if the file cannot be opened or read to the end.
fn read_to_vec(path: &str) -> anyhow::Result<Vec<u8>> {
    let mut buf = Vec::new();
    std::fs::File::open(path)?.read_to_end(&mut buf)?;
    Ok(buf)
}

/// Process each file in the given `files` vector by attempting to parse it using the given
/// regular expression `search_re`. The results are collected into a vector of `SearchResult`s.
///
/// # Arguments
///
/// * `files` - A vector of file names to be processed.
/// * `search_re` - A reference to the regular expression used to find matching text within the DOCX files.
///
/// # Returns
///
/// * `Vec<SearchResult>` - A vector of `SearchResult`s containing the file name and the result of
///   parsing the file, if successful, or an error if the parsing or reading process fails.
fn process_files(fnames: Vec<Arc<String>>, search_re: &Regex) -> Vec<SearchResult> {
    let results = fnames
        .par_iter()
        .map(|file| {
            let result = parse_docx(file, search_re);
            let search_result = SearchResult {
                file_name: file.to_string(),
                maybe_result: result,
            };
            search_result
        })
        .collect();
    results
}

/// Segment the given string `s` into a vector of `MatchTriple`s based on the matches of the
/// regular expression `re`. The first element of each `MatchTriple` is the text preceding the
/// match, the second element is the matched text itself, and the third element is the text
/// following the match. If the regular expression matches the beginning of the string, the first
/// element of the `MatchTriple` will be an empty string. If the regular expression matches the end
/// of the string, the third element of the `MatchTriple` will be an empty string.
fn segment_on_regex(s: &str, re: &Regex) -> Vec<MatchTriple> {
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

/// Search for the given regular expression in all .docx files in the current directory,
/// and all subdirectories.
///
/// # Example: docread --regex "Hi|[Hh]ello"
///
///
fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    println!("regex: {:#?}\n\n", args.regex);
    let re = Regex::new(&args.regex).unwrap();
    let fpaths = glob("**/*.docx")?;
    let fnames: Vec<Arc<String>> = fpaths
        .into_iter()
        .filter(|f| f.is_ok())
        .map(|f| f.unwrap())
        .map(|p| format!("{}", p.display()))
        .map(|s| Arc::new(s))
        .collect();
    let search_results = process_files(fnames, &re);

    for (seq_no, result) in search_results.into_iter().enumerate() {
        println!(
            "File {}. === Searched--> {}\n",
            seq_no + 1,
            result.file_name.bright_red()
        );
        match result.maybe_result {
            Ok(runs) => {
                for (run_index, run) in runs.iter().enumerate() {
                    let mtriples = segment_on_regex(run, &re);
                    for (match_index, mtriple) in mtriples.iter().enumerate() {
                        let prompt = format!("{}-{}", run_index + 1, match_index + 1);
                        println!("  {}-> {}\n", prompt.bright_yellow().on_blue(), mtriple);
                    }
                }
            }
            Err(e) => eprintln!("{:?}", e),
        }
    }
    Ok(())
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
}
