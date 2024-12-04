use clap::Parser;
use docx_rs::*;
use glob::glob;
use regex::Regex;
use serde_json::Value;
// use std::cell::RefCell;
use std::collections::VecDeque;
use std::io::Read;
use std::path::Path;

type Run = String;
type Runs = Vec<Run>;

// modified from https://betterprogramming.pub/how-to-parse-microsoft-word-documents-docx-in-rust-d62a4f56ba94

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    regex: String,
}

fn parse_docx(file_name: &Path, search_re: &Regex) -> anyhow::Result<()> {
    let data: Value = serde_json::from_str(&read_docx(&read_to_vec(file_name)?)?.json())?;
    let matched_runs = xtract_text_from_doctree(&data, search_re);
    for (index, run) in matched_runs.iter().enumerate() {
        println!("Match: {}-> {}\n", index + 1, run);
    }
    Ok(())
}

fn xtract_text_from_doctree(root: &Value, search_re: &Regex) -> Runs {
    let mut queue = VecDeque::new();
    let mut matching_runs = Vec::new();
    if let Some(children) = root["document"]["children"].as_array() {
        // println!("init children: {:#?}\n\n", children);
        for child in children {
            queue.push_back(child);
        }
    }
    while let Some(child) = queue.pop_front() {
        if child["type"] == "text" {
            let text = child["data"]["text"].as_str().unwrap();
            // println!("xtract {}", text);
            if search_re.is_match(text) {
                matching_runs.push(text.to_string());
            }
        } else {
            // println!("pushing back child type: {:#?}\n", child["type"]);
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
    // extract matching text from each file
    for file in files {
        println!("\n*Parsing--> {}\n===", file.display());
        match parse_docx(file.as_path(), &re) {
            Ok(()) => {}
            Err(e) => eprintln!("{:?}", e),
        };
    }
    Ok(())
}
