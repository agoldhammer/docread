//! End-to-end tests for the docread CLI.
//!
//! Each test builds a fresh temporary directory tree containing generated
//! .docx files (and sometimes zip archives of them), runs the binary, and
//! asserts on the ANSI-stripped output. The per-file result blocks are
//! printed under a mutex, so individual lines are stable even though the
//! order of blocks across files is not.

mod common;

use common::*;

/// The trimmed match line (e.g. `1-1-> ...`) in `stdout`, if any.
fn match_lines(stdout: &str) -> Vec<String> {
    // Match prompt lines only; "Searched file--> ..." headers contain "-> " too.
    let re = regex::Regex::new(r"^\s*\d+-\d+-> ").unwrap();
    stdout
        .lines()
        .filter(|l| re.is_match(l))
        .map(|l| l.trim().to_string())
        .collect()
}

#[test]
fn help_flag_prints_usage() {
    let (out, stdout, _stderr) = run(&["-h"]);
    assert!(out.status.success());
    assert!(stdout.contains("Usage"));
    assert!(stdout.contains("--regex"));
    assert!(stdout.contains("--dir"));
    assert!(stdout.contains("--context"));
}

#[test]
fn version_flag_prints_version() {
    let (out, stdout, _stderr) = run(&["-V"]);
    assert!(out.status.success());
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn missing_regex_argument_fails() {
    let (out, _stdout, stderr) = run(&[]);
    assert!(!out.status.success());
    assert!(stderr.contains("--regex"));
}

#[test]
fn invalid_regex_fails() {
    let dir = tempfile::tempdir().unwrap();
    let (out, _stdout, stderr) = run(&["-r", "[", "-d", dir.path().to_str().unwrap()]);
    assert!(!out.status.success());
    assert!(stderr.contains("Invalid regular expression"));
}

#[test]
fn finds_match_in_docx_with_default_context() {
    let dir = tempfile::tempdir().unwrap();
    make_docx(
        &dir.path().join("fox.docx"),
        &["The quick brown fox jumps over the lazy dog"],
    )
    .unwrap();

    let (out, stdout, _stderr) = run(&["-r", "fox", "-d", dir.path().to_str().unwrap()]);
    assert!(out.status.success());
    assert!(stdout.contains("Searched file-->"));
    assert!(stdout.contains("fox.docx"));
    // 44 chars < the default 75-char context on each side, so the whole
    // run text appears around the match.
    let lines = match_lines(&stdout);
    assert_eq!(
        lines,
        vec!["1-1-> The quick brown fox jumps over the lazy dog"]
    );
    assert!(stdout.contains("==="));
}

#[test]
fn context_length_limits_preamble_and_postamble() {
    let dir = tempfile::tempdir().unwrap();
    make_docx(
        &dir.path().join("ctx.docx"),
        &["AAAAAAAAAA needle BBBBBBBBBB"],
    )
    .unwrap();

    let (out, stdout, _stderr) = run(&[
        "-r",
        "needle",
        "-c",
        "5",
        "-d",
        dir.path().to_str().unwrap(),
    ]);
    assert!(out.status.success());
    // The 5-char windows are "AAAA " before and " BBBB" after, so the
    // spaces adjacent to the match are consumed by the context.
    assert_eq!(match_lines(&stdout), vec!["1-1-> AAAA needle BBBB"]);
}

#[test]
fn zero_context_shows_only_the_match() {
    let dir = tempfile::tempdir().unwrap();
    make_docx(
        &dir.path().join("ctx.docx"),
        &["AAAAAAAAAA needle BBBBBBBBBB"],
    )
    .unwrap();

    let (out, stdout, _stderr) = run(&[
        "-r",
        "needle",
        "-c",
        "0",
        "-d",
        dir.path().to_str().unwrap(),
    ]);
    assert!(out.status.success());
    assert_eq!(match_lines(&stdout), vec!["1-1-> needle"]);
}

#[test]
fn no_match_is_silent_without_unmatched_show() {
    let dir = tempfile::tempdir().unwrap();
    make_docx(&dir.path().join("quiet.docx"), &["nothing to find here"]).unwrap();

    let (out, stdout, _stderr) = run(&["-r", "zebra", "-d", dir.path().to_str().unwrap()]);
    assert!(out.status.success());
    assert!(stdout.is_empty());
}

#[test]
fn unmatched_show_lists_files_without_matches() {
    let dir = tempfile::tempdir().unwrap();
    make_docx(&dir.path().join("quiet.docx"), &["nothing to find here"]).unwrap();

    let (out, stdout, _stderr) = run(&["-r", "zebra", "-u", "-d", dir.path().to_str().unwrap()]);
    assert!(out.status.success());
    assert!(stdout.contains("Searched file-->"));
    assert!(stdout.contains("quiet.docx"));
    assert!(stdout.contains("==="));
    assert!(!stdout.contains("Matched"));
    assert!(!stdout.contains("No matches found"));
}

#[test]
fn quiet_unmatched_report_no_matches_found() {
    let dir = tempfile::tempdir().unwrap();
    make_docx(&dir.path().join("quiet.docx"), &["nothing to find here"]).unwrap();

    let (out, stdout, _stderr) = run(&[
        "-r",
        "zebra",
        "-q",
        "-u",
        "-d",
        dir.path().to_str().unwrap(),
    ]);
    assert!(out.status.success());
    assert!(stdout.contains("No matches found"));
}

#[test]
fn quiet_mode_counts_matched_runs() {
    let dir = tempfile::tempdir().unwrap();
    make_docx(
        &dir.path().join("hello.docx"),
        &["Hello there", "hello world"],
    )
    .unwrap();

    let (out, stdout, _stderr) = run(&["-r", "[Hh]ello", "-q", "-d", dir.path().to_str().unwrap()]);
    assert!(out.status.success());
    assert!(stdout.contains("Matched 2 runs"));
    assert!(match_lines(&stdout).is_empty());
}

#[test]
fn multiple_matches_in_one_run_are_numbered() {
    let dir = tempfile::tempdir().unwrap();
    make_docx(&dir.path().join("nums.docx"), &["one two three"]).unwrap();

    let (out, stdout, _stderr) = run(&["-r", "o", "-d", dir.path().to_str().unwrap()]);
    assert!(out.status.success());
    let lines = match_lines(&stdout);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "1-1-> one tw");
    assert_eq!(lines[1], "1-2-> ne two three");
}

