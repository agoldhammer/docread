use clap::Parser;
use docx_rs::*;
use glob::glob;
use serde_json::Value;
use std::io::Read;
use std::path::Path;

// taken from https://betterprogramming.pub/how-to-parse-microsoft-word-documents-docx-in-rust-d62a4f56ba94

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    regex: String,
}

fn parse_docx(file_name: &Path, search_str: &str) -> anyhow::Result<()> {
    let data: Value = serde_json::from_str(&read_docx(&read_to_vec(file_name)?)?.json())?;
    // println!("data: {:#?}\n\n", data);
    if let Some(children) = data["document"]["children"].as_array() {
        // println! {"children: {:#?}\n\n", children};
        children
            .iter()
            .for_each(|child| read_children(child, search_str));
    }
    Ok(())
}

fn read_children(node: &Value, search_str: &str) {
    if let Some(children) = node["data"]["children"].as_array() {
        children.iter().for_each(|child| {
            if child["type"] != "text" {
                // println!(
                //     "---->type: {}; data: {:#?}\n;",
                //     child["type"], child["data"]
                // );
                // println!("recursing on type {}...", child["type"]);
                read_children(child, search_str);
            } else {
                let text = child["data"]["text"].as_str().unwrap();
                if text.contains(search_str) {
                    println!("found match: {}", text);
                }
                // println!("text: {}\n", child["data"]["text"]);
            }
        });
    }
}

fn read_to_vec(path: &Path) -> anyhow::Result<Vec<u8>> {
    let mut buf = Vec::new();
    std::fs::File::open(path)?.read_to_end(&mut buf)?;
    Ok(buf)
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    println!("regex: {:#?}\n\n", args.regex);
    let files = glob("**/*.docx")?;
    for file in files {
        match file {
            Ok(path) => {
                println!("\n*Parsing--> {}", path.display());
                parse_docx(path.as_path(), &args.regex)?;
            }
            Err(e) => eprintln!("{:?}", e),
        }
    }
    // parse_docx(&args.name)?;
    Ok(())
}
