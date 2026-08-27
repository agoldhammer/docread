use docx_rs::*;
use regex::Regex;
use serde_json::Value;
use std::io::Read;
type Run = String;
type Runs = Vec<Run>;
use anyhow::Context;
use colored::Colorize;
use rayon::prelude::*;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::matcher;
use crate::selector::make_fnames;
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
    std::fs::File::open(path)
        .with_context(|| format!("Failed to open file: {}", path))?
        .read_to_end(&mut buf)
        .with_context(|| format!("Failed to read file: {}", path))?;
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
        let mut archive = zip::ZipArchive::new(std::fs::File::open(&self.archive_name)?)?;
        let mut file = archive.by_name(&self.entry_name)?;
        let mut buffer = vec![];
        file.read_to_end(&mut buffer)?;
        Ok(buffer)
    }

    fn get_fname(&self) -> String {
        format!("File: {} in {}", self.entry_name, self.archive_name)
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
fn parse_docx(
    file_like: &(dyn ReadIntoBuf + Send + Sync),
    search_re: &Regex,
) -> anyhow::Result<Runs> {
    let buffer = file_like.read_into_buf()?;
    let data: Value = serde_json::from_str(
        &read_docx(&buffer)
            .with_context(|| format!("Error decoding {}", file_like.get_fname()))?
            .json(),
    )?;
    let matched_runs = xtract_text_from_doctree(&data, search_re);
    Ok(matched_runs)
}

/// Processes files matching the given glob pattern, searching for text that matches the
/// specified regular expression, and printing the results.
///
/// # Arguments
///
/// * `base_dir` - A glob base_dir to match files`.
/// * `search_re` - A regular expression used to search for matching text within each file.
/// * `quiet` - A boolean flag to control whether minimal output is shown.
///
/// # Returns
///
/// * `anyhow::Result<()>` - Returns an Ok result if processing is successful; otherwise, returns an error.
pub(crate) fn process_files(
    base_dir: &str,
    search_re: &Regex,
    quiet: bool,
    n_context_chars: usize,
    summary: bool,
    unmatched_show: bool,
) -> anyhow::Result<()> {
    let output_mutex = Arc::new(Mutex::new(()));
    let zip_fnames = make_fnames(base_dir, ".zip")?;
    let docx_fnames = make_fnames(base_dir, ".docx")?;
    let nfiles = docx_fnames.fnames.len();
    let nzips = zip_fnames.fnames.len();
    let mut file_surrogates: Vec<Box<dyn ReadIntoBuf + Send + Sync>> = Vec::new();
    for fname in &docx_fnames.fnames {
        file_surrogates.push(Box::new(RegularFile {
            fname: fname.clone(),
        }));
    }
    for zip_fname in &zip_fnames.fnames {
        match zip_to_zipentries(zip_fname) {
            Ok(zipentries) => {
                for ze in zipentries {
                    file_surrogates.push(Box::new(ze));
                }
            }
            Err(e) => eprintln!("Skipping unreadable zip archive {zip_fname}: {e:?}"),
        }
    }

    file_surrogates
        .par_iter()
        .map(|file_like| {
            let result = parse_docx(&**file_like, search_re);
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
                unmatched_show,
            );
        });
    if summary {
        let fileword = if nfiles == 1 { "file" } else { "files" };
        let zipword = if nzips == 1 {
            "zip archive"
        } else {
            "zip archives"
        };
        println!("Searched {nfiles} {fileword} and {nzips} {zipword}\n");
        println!(
            "  Search parameters: regex: {}, base_path={base_dir}\n\n",
            search_re
        );
        for fname in &docx_fnames.fnames {
            println!("Searched docx file  {fname}");
        }
        for fname in &zip_fnames.fnames {
            println!("Searched zip archive  {fname}");
        }
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
    output_mutex: Arc<Mutex<()>>,
    n_context_chars: usize,
    unmatched_show: bool,
) {
    let _output_guard = output_mutex.lock().unwrap();
    match &result.maybe_result {
        Ok(runs) => {
            if runs.is_empty() && !unmatched_show {
                return;
            }
            println!("Searched file--> {}\n", result.file_name.bright_red());
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
        Err(e) => eprintln!("{}", format!("{e:?}\n").bright_red()),
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
            if let Some(text) = child["data"]["text"].as_str() {
                if search_re.is_match(text) {
                    matching_runs.push(text.to_string());
                }
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

    use std::fs::File;
    use std::io::Write as _;

    use tempfile::tempdir;

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
    fn test_xtract_text_from_doctree_nested_children() {
        // Text buried under non-text container nodes must still be found.
        let data = r#"
        {
            "document": {
                "children": [
                    { "type": "table", "data": { "children": [
                        { "type": "tableRow", "data": { "children": [
                            { "type": "tableCell", "data": { "children": [
                                { "type": "paragraph", "data": { "children": [
                                    { "type": "run", "data": { "children": [
                                        { "type": "text", "data": { "text": "deeply nested match" } }
                                    ] } }
                                ] } }
                            ] } }
                        ] } }
                    ] } }
                ]
            }
        }"#;
        let root: Value = serde_json::from_str(data).unwrap();
        let runs = xtract_text_from_doctree(&root, &Regex::new("nested").unwrap());
        assert_eq!(runs, vec!["deeply nested match"]);
    }

    #[test]
    fn test_xtract_text_from_doctree_missing_document_key() {
        let root: Value = serde_json::from_str(r#"{"other": {}}"#).unwrap();
        assert!(xtract_text_from_doctree(&root, &Regex::new(".*").unwrap()).is_empty());
    }

    #[test]
    fn test_xtract_text_from_doctree_keeps_order_skips_non_matching() {
        let data = r#"
        { "document": { "children": [
            { "type": "text", "data": { "text": "match one" } },
            { "type": "text", "data": { "text": "nope" } },
            { "type": "text", "data": { "text": "match two" } }
        ] } }"#;
        let root: Value = serde_json::from_str(data).unwrap();
        let runs = xtract_text_from_doctree(&root, &Regex::new("match").unwrap());
        assert_eq!(runs, vec!["match one", "match two"]);
    }

    #[test]
    fn test_xtract_text_from_doctree_ignores_non_text_nodes() {
        // A node that is neither a text node nor a container is skipped.
        let data = r#"
        { "document": { "children": [
            { "type": "tab", "data": {} },
            { "type": "text", "data": { "text": "the text" } }
        ] } }"#;
        let root: Value = serde_json::from_str(data).unwrap();
        let runs = xtract_text_from_doctree(&root, &Regex::new("text").unwrap());
        assert_eq!(runs, vec!["the text"]);
    }

    #[test]
    fn test_zip_entry_name() {
        let zip_entry = ZipEntry {
            archive_name: "test.zip".to_string(),
            entry_name: "test.docx".to_string(),
        };
        assert_eq!(zip_entry.get_fname(), "File: test.docx in test.zip");
    }

    #[test]
    fn test_read_to_vec_error() {
        let _: Vec<u8> = Vec::new();
        let res = read_to_vec("nonexistent.docx");
        match res {
            Ok(_) => panic!("Expected an error"),
            Err(e) => assert_eq!(e.to_string(), "Failed to open file: nonexistent.docx"),
        }
    }

    #[test]
    fn test_read_to_vec_success() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let path = dir.path().join("data.bin");
        std::fs::write(&path, b"some bytes")?;

        let buf = read_to_vec(path.to_str().unwrap())?;
        assert_eq!(buf, b"some bytes");
        Ok(())
    }

    #[test]
    fn test_regular_file_from_and_fname() {
        let file = RegularFile::from("a/b.docx");
        assert_eq!(file.get_fname(), "a/b.docx");
        assert!(file.read_into_buf().is_err());
    }

    #[test]
    fn test_parse_docx_extracts_matching_runs() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let path = dir.path().join("doc.docx");
        fixtures::make_docx(&path, &["Hello, world!", "Nothing to see", "hello again"])?;
        let file = RegularFile::from(path.to_str().unwrap());

        let runs = parse_docx(&file, &Regex::new("[Hh]ello").unwrap())?;
        assert_eq!(runs, vec!["Hello, world!", "hello again"]);
        Ok(())
    }

    #[test]
    fn test_parse_docx_no_matches() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let path = dir.path().join("doc.docx");
        fixtures::make_docx(&path, &["nothing to find"])?;
        let file = RegularFile::from(path.to_str().unwrap());

        assert!(parse_docx(&file, &Regex::new("xyz").unwrap())?.is_empty());
        Ok(())
    }

    #[test]
    fn test_parse_docx_pattern_spanning_runs_does_not_match() -> anyhow::Result<()> {
        // A pattern spanning two runs must not match (documented limitation).
        let dir = tempdir()?;
        let path = dir.path().join("split.docx");
        fixtures::make_docx_with_runs(&path, &[&["Hel", "lo"]])?;
        let file = RegularFile::from(path.to_str().unwrap());

        assert!(parse_docx(&file, &Regex::new("Hello").unwrap())?.is_empty());
        let runs = parse_docx(&file, &Regex::new("Hel").unwrap())?;
        assert_eq!(runs, vec!["Hel"]);
        Ok(())
    }

    #[test]
    fn test_parse_docx_unescapes_xml_entities() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let path = dir.path().join("special.docx");
        fixtures::make_docx(&path, &["AT&T <Inc> & friends"])?;
        let file = RegularFile::from(path.to_str().unwrap());

        let runs = parse_docx(&file, &Regex::new("AT&T").unwrap())?;
        assert_eq!(runs, vec!["AT&T <Inc> & friends"]);
        Ok(())
    }

    #[test]
    fn test_parse_docx_corrupted_file_errors() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bad.docx");
        std::fs::write(&path, b"this is not a zip archive").unwrap();
        let file = RegularFile::from(path.to_str().unwrap());

        assert!(parse_docx(&file, &Regex::new("a").unwrap()).is_err());
    }

    #[test]
    fn test_zip_entry_read_into_buf_and_parse() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let docx_path = dir.path().join("inner.docx");
        fixtures::make_docx(&docx_path, &["zipped text"])?;
        let docx_bytes = std::fs::read(&docx_path)?;

        let zip_path = dir.path().join("outer.zip");
        let file = File::create(&zip_path)?;
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip.start_file("inner.docx", options)?;
        zip.write_all(&docx_bytes)?;
        zip.finish()?;

        let entry = ZipEntry {
            archive_name: zip_path.to_str().unwrap().to_string(),
            entry_name: "inner.docx".to_string(),
        };
        let buf = entry.read_into_buf()?;
        assert_eq!(buf, docx_bytes);
        let runs = parse_docx(&entry, &Regex::new("zipped").unwrap())?;
        assert_eq!(runs, vec!["zipped text"]);
        Ok(())
    }

    #[test]
    fn test_zip_entry_missing_entry_errors() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let zip_path = dir.path().join("outer.zip");
        let file = File::create(&zip_path)?;
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip.start_file("a.txt", options)?;
        zip.write_all(b"unrelated")?;
        zip.finish()?;

        let entry = ZipEntry {
            archive_name: zip_path.to_str().unwrap().to_string(),
            entry_name: "nope.docx".to_string(),
        };
        assert!(entry.read_into_buf().is_err());
        Ok(())
    }
}

