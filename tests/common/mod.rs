//! Shared helpers for the CLI integration tests.

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub fn bin_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_docread"))
}

/// Runs the docread binary with the given arguments, returning the raw output.
pub fn run_docread(args: &[&str]) -> Output {
    Command::new(bin_path())
        .args(args)
        .output()
        .expect("failed to run docread binary")
}

/// Runs the docread binary and returns (output, ANSI-stripped stdout, ANSI-stripped stderr).
///
/// ANSI sequences are stripped so the tests do not depend on whether the
/// binary decided to colorize (which depends on terminal detection).
pub fn run(args: &[&str]) -> (Output, String, String) {
    let out = run_docread(args);
    let stdout = strip_ansi(&String::from_utf8_lossy(&out.stdout));
    let stderr = strip_ansi(&String::from_utf8_lossy(&out.stderr));
    (out, stdout, stderr)
}

pub fn strip_ansi(s: &str) -> String {
    regex::Regex::new("\\x1b\\[[0-9;]*m")
        .unwrap()
        .replace_all(s, "")
        .into_owned()
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Writes a minimal .docx to `path` (the zip parts docx-rs requires) whose body
/// holds the given single-run paragraphs.
pub fn make_docx(path: &Path, paragraphs: &[&str]) -> std::io::Result<()> {
    let paras: Vec<&[&str]> = paragraphs.iter().map(std::slice::from_ref).collect();
    make_docx_with_runs(path, &paras)
}

/// Like `make_docx`, but each entry of `paragraphs` is a paragraph consisting
/// of one run per text fragment.
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
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
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

/// Writes a zip archive at `zip_path` containing `members` (entry name -> bytes).
pub fn make_zip(zip_path: &Path, members: &[(&str, Vec<u8>)]) -> std::io::Result<()> {
    let file = File::create(zip_path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, data) in members {
        zip.start_file(name, options)?;
        zip.write_all(data)?;
    }
    zip.finish()?;
    Ok(())
}