#[test]
fn matches_in_different_runs_are_numbered_per_run() {
    let dir = tempfile::tempdir().unwrap();
    make_docx(
        &dir.path().join("two.docx"),
        &["Hello there", "hello world"],
    )
    .unwrap();

    let (out, stdout, _stderr) = run(&["-r", "[Hh]ello", "-d", dir.path().to_str().unwrap()]);
    assert!(out.status.success());
    let lines = match_lines(&stdout);
    assert_eq!(lines.len(), 2);
    assert!(lines.iter().any(|l| l.starts_with("1-1->")));
    assert!(lines.iter().any(|l| l.starts_with("2-1->")));
}

#[test]
fn summary_reports_counts_and_lists_files() {
    let dir = tempfile::tempdir().unwrap();
    make_docx(&dir.path().join("a.docx"), &["fox one"]).unwrap();
    make_docx(&dir.path().join("b.docx"), &["fox two"]).unwrap();
    let inner = dir.path().join("inner.docx");
    make_docx(&inner, &["fox three"]).unwrap();
    let zip_path = dir.path().join("archive.zip");
    make_zip(&zip_path, &[("inner.docx", std::fs::read(&inner).unwrap())]).unwrap();
    std::fs::remove_file(&inner).unwrap();

    let (out, stdout, _stderr) = run(&["-r", "fox", "-s", "-d", dir.path().to_str().unwrap()]);
    assert!(out.status.success());
    assert!(stdout.contains("Searched 2 files and 1 zip archive"));
    assert!(stdout.contains("regex: fox"));
    assert!(stdout.contains("base_path="));
    for f in ["a.docx", "b.docx"] {
        assert!(
            stdout
                .lines()
                .any(|l| l.contains("Searched docx file") && l.contains(f)),
            "summary should list {f}"
        );
    }
    assert!(stdout
        .lines()
        .any(|l| { l.contains("Searched zip archive") && l.contains("archive.zip") }));
}

#[test]
fn summary_is_singular_for_single_file() {
    let dir = tempfile::tempdir().unwrap();
    make_docx(&dir.path().join("a.docx"), &["fox"]).unwrap();

    let (out, stdout, _stderr) = run(&["-r", "fox", "-s", "-d", dir.path().to_str().unwrap()]);
    assert!(out.status.success());
    assert!(stdout.contains("Searched 1 file and 0 zip archives"));
}

#[test]
fn empty_directory_reports_zero_files() {
    let dir = tempfile::tempdir().unwrap();

    let (out, stdout, _stderr) = run(&["-r", "fox", "-s", "-d", dir.path().to_str().unwrap()]);
    assert!(out.status.success());
    assert!(stdout.contains("Searched 0 files and 0 zip archives"));
}

#[test]
fn nonexistent_directory_is_not_an_error() {
    let (out, stdout, _stderr) = run(&["-r", "fox", "-s", "-d", "/definitely/not/a/real/dir"]);
    assert!(out.status.success());
    assert!(stdout.contains("Searched 0 files and 0 zip archives"));
}

#[test]
fn finds_docx_in_nested_subdirectories() {
    let dir = tempfile::tempdir().unwrap();
    let deep = dir.path().join("sub").join("deep");
    std::fs::create_dir_all(&deep).unwrap();
    make_docx(&deep.join("inner.docx"), &["needle in a haystack"]).unwrap();

    let (out, stdout, _stderr) = run(&["-r", "haystack", "-d", dir.path().to_str().unwrap()]);
    assert!(out.status.success());
    assert!(stdout.contains("inner.docx"));
    assert_eq!(match_lines(&stdout), vec!["1-1-> needle in a haystack"]);
}

