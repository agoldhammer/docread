use docx_rs::*;
use glob::glob;
use regex::Regex;
use serde_json::Value;
use std::io::Read;
type Run = String;
type Runs = Vec<Run>;
use colored::Colorize;
use rayon::prelude::*;
use std::collections::VecDeque;

use crate::matcher;

struct SearchResult {
    file_name: String,
    maybe_result: anyhow::Result<Runs>,
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

pub trait ReadIntoBuf {
    fn read_into_buf(&self) -> anyhow::Result<Vec<u8>>;
    fn get_fname(&self) -> &str;
}

#[derive(Debug)]
struct RegularFile {
    fname: String,
}

impl From<&str> for RegularFile {
    fn from(s: &str) -> Self {
        RegularFile {
            fname: s.to_string(),
        }
    }
}

impl ReadIntoBuf for RegularFile {
    fn read_into_buf(&self) -> anyhow::Result<Vec<u8>> {
        read_to_vec(&self.fname)
    }

    fn get_fname(&self) -> &str {
        self.fname.as_str()
    }
}

/// Parses a DOCX file specified by `file_name` and extracts text that matches the given regular
/// expression `search_re`.
///
/// # Arguments
///
/// * `file_like` - A reference to the name of a file_like object (docx or zip subarchive) to be parsed.
/// * `search_re` - A reference to the regular expression used to find matching text within the DOCX file.
///
/// # Returns
///
/// * `anyhow::Result<Runs>` - A result containing a vector of text runs that match the regular expression,
///   or an error if the parsing or reading process fails.
fn parse_docx(
    file_like: &Box<dyn ReadIntoBuf + Send + Sync>,
    search_re: &Regex,
) -> anyhow::Result<Runs> {
    let buffer = file_like.read_into_buf()?;
    let data: Value = serde_json::from_str(&read_docx(&buffer)?.json())?;
    let matched_runs = xtract_text_from_doctree(&data, search_re);
    Ok(matched_runs)
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
pub(crate) fn process_files(pattern: &str, search_re: &Regex, quiet: &bool) -> anyhow::Result<()> {
    // obtain paths from specified glob pattern
    let fpaths = glob(pattern)?;
    let zip_pattern = pattern.replace(".docx", ".zip");
    let zip_fpaths = glob(&zip_pattern)?;
    let zip_names: Vec<String> = zip_fpaths
        .flatten()
        .map(|p| format!("{}", p.display()))
        .collect(); // TODO: make this trait object Vec<Box <dyn ReadIntoBuf>>
    println!("Found {:?} zip archives\n", zip_names);
    // and then store all fnames in a vector (needed for count)
    // can use par_bridge here, but this compromise seems better
    let fnames: Vec<String> = fpaths
        .flatten()
        .map(|p| format!("{}", p.display()))
        .collect();
    let nfiles = fnames.len();
    // TODO: make this trait object Vec<Box <dyn ReadIntoBuf>>
    let mut file_surrogates: Vec<Box<dyn ReadIntoBuf + Send + Sync>> = Vec::new();
    for fname in &fnames {
        file_surrogates.push(Box::new(RegularFile {
            fname: fname.clone(),
        }));
    }

    file_surrogates
        .par_iter()
        .map(|file_like| {
            let result = parse_docx(file_like, search_re);
            SearchResult {
                file_name: file_like.get_fname().to_string(),
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
                    let runs_len = format!("Matched {} runs", runs.len())
                        .bright_green()
                        .on_black();
                    println!("{runs_len}\n");
                } else {
                    let not_found = "No matches found".to_string().bright_red().on_black();
                    println!("{not_found}\n");
                }
            } else {
                for (run_index, run) in runs.iter().enumerate() {
                    let mtriples = matcher::segment_on_regex(run, re);
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
