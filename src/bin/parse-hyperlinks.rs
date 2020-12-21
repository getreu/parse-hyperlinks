//! This little command line program illustrates how to use this
//! library. It extracts all Markdown and RestructuredText
//! hyperlinks found in the input stream `stdin` and
//! prints the list as HTML.
use parse_hyperlinks::renderer::text_rawlinks2html_writer;
use std::io;
use std::io::Read;

/// Minimal application that prints all Markdown and
/// RestructuredText links in `stdin`as HTML to `stdout`.
fn main() -> Result<(), ::std::io::Error> {
    let mut stdin = String::new();
    Read::read_to_string(&mut io::stdin(), &mut stdin)?;

    text_rawlinks2html_writer(&stdin, &mut io::stdout())?;

    Ok(())
}
