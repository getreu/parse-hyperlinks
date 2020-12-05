//! This little command line program illustrates how to use this
//! library. It extracts all Markdown and RestructuredText
//! hyperlinks found in the input stream `stdin` and
//! prints the list as HTML.
use html_escape::encode_double_quoted_attribute;
use html_escape::encode_text;
use parse_hyperlinks::parser::take_text2dest_label2dest;
use std::io;
use std::io::Read;

/// Minimal application that prints all Markdown and
/// RestructuredText links in `stdin`as HTML to `stdout`.
fn main() -> Result<(), ::std::io::Error> {
    let mut buffer = String::new();
    Read::read_to_string(&mut io::stdin(), &mut buffer)?;

    let mut bufp = buffer.as_str();
    while let Ok((b, (ln, lta, lti))) = take_text2dest_label2dest(&bufp) {
        bufp = b;
        println!(
            r#"<a href="{}" title="{}">{}</a><br/>"#,
            encode_double_quoted_attribute(&lta),
            encode_double_quoted_attribute(&lti),
            encode_text(&ln)
        );
    }
    Ok(())
}
