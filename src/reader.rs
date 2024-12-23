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
use std::sync::{Arc, Mutex};

use crate::matcher;
use crate::selector::make_path;
use crate::ziphandler::{zip_to_zipentries, ZipEntry};

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
    fn get_fname(&self) -> String;
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

    fn get_fname(&self) -> String {
        self.fname.clone()
    }
}

impl ReadIntoBuf for ZipEntry {
    fn read_into_buf(&self) -> anyhow::Result<Vec<u8>> {
        // read_to_vec(&self.entry_name)
        // TODO: Implement zip archive handling
        let mut archive = zip::ZipArchive::new(std::fs::File::open(&self.archive_name)?)?;
        let mut file = archive.by_name(&self.entry_name)?;
        let mut buffer = vec![];
        file.read_to_end(&mut buffer)?;
        Ok(buffer)
    }

    fn get_fname(&self) -> String {
        format! {"File: {} in {}", self.entry_name, self.archive_name}.clone()
        // self.entry_name.clone()
    }
}

/// Parses a DOCX file or archive entry specified by `file_like` (which must implement `ReadIntoBuf`)
/// and extracts text that matches the given regular expression `search_re`.
///
/// # Arguments
///
/// * `file_like` - A reference to the name of a `file_like` object (docx or zip subarchive) to be parsed.
/// * `search_re` - A reference to the regular expression used to find matching text within the DOCX file.
///
/// # Returns
///
/// * `anyhow::Result<Runs>` - A result containing a vector of text runs that match the regular expression,
///   or an error if the parsing or reading process fails.
#[allow(clippy::borrowed_box)]
fn parse_docx(
    file_like: &Box<dyn ReadIntoBuf + Send + Sync>,
    search_re: &Regex,
) -> anyhow::Result<Runs> {
    let buffer = file_like.read_into_buf()?;
    let data: Value = serde_json::from_str(&read_docx(&buffer)?.json())?;
    let matched_runs = xtract_text_from_doctree(&data, search_re);
    Ok(matched_runs)
}

#[derive(Debug)]
struct Fnames {
    fnames: Vec<String>,
}

impl TryFrom<&str> for Fnames {
    type Error = anyhow::Error;
    /// Attempts to create a `Fnames` from a glob pattern. The `glob` crate is used to find all
    /// matching files, and the resulting paths are converted to `String`s and stored in the
    /// `fnames` member of the `Fnames` struct.
    ///
    fn try_from(pattern: &str) -> anyhow::Result<Self> {
        let fpaths = glob(pattern)?;
        let fnames: Vec<String> = fpaths
            .flatten()
            .map(|p| format!("{}", p.display()))
            .collect();
        Ok(Fnames { fnames })
    }
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
pub(crate) fn process_files(
    pattern: &str,
    search_re: &Regex,
    quiet: bool,
    n_context_chars: usize,
) -> anyhow::Result<()> {
    // output mutex
    let output_mutex = Arc::new(Mutex::new(0));
    let base_path = make_path(pattern);
    // done: Implement zip archive handling
    let zip_path = base_path.replace(".docx", ".zip");
    let zip_fnames = Fnames::try_from(zip_path.as_str())?;
    println!("Found {:?} zip archives\n", zip_fnames);

    // ! can use par_bridge here, but this compromise seems better
    let docx_fnames = Fnames::try_from(base_path.as_str())?;
    let nfiles = docx_fnames.fnames.len();
    let nzips = zip_fnames.fnames.len();
    let mut file_surrogates: Vec<Box<dyn ReadIntoBuf + Send + Sync>> = Vec::new();
    for fname in &docx_fnames.fnames {
        file_surrogates.push(Box::new(RegularFile {
            fname: fname.clone(),
        }));
    }
    for zip_fname in &zip_fnames.fnames {
        let zipentries = zip_to_zipentries(zip_fname)?;
        for ze in zipentries {
            file_surrogates.push(Box::new(ze));
        }
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
        .for_each(|search_result| {
            print_result(
                &search_result,
                search_re,
                quiet,
                output_mutex.clone(),
                n_context_chars,
            );
        });
    let fileword = if nfiles == 1 { "file" } else { "files" };
    let zipword = if nzips == 1 {
        "zip archive"
    } else {
        "zip archives"
    };
    println!("Searched {nfiles} {fileword} amd {nzips} {zipword}\n");
    println!(
        "  Search parameters: regex: {}, base_path={:#?}\n\n",
        search_re, base_path
    );
    for fname in &docx_fnames.fnames {
        println!("Searched docx file  {}", fname);
    }
    for fname in &zip_fnames.fnames {
        println!("Searched zip archive  {}", fname);
    }
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
fn print_result(
    result: &SearchResult,
    re: &Regex,
    quiet: bool,
    output_mutex: Arc<Mutex<u32>>,
    n_context_chars: usize,
) {
    let _output_guard = output_mutex.lock().unwrap();
    println!("Searched file--> {}\n", result.file_name.bright_red());
    match &result.maybe_result {
        Ok(runs) => {
            if quiet {
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
                    let mtriples = matcher::segment_on_regex(run, re, n_context_chars);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xtract_text_from_doctree() {
        let data = r#"
        {
            "document": {
                "children": [
                    {
                        "type": "text",
                        "data": {
                            "text": "Hello, world!"
                        }
                    }
                ]
            }
        }
        "#;
        let root: Value = serde_json::from_str(data).unwrap();
        let search_re = Regex::new(r"[Hh]ello").unwrap();
        let runs = xtract_text_from_doctree(&root, &search_re);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0], "Hello, world!");
    }

    #[test]
    fn test_zip_entry_name() {
        let zip_entry = ZipEntry {
            archive_name: "test.zip".to_string(),
            entry_name: "test.docx".to_string(),
        };
        assert_eq!(zip_entry.get_fname(), "File: test.docx in test.zip");
    }
}
