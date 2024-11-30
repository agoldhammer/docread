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

fn parse_docx(file_name: &Path) -> anyhow::Result<()> {
    let data: Value = serde_json::from_str(&read_docx(&read_to_vec(file_name)?)?.json())?;
    // println!("data: {:#?}\n\n", data);
    if let Some(children) = data["document"]["children"].as_array() {
        // println! {"children: {:#?}\n\n", children};
        children.iter().for_each(read_children);
    }
    Ok(())
}

fn read_children(node: &Value) {
    if let Some(children) = node["data"]["children"].as_array() {
        children.iter().for_each(|child| {
            if child["type"] != "text" {
                // println!(
                //     "---->type: {}; data: {:#?}\n;",
                //     child["type"], child["data"]
                // );
                println!("recursing on type {}...", child["type"]);
                read_children(child);
            } else {
                println!("text: {}\n", child["data"]["text"]);
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
    let files = glob("**/*.docx")?;
    for file in files {
        match file {
            Ok(path) => {
                println!("Parsing--> {}", path.display());
                // parse_docx(&path.display().to_string())?;
                parse_docx(path.as_path())?;
            }
            Err(e) => eprintln!("{:?}", e),
        }
    }
    let args = Args::parse();
    println!("regex: {:#?}\n\n", args.regex);
    // parse_docx(&args.name)?;
    Ok(())
}
