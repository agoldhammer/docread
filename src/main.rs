use clap::Parser;
use docx_rs::*;
use glob::glob;
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

fn parse_docx(file_name: &Path, search_re: &Regex) -> anyhow::Result<Runs> {
    let data: Value = serde_json::from_str(&read_docx(&read_to_vec(file_name)?)?.json())?;
    let matched_runs = xtract_text_from_doctree(&data, search_re);
    // for (index, run) in matched_runs.iter().enumerate() {
    //     println!("Match: {}-> {}\n", index + 1, run);
    // }
    Ok(matched_runs)
}

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
        } else {
            if let Some(children) = child["data"]["children"].as_array() {
                for child in children {
                    queue.push_back(child);
                }
            }
        }
    }
    matching_runs
}

fn read_to_vec(path: &Path) -> anyhow::Result<Vec<u8>> {
    let mut buf = Vec::new();
    std::fs::File::open(path)?.read_to_end(&mut buf)?;
    Ok(buf)
}

fn process_files(files: Vec<PathBuf>, search_re: &Regex) -> Vec<SearchResult> {
    let mut results = Vec::<SearchResult>::new();
    files.iter().for_each(|file| {
        // println!("\n*Parsing--> {}\n===", file.display());
        let result = parse_docx(file.as_path(), &search_re);
        let search_result = SearchResult {
            file_name: file.display().to_string(),
            maybe_result: result,
        };
        results.push(search_result);
    });
    // match result {
    //     Ok(runs) => results.push(runs),
    //     Err(e) => eprintln!("{:?}", e),
    // };
    // results.iter().for_each(|runs| {
    //     println!("\n\n===");
    //     runs.iter().for_each(|run| println!("{:?}", run));
    // })
    results
}
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
    println!("Searching {} files", files.len());
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

    Ok(())
}
