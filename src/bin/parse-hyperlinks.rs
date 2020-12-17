//! This little command line program illustrates how to use this
//! library. It extracts all Markdown and RestructuredText
//! hyperlinks found in the input stream `stdin` and
//! prints the list as HTML.
use html_escape::encode_double_quoted_attribute;
use html_escape::encode_text;
use parse_hyperlinks::iterator::Hyperlink;
use std::io;
use std::io::Read;

/// Minimal application that prints all Markdown and
/// RestructuredText links in `stdin`as HTML to `stdout`.
fn main() -> Result<(), ::std::io::Error> {
    let mut buffer = String::new();
    Read::read_to_string(&mut io::stdin(), &mut buffer)?;

    let bufp = buffer.as_str();
    for (text, dest, title) in Hyperlink::new(&bufp) {
        println!(
            r#"<a href="{}" title="{}">{}</a><br/>"#,
            encode_double_quoted_attribute(&dest),
            encode_double_quoted_attribute(&title),
            encode_text(&text)
        );
    }
    Ok(())
}