#[cfg(test)]
mod fixtures {
    //! Builds minimal .docx packages (the zip parts docx-rs requires) so that
    //! tests do not depend on hand-made resource files.

    use std::fs::File;
    use std::io::Write as _;
    use std::path::Path;

    fn xml_escape(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    }

    /// Writes a minimal .docx to `path` whose body holds the given single-run paragraphs.
    pub fn make_docx(path: &Path, paragraphs: &[&str]) -> std::io::Result<()> {
        let paras: Vec<&[&str]> = paragraphs.iter().map(std::slice::from_ref).collect();
        make_docx_with_runs(path, &paras)
    }

    /// Writes a minimal .docx to `path`; each entry of `paragraphs` is a paragraph
    /// consisting of one run per text fragment.
    pub fn make_docx_with_runs(path: &Path, paragraphs: &[&[&str]]) -> std::io::Result<()> {
        let mut body = String::new();
        for runs in paragraphs.iter() {
            for run in *runs {
                body.push_str(&format!(
                    "<w:p><w:r><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
                    xml_escape(run)
                ));
            }
        }
        let document_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>{}</w:body></w:document>"#,
            body
        );
        const CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#;
        const RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;
        const DOC_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"></Relationships>"#;

        let file = File::create(path)?;
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip.start_file("[Content_Types].xml", options)?;
        zip.write_all(CONTENT_TYPES.as_bytes())?;
        zip.start_file("_rels/.rels", options)?;
        zip.write_all(RELS.as_bytes())?;
        zip.start_file("word/document.xml", options)?;
        zip.write_all(document_xml.as_bytes())?;
        zip.start_file("word/_rels/document.xml.rels", options)?;
        zip.write_all(DOC_RELS.as_bytes())?;
        zip.finish()?;
        Ok(())
    }
}
