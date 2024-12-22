use clap::Parser;
use regex::Regex;

mod matcher;
mod reader;
mod selector;
mod ziphandler;
use reader::process_files;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(
        short,
        long,
        help = "Regular expression to search for, e.g. 'Hi|[Hh]ello'"
    )]
    regex: String,
    #[arg(
        short,
        long,
        default_value = ".",
        help = "top-leveldir or file name to search for docx or zip files"
    )]
    dir: String,
    #[arg(short, long, help = "show file names & match status only")]
    quiet: bool,
}

/// Search for the given regular expression in all .docx files in the current directory,
/// and all subdirectories.
///
/// # Example: docread --regex "Hi|[Hh]ello"
///
///
fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let re = Regex::new(&args.regex).unwrap();
    process_files(&args.dir, &re, &args.quiet)?;
    Ok(())
}
