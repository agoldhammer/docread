use clap::Parser;
use docx_rs::*;
use glob::glob;
use rayon::prelude::*;
use regex::Regex;
use serde_json::Value;
use std::collections::VecDeque;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;

type Run = String;
type Runs = Vec<Run>;
struct SearchResult {
    file_name: String,
    maybe_result: anyhow::Result<Runs>,
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
/// * `file_name` - A reference to the path of the DOCX file to be parsed.
/// * `search_re` - A reference to the regular expression used to find matching text within the DOCX file.
///
/// # Returns
///
/// * `anyhow::Result<Runs>` - A result containing a vector of text runs that match the regular expression,
///   or an error if the parsing or reading process fails.
fn parse_docx(file_name: &Path, search_re: &Regex) -> anyhow::Result<Runs> {
    let data: Value = serde_json::from_str(&read_docx(&read_to_vec(file_name)?)?.json())?;
    let matched_runs = xtract_text_from_doctree(&data, search_re);
    // for (index, run) in matched_runs.iter().enumerate() {
    //     println!("Match: {}-> {}\n", index + 1, run);
    // }
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
fn read_to_vec(path: &Path) -> anyhow::Result<Vec<u8>> {
    let mut buf = Vec::new();
    std::fs::File::open(path)?.read_to_end(&mut buf)?;
    Ok(buf)
}

/// Process each file in the given `files` vector by attempting to parse it using the given
/// regular expression `search_re`. The results are collected into a vector of `SearchResult`s.
///
/// # Arguments
///
/// * `files` - A vector of file paths to be processed.
/// * `search_re` - A reference to the regular expression used to find matching text within the DOCX files.
///
/// # Returns
///
/// * `Vec<SearchResult>` - A vector of `SearchResult`s containing the file name and the result of
///   parsing the file, if successful, or an error if the parsing or reading process fails.
fn process_files(files: Vec<PathBuf>, search_re: &Regex) -> Vec<SearchResult> {
    // let mut results = Vec::<SearchResult>::new();
    let results = files
        .par_iter()
        .map(|file| {
            // println!("\n*Parsing--> {}\n===", file.display());
            let result = parse_docx(file.as_path(), search_re);
            let search_result = SearchResult {
                file_name: file.display().to_string(),
                maybe_result: result,
            };
            search_result
            // results.push(search_result);
        })
        .collect();
    results
}
/// Search for the given regular expression in all .docx files in the current directory,
/// and all subdirectories.
///
/// # Example
///
///
fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    println!("regex: {:#?}\n\n", args.regex);
    let re = Regex::new(&args.regex).unwrap();
    let mut files = Vec::new();
    let fnames = glob("**/*.docx")?;
    for fname in fnames {
        match fname {
            Ok(path) => {
                files.push(path);
            }
            Err(e) => eprintln!("{:?}", e),
        }
    }
    let nfiles = files.len();
    let search_results = process_files(files, &re);

    for result in search_results {
        println!("=== Searched--> {}\n", result.file_name);
        match result.maybe_result {
            Ok(runs) => {
                for (index, run) in runs.iter().enumerate() {
                    println!("Match: {}-> {}\n", index + 1, run);
                }
            }
            Err(e) => eprintln!("{:?}", e),
        }
    }
    println!("\nBye! Searched {} files", nfiles);

    Ok(())
}