#[test]
fn finds_match_in_docx_inside_zip() {
    let dir = tempfile::tempdir().unwrap();
    let inner = dir.path().join("scratch-inner.docx");
    make_docx(&inner, &["zipped needle here"]).unwrap();
    let zip_path = dir.path().join("archive.zip");
    make_zip(
        &zip_path,
        &[
            ("inner.docx", std::fs::read(&inner).unwrap()),
            ("readme.txt", b"not a docx".to_vec()),
        ],
    )
    .unwrap();
    std::fs::remove_file(&inner).unwrap();

    let zip_str = zip_path.to_str().unwrap();
    let (out, stdout, _stderr) = run(&["-r", "zipped", "-d", dir.path().to_str().unwrap()]);
    assert!(out.status.success());
    assert!(stdout.contains(&format!("File: inner.docx in {zip_str}")));
    assert_eq!(match_lines(&stdout), vec!["1-1-> zipped needle here"]);
}

#[test]
fn zip_without_docx_entries_yields_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let zip_path = dir.path().join("archive.zip");
    make_zip(&zip_path, &[("readme.txt", b"no docx here".to_vec())]).unwrap();

    let (out, stdout, _stderr) = run(&["-r", "needle", "-d", dir.path().to_str().unwrap()]);
    assert!(out.status.success());
    assert!(stdout.is_empty());

    let (out, stdout, _stderr) = run(&["-r", "needle", "-s", "-d", dir.path().to_str().unwrap()]);
    assert!(out.status.success());
    assert!(stdout.contains("Searched 0 files and 1 zip archive"));
}

#[test]
fn corrupt_docx_reports_error_on_stderr_and_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("bad.docx"), b"this is not a zip archive").unwrap();

    let (out, stdout, stderr) = run(&["-r", "a", "-d", dir.path().to_str().unwrap()]);
    assert!(out.status.success());
    assert!(stdout.is_empty());
    assert!(stderr.contains("Error decoding"));
    assert!(stderr.contains("bad.docx"));
}

#[test]
fn corrupt_zip_is_skipped_with_warning() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("bad.zip"), b"garbage, not a zip").unwrap();

    let (out, stdout, stderr) = run(&["-r", "a", "-d", dir.path().to_str().unwrap()]);
    assert!(out.status.success());
    assert!(stdout.is_empty());
    assert!(stderr.contains("Skipping unreadable zip archive"));
    assert!(stderr.contains("bad.zip"));
}

#[test]
fn pattern_spanning_two_runs_does_not_match() {
    // Documented limitation: Word splits text into runs readily, so a
    // pattern spanning a run boundary never matches.
    let dir = tempfile::tempdir().unwrap();
    make_docx_with_runs(&dir.path().join("split.docx"), &[&["Hel", "lo"]]).unwrap();

    let (out, stdout, _stderr) = run(&["-r", "Hello", "-d", dir.path().to_str().unwrap()]);
    assert!(out.status.success());
    assert!(stdout.is_empty());

    let (out, stdout, _stderr) = run(&["-r", "Hel", "-d", dir.path().to_str().unwrap()]);
    assert!(out.status.success());
    assert_eq!(match_lines(&stdout), vec!["1-1-> Hel"]);
}

#[test]
fn matches_unicode_text() {
    let dir = tempfile::tempdir().unwrap();
    make_docx(
        &dir.path().join("fr.docx"),
        &["Bonjour Célimène, au revoir"],
    )
    .unwrap();

    let (out, stdout, _stderr) = run(&["-r", "Célimène", "-d", dir.path().to_str().unwrap()]);
    assert!(out.status.success());
    assert_eq!(
        match_lines(&stdout),
        vec!["1-1-> Bonjour Célimène, au revoir"]
    );
}

#[test]
fn matches_xml_special_characters_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    make_docx(&dir.path().join("special.docx"), &["AT&T <Inc> & friends"]).unwrap();

    let (out, stdout, _stderr) = run(&["-r", "AT&T", "-d", dir.path().to_str().unwrap()]);
    assert!(out.status.success());
    assert_eq!(match_lines(&stdout), vec!["1-1-> AT&T <Inc> & friends"]);
}

#[test]
fn resource_fixture_archive_is_searchable() {
    // The checked-in TestArchive.zip must still be readable end to end.
    let (out, stdout, _stderr) = run(&[
        "-r",
        ".",
        "-s",
        "-d",
        concat!(env!("CARGO_MANIFEST_DIR"), "/resources"),
    ]);
    assert!(out.status.success());
    assert!(stdout.contains("Searched 2 files and 1 zip archive"));
    assert!(stdout.contains("Searched zip archive"));
}
