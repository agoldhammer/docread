use clap::Parser;
use regex::Regex;

mod matcher;
mod reader;
mod selector;
mod ziphandler;
use reader::process_files;

#[derive(Parser, Debug)]
#[command(
    author = "agold",
    version = "0.1.0",
    about,
    long_about = "Search for regular expressions in .docx and zipped .docx files"
)]
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
        help = "top-level dir or file name to search for docx or zip files"
    )]
    dir: String,
    #[arg(
        short,
        long,
        default_value = "75",
        help = "number of context chars to show before/after matches"
    )]
    context: String,
    #[arg(short, long, help = "show file names & match status only")]
    quiet: bool,
}

/// Search for the given regular expression in all .docx and zipped .docx files in the current directory,
/// and all subdirectories.
///
/// Command line arguments:
/// - `--regex, -r`: Regular expression to search for, e.g. 'Hi|[Hh]ello'
/// - `--dir, -d`: case dirctory to begin search (default: current directory)
/// - `--context, -c`: number of context characters to show before/after matches (default: 75)
/// - `--quiet, -q`: show file names & match status only
/// - `--help, -h`: show help message
/// - `--version, -V`: show version information
///
/// # Example
/// docread -r 'Hi|[Hh]ello' -d $HOME/docs -c 100
///   will find all occurrences of 'Hi' or 'Hello' or 'hello' in all .docx and zipped docxfiles in the $HOME/docs directory
///   and all subdirectories, and show 100 characters of context before and after each match.
///
fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let re = Regex::new(&args.regex).unwrap();
    let n_context_chars = args.context.parse::<usize>()?;
    process_files(&args.dir, &re, args.quiet, n_context_chars)?;
    Ok(())
}
