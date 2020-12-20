//! This little command line program illustrates how to use this
//! library. It extracts all Markdown and RestructuredText
//! hyperlinks found in the input stream `stdin` and
//! prints the list as HTML.
use parse_hyperlinks::renderer::text_rawlinks2html;
use std::io;
use std::io::Read;
use std::io::Write;

/// Minimal application that prints all Markdown and
/// RestructuredText links in `stdin`as HTML to `stdout`.
fn main() -> Result<(), ::std::io::Error> {
    let mut stdin = String::new();
    Read::read_to_string(&mut io::stdin(), &mut stdin)?;

    let outbuf = text_rawlinks2html(&stdin);

    io::stdout().write_all(&outbuf.as_bytes())?;
    Ok(())
}
