//! This little command line program illustrates how to use this
//! library. It extracts all Markdown and RestructuredText
//! hyperlinks found in the input stream `stdin` and
//! prints the list as HTML.
use parse_hyperlinks::parser::take_hyperlink;
use std::io;
use std::io::Read;

/// Minimal application that prints all Markdown and
/// RestructuredText links in `stdin`as HTML to `stdout`.
fn main() -> Result<(), ::std::io::Error> {
    let mut buffer = String::new();
    Read::read_to_string(&mut io::stdin(), &mut buffer)?;

    let mut bufp = buffer.as_str();
    while let Ok((b, (ln, lta, lti))) = take_hyperlink(&bufp) {
        bufp = b;
        println!(r#"<a href="{}" title="{}">{}</a><br/>"#, lta, lti, ln);
    }
    Ok(())
}
