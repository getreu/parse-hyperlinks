//! This little command line program illustrates how to use this
//! library. It extracts all Markdown and RestructuredText
//! hyperlinks found in the input stream `stdin` and
//! prints the list as HTML.
use clap::Parser;
use parse_hyperlinks::renderer::links2html_writer;
use parse_hyperlinks::renderer::text_links2html_writer;
use parse_hyperlinks::renderer::text_rawlinks2html_writer;
use std::fs::File;
use std::io;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process;
use std::sync::LazyLock;

#[derive(Debug, Eq, PartialEq, Parser)]
#[command(
    version,
    name = "atext2html",
    about,
    long_about = "Render source text with markup hyperlinks.",
    disable_version_flag = true
)]
/// This structure holds the command-line-options.
pub struct Args {
    #[arg(long, short = 'r')]
    /// render hyperlinks
    pub render_links: bool,

    #[arg(long, short = 'l')]
    /// print only links (one per line)
    pub only_links: bool,

    #[structopt(name = "FILE")]
    /// paths to files to render (or `-` for stdin)
    pub inputs: Vec<PathBuf>,

    #[arg(long, short = 'o')]
    /// print not to stdout but in file
    pub output: Option<PathBuf>,

    /// print version and exit
    #[arg(long, short = 'V')]
    pub version: bool,
}

/// Structure to hold the parsed command-line arguments.
pub static ARGS: LazyLock<Args> = LazyLock::new(Args::parse);

/// Uses the version-number defined in `../Cargo.toml`.
const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
/// (c) Jens Getreu
const AUTHOR: &str = "(c) Jens Getreu, 2020-2025";

/// Minimal application that prints all Markdown and
/// RestructuredText links in `stdin`as HTML to `stdout`.
fn main() -> Result<(), ::std::io::Error> {
    if ARGS.version {
        println!("Version {}, {}", VERSION.unwrap_or("unknown"), AUTHOR);
        process::exit(0);
    };

    let renderer = match (ARGS.render_links, ARGS.only_links) {
        (false, false) => |(inbuf, mut output): (&str, &mut dyn Write)| -> Result<_, _> {
            text_rawlinks2html_writer(inbuf, &mut output)
        },
        (true, false) => |(inbuf, mut output): (&str, &mut dyn Write)| -> Result<_, _> {
            text_links2html_writer(inbuf, &mut output)
        },
        (_, true) => |(inbuf, mut output): (&str, &mut dyn Write)| -> Result<_, _> {
            links2html_writer(inbuf, &mut output)
        },
    };

    // Where to print the output.
    let mut output = if let Some(outname) = &ARGS.output {
        let file = File::create(Path::new(&outname))?;
        Box::new(file) as Box<dyn Write>
    } else {
        Box::new(io::stdout()) as Box<dyn Write>
    };

    if (ARGS.inputs.is_empty()) || ((ARGS.inputs.len() == 1) && ARGS.inputs[0] == Path::new("-")) {
        let mut inbuf = String::new();
        Read::read_to_string(&mut io::stdin(), &mut inbuf)?;

        renderer((&inbuf, &mut output))?;
    } else {
        for filename in ARGS.inputs.iter() {
            let mut inbuf = String::new();
            let mut file = File::open(filename)?;
            Read::read_to_string(&mut file, &mut inbuf)?;

            renderer((&inbuf, &mut output))?;
        }
    };

    Ok(())
}
