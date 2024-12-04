use clap::Parser;
use docx_rs::*;
use glob::glob;
use regex::Regex;
use serde_json::Value;
// use std::cell::RefCell;
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
    if let Some(children) = data["document"]["children"].as_array() {
        // println!("children: {:#?}\n\n", children);
        children.iter().for_each(|child| {
            let matched_runs = proc_children(child, search_re);
            for run in matched_runs {
                println!("{}", run);
            }
        })
    }
    Ok(())
}

fn proc_children(node: &Value, search_re: &Regex) -> Runs {
    let mut result = Runs::new();
    if let Some(children) = node["data"]["children"].as_array() {
        result = children
            .iter()
            .map(|child| {
                if child["type"] != "text" {
                    proc_children(child, search_re).concat()
                } else {
                    let text = child["data"]["text"].as_str().unwrap();
                    if search_re.is_match(text) {
                        text.to_string()
                    } else {
                        "*nomatch*".to_string()
                    }
                }
            })
            .filter(|s| s.len() > 0)
            .filter(|s| s != "*nomatch*")
            .collect();
    }
    result
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
    let files = glob("**/*.docx")?;
    for file in files {
        match file {
            Ok(path) => {
                println!("\n*Parsing--> {}", path.display());
                parse_docx(path.as_path(), &re)?;
            }
            Err(e) => eprintln!("{:?}", e),
        }
    }
    Ok(())
}
