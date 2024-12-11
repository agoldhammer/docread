use clap::Parser;
use colored::Colorize;
use docx_rs::*;
use glob::glob;
// use rayon::iter::ParallelBridge;
use rayon::prelude::*;
use regex::Regex;
use serde_json::Value;
use std::collections::VecDeque;
use std::fmt::{self, Display, Formatter};
use std::io::Read;
// use std::sync::Arc;

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
    #[arg(
        short,
        long,
        help = "Regular expression to search for, e.g. 'Hi|[Hh]ello'"
    )]
    regex: String,
    #[arg(
        short,
        long,
        default_value = "**/*.docx",
        help = "Must enclose in parens"
    )]
    glob: String,
    #[arg(short, long, help = "show file names & match status only")]
    quiet: bool,
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

/// Processes files matching the given glob pattern, searching for text that matches the
/// specified regular expression, and printing the results.
///
/// # Arguments
///
/// * `pattern` - A glob pattern to match files. Should end with `.docx`.
/// * `search_re` - A regular expression used to search for matching text within each file.
/// * `quiet` - A boolean flag to control whether minimal output is shown.
///
/// # Returns
///
/// * `anyhow::Result<()>` - Returns an Ok result if processing is successful; otherwise, returns an error.
fn process_files(pattern: &str, search_re: &Regex, quiet: &bool) -> anyhow::Result<()> {
    // obtain paths from specified glob pattern
    let fpaths = glob(pattern)?;
    // and then store all fnames in a vector (needed for count)
    // can use par_bridge here, but this compromise seems better
    let fnames: Vec<String> = fpaths
        .flatten()
        .map(|p| format!("{}", p.display()))
        .collect();
    let nfiles = fnames.len(); // save to print at end of procedure
    fnames
        .par_iter()
        .map(|file| {
            let result = parse_docx(file.as_str(), search_re);
            SearchResult {
                file_name: file.to_string(),
                maybe_result: result,
            }
        })
        .for_each(|search_result| print_result(&search_result, search_re, quiet));
    println!("Searched {nfiles} files\n");
    println!(
        "  Search parameters: regex: {}, glob={:#?}\n\n",
        search_re, pattern
    );
    Ok(())
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

/// Prints the search results for a DOCX file, highlighting matches of a regular expression.
///
/// # Arguments
///
/// * `result` - A reference to a `SearchResult` struct containing the file name and potential matches.
/// * `re` - A reference to the regular expression used for identifying matches in the text runs.
/// * `quiet` - A boolean indicating whether to suppress detailed output. If true, only the count of
///   matched runs is printed. Otherwise, details of each match within each run are printed.
///
/// # Behavior
///
/// If a `SearchResult` contains matches (`Ok` variant), the function prints the number of matched runs
/// when `quiet` is true. Otherwise, it iterates through each match and prints details in a formatted
/// manner, using `segment_on_regex` to divide the text into segments. If there's an error (`Err` variant),
/// the error is printed to standard error.
fn print_result(result: &SearchResult, re: &Regex, quiet: &bool) {
    println!("Searched file--> {}\n", result.file_name.bright_red());
    match &result.maybe_result {
        Ok(runs) => {
            if *quiet {
                if !runs.is_empty() {
                    let found = "Matched {runs.len()} runs\n".to_string().bright_green();
                    println!("{found}\n");
                } else {
                    let not_found = "No matches found".to_string().bright_red();
                    println!("{not_found}\n");
                }
            } else {
                for (run_index, run) in runs.iter().enumerate() {
                    let mtriples = segment_on_regex(run, re);
                    for (match_index, mtriple) in mtriples.iter().enumerate() {
                        let prompt = format!("{}-{}", run_index + 1, match_index + 1);
                        println!("  {}-> {}\n", prompt.bright_yellow().on_blue(), mtriple);
                    }
                }
            }
            println!("===\n");
        }
        Err(e) => eprintln!("{:?}\n", e),
    }
}

/// Search for the given regular expression in all .docx files in the current directory,
/// and all subdirectories.
///
/// # Example: docread --regex "Hi|[Hh]ello"
///
///
fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let re = Regex::new(&args.regex).unwrap();
    let valid_glob = &args.glob.ends_with(".docx");
    if *valid_glob {
        process_files(&args.glob, &re, &args.quiet)?;
    } else {
        eprintln!("Glob pattern {} does not end with .docx", args.glob);
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
