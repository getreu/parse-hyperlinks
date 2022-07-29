//! A set of functions providing markup source code to HTML renderer, that make
//! hyperlinks clickable.

use crate::iterator::Hyperlink;
use html_escape::encode_double_quoted_attribute;
use html_escape::encode_safe;
use html_escape::encode_text;
use std::borrow::Cow;
use std::io;
use std::io::Write;

fn render<'a, O, P, W>(
    input: &'a str,
    begin_doc: &str,
    end_doc: &str,
    verb_renderer: O,
    link_renderer: P,
    render_label: bool,
    output: &mut W,
) -> Result<(), io::Error>
where
    O: Fn(Cow<'a, str>) -> Cow<'a, str>,
    P: Fn((Cow<'a, str>, (String, String, String))) -> String,
    W: Write,
{
    // As this will be overwritten inside the loop, the first value only counts
    // when there are no hyperlinks in the input. In this case we print the
    // input as a whole.
    let mut rest = Cow::Borrowed(input);

    output.write_all(begin_doc.as_bytes())?;
    for ((skipped2, consumed2, remaining2), (text2, dest2, title2)) in
        Hyperlink::new(input, render_label)
    {
        let skipped = encode_text(skipped2);
        let consumed = encode_text(consumed2);
        let remaining = encode_text(remaining2);
        let text = encode_safe(&text2).to_string();
        let dest = encode_double_quoted_attribute(&dest2).to_string();
        let title = encode_double_quoted_attribute(&title2).to_string();
        output.write_all(verb_renderer(skipped).as_bytes())?;
        let rendered_link = link_renderer((consumed, (text, dest, title)));
        output.write_all(rendered_link.as_bytes())?;
        rest = remaining;
    }
    output.write_all(verb_renderer(rest).as_bytes())?;
    output.write_all(end_doc.as_bytes())?;
    Ok(())
}

/// # Source code viewer with link renderer
///
/// Text to HTML renderer that prints the input text “as it is”, but
/// renders links with markup. Links are clickable and only their
/// _link text_ is shown (the part enclosed with `<a>` and `</a>`).
///
/// ## Markdown
/// ```
/// use parse_hyperlinks::renderer::text_links2html;
/// use std::borrow::Cow;
///
/// let i = r#"abc[text0](dest0 "title0")abc
/// abc[text1][label1]abc
/// abc[text2](dest2 "title2")abc
/// [text3]: dest3 "title3"
/// [label1]: dest1 "title1"
/// abc[text3]abc
/// "#;
///
/// let expected = "\
/// <pre>abc<a href=\"dest0\" title=\"title0\">text0</a>abc
/// abc<a href=\"dest1\" title=\"title1\">text1</a>abc
/// abc<a href=\"dest2\" title=\"title2\">text2</a>abc
/// <a href=\"dest3\" title=\"title3\">[text3]: dest3 &quot;title3&quot;</a>
/// <a href=\"dest1\" title=\"title1\">[label1]: dest1 &quot;title1&quot;</a>
/// abc<a href=\"dest3\" title=\"title3\">text3</a>abc
/// </pre>";
/// let res = text_links2html(i);
/// assert_eq!(res, expected);
/// ```
///
/// ### Rendered text
///
/// This is how the rendered text looks like in the browser:
///
/// <pre>abc<a href="dest0" title="title0">text0</a>abc
/// abc<a href="dest1" title="title1">text1</a>abc
/// abc<a href="dest2" title="title2">text2</a>abc
/// <a href="dest3" title="title3">[text3]: dest3 &quot;title3&quot;</a>
/// <a href="dest1" title="title1">[label1]: dest1 &quot;title1&quot;</a>
/// abc<a href="dest3" title="title3">text3</a>abc
/// </pre>
///
/// ## reStructuredText
/// ```
/// use parse_hyperlinks::renderer::text_links2html;
/// use std::borrow::Cow;
///
/// let i = r#"abc `text1 <label1_>`_abc
/// abc text2_ abc
/// abc text3__ abc
/// abc text_label4_ abc
/// abc text5__ abc
/// .. _label1: dest1
/// .. _text2: dest2
/// .. __: dest3
/// __ dest5
/// "#;
///
/// let expected = "\
/// <pre>abc <a href=\"dest1\" title=\"\">text1</a>abc
/// abc <a href=\"dest2\" title=\"\">text2</a> abc
/// abc <a href=\"dest3\" title=\"\">text3</a> abc
/// abc text_label4_ abc
/// abc <a href=\"dest5\" title=\"\">text5</a> abc
/// <a href=\"dest1\" title=\"\">.. _label1: dest1</a>
/// <a href=\"dest2\" title=\"\">.. _text2: dest2</a>
/// <a href=\"dest3\" title=\"\">.. __: dest3</a>
/// <a href=\"dest5\" title=\"\">__ dest5</a>
/// </pre>\
/// ";
///
/// let res = text_links2html(i);
/// assert_eq!(res, expected);
/// ```
///
/// ### Rendered text
///
/// This is how the rendered text looks like in the browser:
///
/// <pre>abc <a href="dest1" title="">text1</a>abc
/// abc <a href="dest2" title="">text2</a> abc
/// abc <a href="dest3" title="">text3</a> abc
/// abc text_label4_ abc
/// abc <a href="dest5" title="">text5</a> abc
/// <a href="dest1" title="">.. _label1: dest1</a>
/// <a href="dest2" title="">.. _text2: dest2</a>
/// <a href="dest3" title="">.. __: dest3</a>
/// <a href="dest5" title="">__ dest5</a>
/// </pre>
///
/// ## Asciidoc
///
/// ```
/// use parse_hyperlinks::renderer::text_links2html;
/// use std::borrow::Cow;
///
/// let i = r#"abc https://dest0[text0]abc
/// abc link:https://dest1[text1]abc
/// abc{label2}[text2]abc
/// abc{label3}abc
/// :label2: https://dest2
/// :label3: https://dest3
/// "#;
///
/// let expected = "\
/// <pre>abc <a href=\"https://dest0\" title=\"\">text0</a>abc
/// abc <a href=\"https://dest1\" title=\"\">text1</a>abc
/// abc<a href=\"https://dest2\" title=\"\">text2</a>abc
/// abc<a href=\"https://dest3\" title=\"\">https:&#x2F;&#x2F;dest3</a>abc
/// <a href=\"https://dest2\" title=\"\">:label2: https:&#x2F;&#x2F;dest2</a>
/// <a href=\"https://dest3\" title=\"\">:label3: https:&#x2F;&#x2F;dest3</a>
/// </pre>";
///
/// let res = text_links2html(i);
/// assert_eq!(res, expected);
/// ```
///
/// ### Rendered text
///
/// This is how the rendered text looks like in the browser:
///
/// <pre>abc <a href="https://dest0" title="">text0</a>abc
/// abc <a href="https://dest1" title="">text1</a>abc
/// abc<a href="https://dest2" title="">text2</a>abc
/// abc<a href="https://dest3" title="">https:&#x2F;&#x2F;dest3</a>abc
/// <a href="https://dest2" title="">:label2: https:&#x2F;&#x2F;dest2</a>
/// <a href="https://dest3" title="">:label3: https:&#x2F;&#x2F;dest3</a>
/// </pre>
///
///
/// ## Wikitext
///
/// ```
/// use parse_hyperlinks::renderer::text_links2html;
/// use std::borrow::Cow;
///
/// let i = r#"abc[https://dest0 text0]abc
/// "#;
///
/// let expected = "\
/// <pre>abc<a href=\"https://dest0\" title=\"\">text0</a>abc
/// </pre>";
///
/// let res = text_links2html(i);
/// assert_eq!(res, expected);
/// ```
///
/// ### Rendered text
///
/// This is how the rendered text looks like in the browser:
///
/// <pre>abc<a href="https://dest0" title="">text0</a>abc
/// </pre>
///
///
/// ## HTML
///
/// HTML _inline links_ are sanitized and passed through.
///
/// ```
/// use parse_hyperlinks::renderer::text_links2html;
/// use std::borrow::Cow;
///
/// let i = r#"abc<a href="dest1" title="title1">text1</a>abc"#;
///
/// let expected = "<pre>\
/// abc<a href=\"dest1\" title=\"title1\">text1</a>abc\
/// </pre>";
///
/// let res = text_links2html(i);
/// assert_eq!(res, expected);
/// ```
///
/// ### Rendered text
///
/// This is how the rendered text looks like in the browser:
///
/// <pre>
/// abc<a href="dest1" title="title1">text1</a>abc
/// </pre>
///
#[inline]
pub fn text_links2html(input: &str) -> String {
    let mut output = Vec::new();
    text_links2html_writer(input, &mut output).unwrap_or_default();
    // We know this is safe, because only `str` have been written into `output`.
    // So the following would be fine, but I want to keep this crate `unsafe`-free.
    //    unsafe {String::from_utf8_unchecked(output)}
    String::from_utf8(output).unwrap_or_default()
}

/// # Source code viewer with link renderer
///
/// Same as `text_links2html()`, but it uses `Write` for output. This function
/// allocates much less memory and is faster because it avoids copying.
///
/// Usage example:
/// ```no_run
/// use parse_hyperlinks::renderer::text_links2html_writer;
/// use std::io;
/// use std::io::Read;
/// fn main() -> Result<(), ::std::io::Error> {
///     let mut stdin = String::new();
///     Read::read_to_string(&mut io::stdin(), &mut stdin)?;
///
///     text_links2html_writer(&stdin, &mut io::stdout())?;
///
///     Ok(())
/// }
/// ```
pub fn text_links2html_writer<'a, S: 'a + AsRef<str>, W: Write>(
    input: S,
    output: &mut W,
) -> Result<(), io::Error> {
    let input = input.as_ref();

    let verb_renderer = |verb| verb;

    let link_renderer = |(_, (text, dest, title)): (_, (String, String, String))| {
        let mut s = String::new();
        s.push_str(r#"<a href=""#);
        s.push_str(&*dest);
        s.push_str(r#"" title=""#);
        s.push_str(&*title);
        s.push_str(r#"">"#);
        s.push_str(&*text);
        s.push_str(r#"</a>"#);
        s
    };

    render(
        input,
        "<pre>",
        "</pre>",
        verb_renderer,
        link_renderer,
        true,
        output,
    )
}

/// # Markup source code viewer
///
/// Markup source code viewer, that make hyperlinks
/// clickable in your web-browser.
///
/// This function prints the input text “as it is”, but
/// renders links with markup. Links are clickable.
///
/// ## Markdown
/// ```
/// use parse_hyperlinks::renderer::text_rawlinks2html;
/// use std::borrow::Cow;
///
/// let i = r#"abc[text0](dest0 "title0")abc
/// abc[text1][label1]abc
/// abc[text2](dest2 "title2")abc
/// [text3]: dest3 "title3"
/// [label1]: dest1 "title1"
/// abc[text3]abc
/// "#;
///
/// let expected = "\
/// <pre>abc<a href=\"dest0\" title=\"title0\">[text0](dest0 \"title0\")</a>abc
/// abc<a href=\"dest1\" title=\"title1\">[text1][label1]</a>abc
/// abc<a href=\"dest2\" title=\"title2\">[text2](dest2 \"title2\")</a>abc
/// <a href=\"dest3\" title=\"title3\">[text3]: dest3 \"title3\"</a>
/// <a href=\"dest1\" title=\"title1\">[label1]: dest1 \"title1\"</a>
/// abc<a href=\"dest3\" title=\"title3\">[text3]</a>abc
/// </pre>";
///
/// let res = text_rawlinks2html(i);
/// assert_eq!(res, expected);
/// ```
///
/// ### Rendered text
///
/// This is how the rendered text looks like in the browser:
///
/// <pre>abc<a href="dest0" title="title0">[text0](dest0 "title0")</a>abc
/// abc<a href="dest1" title="title1">[text1][label1]</a>abc
/// abc<a href="dest2" title="title2">[text2](dest2 "title2")</a>abc
/// <a href="dest3" title="title3">[text3]: dest3 "title3"</a>
/// <a href="dest1" title="title1">[label1]: dest1 "title1"</a>
/// abc<a href="dest3" title="title3">[text3]</a>abc
/// </pre>
///
/// ## reStructuredText
/// ```
/// use parse_hyperlinks::renderer::text_rawlinks2html;
/// use std::borrow::Cow;
///
/// let i = r#"
/// abc `text1 <label1_>`_abc
/// abc text2_ abc
/// abc text3__ abc
/// abc text_label4_ abc
/// abc text5__ abc
/// .. _label1: dest1
/// .. _text2: dest2
/// .. __: dest3
/// __ dest5
/// "#;
///
/// let expected = "\
/// <pre>
/// abc <a href=\"dest1\" title=\"\">`text1 &lt;label1_&gt;`_</a>abc
/// abc <a href=\"dest2\" title=\"\">text2_</a> abc
/// abc <a href=\"dest3\" title=\"\">text3__</a> abc
/// abc text_label4_ abc
/// abc <a href=\"dest5\" title=\"\">text5__</a> abc
/// <a href=\"dest1\" title=\"\">.. _label1: dest1</a>
/// <a href=\"dest2\" title=\"\">.. _text2: dest2</a>
/// <a href=\"dest3\" title=\"\">.. __: dest3</a>
/// <a href=\"dest5\" title=\"\">__ dest5</a>
/// </pre>";
///
/// let res = text_rawlinks2html(i);
/// assert_eq!(res, expected);
/// ```
///
/// ### Rendered text
///
/// This is how the rendered text look likes in the browser:
///
/// <pre>
/// abc <a href="dest1" title="">`text1 &lt;label1_&gt;`_</a>abc
/// abc <a href="dest2" title="">text2_</a> abc
/// abc <a href="dest3" title="">text3__</a> abc
/// abc text_label4_ abc
/// abc <a href="dest5" title="">text5__</a> abc
/// <a href="dest1" title="">.. _label1: dest1</a>
/// <a href="dest2" title="">.. _text2: dest2</a>
/// <a href="dest3" title="">.. __: dest3</a>
/// <a href="dest5" title="">__ dest5</a>
/// </pre>
///
/// ## Asciidoc
///
/// ```
/// use parse_hyperlinks::renderer::text_rawlinks2html;
/// use std::borrow::Cow;
///
/// let i = r#"abc https://dest0[text0]abc
/// abc link:https://dest1[text1]abc
/// abc{label2}[text2]abc
/// abc{label3}abc
/// :label2: https://dest2
/// :label3: https://dest3
/// "#;
///
/// let expected = "\
/// <pre>abc <a href=\"https://dest0\" title=\"\">https://dest0[text0]</a>abc
/// abc <a href=\"https://dest1\" title=\"\">link:https://dest1[text1]</a>abc
/// abc<a href=\"https://dest2\" title=\"\">{label2}[text2]</a>abc
/// abc<a href=\"https://dest3\" title=\"\">{label3}</a>abc
/// <a href=\"https://dest2\" title=\"\">:label2: https://dest2</a>
/// <a href=\"https://dest3\" title=\"\">:label3: https://dest3</a>
/// </pre>";
///
/// let res = text_rawlinks2html(i);
/// assert_eq!(res, expected);
/// ```
///
/// ### Rendered text
///
/// This is how the rendered text looks like in the browser:
///
/// <pre>abc <a href="https://dest0" title="">https://dest0[text0]</a>abc
/// abc <a href="https://dest1" title="">link:https://dest1[text1]</a>abc
/// abc<a href="https://dest2" title="">{label2}[text2]</a>abc
/// abc<a href="https://dest3" title="">{label3}</a>abc
/// <a href="https://dest2" title="">:label2: https://dest2</a>
/// <a href="https://dest3" title="">:label3: https://dest3</a>
/// </pre>
///
///
/// ## Wikitext
///
/// ```
/// use parse_hyperlinks::renderer::text_rawlinks2html;
/// use std::borrow::Cow;
///
/// let i = r#"abc[https://dest0 text0]abc
/// "#;
///
/// let expected = "\
/// <pre>abc<a href=\"https://dest0\" title=\"\">[https://dest0 text0]</a>abc
/// </pre>";
///
/// let res = text_rawlinks2html(i);
/// assert_eq!(res, expected);
/// ```
///
/// ### Rendered text
///
/// This is how the rendered text looks like in the browser:
///
/// <pre>abc<a href="https://dest0" title="">[https://dest0 text0]</a>abc
/// </pre>
///
/// ## HTML
///
/// HTML _inline links_ are sanitized and their link
/// source code is shown as _link text_.
///
/// ```
/// use parse_hyperlinks::renderer::text_rawlinks2html;
/// use std::borrow::Cow;
///
/// let i = r#"abc<a href="dest1" title="title1">text1</a>abc"#;
///
/// let expected = "\
/// <pre>abc<a href=\"dest1\" title=\"title1\">\
/// &lt;a href=\"dest1\" title=\"title1\"&gt;text1&lt;/a&gt;\
/// </a>abc</pre>";
///
/// let res = text_rawlinks2html(i);
/// assert_eq!(res, expected);
/// ```
///
/// ### Rendered text
///
/// This is how the rendered text looks like in the browser:
///
/// <pre>
/// abc<a href="dest1" title="title1">&lt;a href="dest1" title="title1"&gt;text1&lt;/a&gt;</a>abc
/// </pre>
///
#[inline]
pub fn text_rawlinks2html(input: &str) -> String {
    let mut output = Vec::new();
    text_rawlinks2html_writer(input, &mut output).unwrap_or_default();
    // We know this is safe, because only `str` have been written into `output`.
    // So the following would be fine, but I want to keep this crate `unsafe`-free.
    //    unsafe {String::from_utf8_unchecked(output)}
    String::from_utf8(output).unwrap_or_default()
}

/// # Markup source code viewer
///
/// Same as `text_rawlinks2html()`, but it uses `Write` for output. This function
/// allocates much less memory and is faster because it avoids copying.
///
/// Usage example:
/// ```no_run
/// use parse_hyperlinks::renderer::text_rawlinks2html_writer;
/// use std::io;
/// use std::io::Read;
/// fn main() -> Result<(), ::std::io::Error> {
///     let mut stdin = String::new();
///     Read::read_to_string(&mut io::stdin(), &mut stdin)?;
///
///     text_rawlinks2html_writer(&stdin, &mut io::stdout())?;
///
///     Ok(())
/// }
/// ```
pub fn text_rawlinks2html_writer<'a, S: 'a + AsRef<str>, W: Write>(
    input: S,
    output: &mut W,
) -> Result<(), io::Error> {
    let input = input.as_ref();

    let verb_renderer = |verb| verb;

    let link_renderer = |(consumed, (_, dest, title)): (Cow<str>, (_, String, String))| {
        let mut s = String::new();
        s.push_str(r#"<a href=""#);
        s.push_str(&*dest);
        s.push_str(r#"" title=""#);
        s.push_str(&*title);
        s.push_str(r#"">"#);
        s.push_str(&*consumed);
        s.push_str(r#"</a>"#);
        s
    };

    render(
        input,
        "<pre>",
        "</pre>",
        verb_renderer,
        link_renderer,
        true,
        output,
    )
}

/// # Hyperlink extractor
///
/// Text to HTML renderer that prints only links with markup as
/// a list, one per line. Links are clickable and only their
/// _link text_ is shown (the part enclosed with `<a>` and `</a>`).
///
/// ## Markdown
/// ```
/// use parse_hyperlinks::renderer::links2html;
/// use std::borrow::Cow;
///
/// let i = r#"abc[text0](dest0 "title0")abc
/// abc[text1][label1]abc
/// abc[text2](dest2 "title2")abc
/// [text3]: dest3 "title3"
/// [label1]: dest1 "title1"
/// abc[text3]abc
/// "#;
///
/// let expected = "\
/// <a href=\"dest0\" title=\"title0\">text0</a>
/// <a href=\"dest1\" title=\"title1\">text1</a>
/// <a href=\"dest2\" title=\"title2\">text2</a>
/// <a href=\"dest3\" title=\"title3\">text3</a>
/// ";
/// let res = links2html(i);
/// assert_eq!(res, expected);
/// ```
///
/// ### Rendered text
///
/// This is how the rendered text looks like in the browser:
///
/// <a href="dest0" title="title0">text0</a>
/// <a href="dest1" title="title1">text1</a>
/// <a href="dest2" title="title2">text2</a>
/// <a href="dest3" title="title3">text3</a>
///
///
/// ## reStructuredText
/// ```
/// use parse_hyperlinks::renderer::links2html;
/// use std::borrow::Cow;
///
/// let i = r#"
/// abc `text1 <label1_>`_abc
/// abc text2_ abc
/// abc text3__ abc
/// abc text_label4_ abc
/// abc text5__ abc
/// .. _label1: dest1
/// .. _text2: dest2
/// .. __: dest3
/// __ dest5
/// "#;
///
/// let expected = "\
/// <a href=\"dest1\" title=\"\">text1</a>
/// <a href=\"dest2\" title=\"\">text2</a>
/// <a href=\"dest3\" title=\"\">text3</a>
/// <a href=\"dest5\" title=\"\">text5</a>
/// ";
///
/// let res = links2html(i);
/// assert_eq!(res, expected);
/// ```
///
/// ### Rendered text
///
/// This is how the rendered text looks like in the browser:
///
/// <a href="dest1" title="">text1</a>
/// <a href="dest2" title="">text2</a>
/// <a href="dest3" title="">text3</a>
/// <a href="dest5" title="">text5</a>
///
///
/// ## Asciidoc
///
/// ```
/// use parse_hyperlinks::renderer::links2html;
/// use std::borrow::Cow;
///
/// let i = r#"abc https://dest0[text0]abc
/// abc link:https://dest1[text1]abc
/// abc{label2}[text2]abc
/// abc{label3}abc
/// :label2: https://dest2
/// :label3: https://dest3
/// "#;
///
/// let expected = "\
/// <a href=\"https://dest0\" title=\"\">text0</a>
/// <a href=\"https://dest1\" title=\"\">text1</a>
/// <a href=\"https://dest2\" title=\"\">text2</a>
/// <a href=\"https://dest3\" title=\"\">https:&#x2F;&#x2F;dest3</a>
/// ";
///
/// let res = links2html(i);
/// assert_eq!(res, expected);
/// ```
///
/// ### Rendered text
///
/// This is how the rendered text looks like in the browser:
///
/// <a href="https://dest0" title="">text0</a>
/// <a href="https://dest1" title="">text1</a>
/// <a href="https://dest2" title="">text2</a>
/// <a href="https://dest3" title="">https:&#x2F;&#x2F;dest3</a>
///
///
/// ## Wikitext
///
/// ```
/// use parse_hyperlinks::renderer::links2html;
/// use std::borrow::Cow;
///
/// let i = r#"abc[https://dest0 text0]abc
/// "#;
///
/// let expected = "\
/// <a href=\"https://dest0\" title=\"\">text0</a>
/// ";
///
/// let res = links2html(i);
/// assert_eq!(res, expected);
/// ```
///
/// ### Rendered text
///
/// This is how the rendered text looks like in the browser:
///
/// <a href="https://dest0" title="">text0</a>
///
///
/// ## HTML
///
/// HTML _inline links_ are sanitized and listed, one per line.
///
/// ```
/// use parse_hyperlinks::renderer::links2html;
/// use std::borrow::Cow;
///
/// let i = r#"abc<a href="dest1" title="title1">text1</a>abc
/// abc<a href="dest2" title="title2">text2</a>abc"#;
///
/// let expected = "\
/// <a href=\"dest1\" title=\"title1\">text1</a>
/// <a href=\"dest2\" title=\"title2\">text2</a>
/// ";
///
/// let res = links2html(i);
/// assert_eq!(res, expected);
/// ```
///
/// ### Rendered text
///
/// This is how the rendered text looks like in the browser:
///
/// <a href="dest1" title="title1">text1</a>
/// <a href="dest2" title="title2">text2</a>
///
#[inline]
pub fn links2html(input: &str) -> String {
    let mut output = Vec::new();
    links2html_writer(input, &mut output).unwrap_or_default();
    // We know this is safe, because only `str` have been written into `output`.
    // So the following would be fine, but I want to keep this crate `unsafe`-free.
    //    unsafe {String::from_utf8_unchecked(output)}
    String::from_utf8(output).unwrap_or_default()
}

/// # Hyperlink extractor
///
/// Same as `links2html()`, but it uses `Write` for output. This function
/// allocates much less memory and is faster because it avoids copying.
///
/// Usage example:
/// ```no_run
/// use parse_hyperlinks::renderer::links2html_writer;
/// use std::io;
/// use std::io::Read;
/// fn main() -> Result<(), ::std::io::Error> {
///     let mut stdin = String::new();
///     Read::read_to_string(&mut io::stdin(), &mut stdin)?;
///
///     links2html_writer(&stdin, &mut io::stdout())?;
///
///     Ok(())
/// }
/// ```
pub fn links2html_writer<'a, S: 'a + AsRef<str>, W: Write>(
    input: S,
    output: &mut W,
) -> Result<(), io::Error> {
    let input = input.as_ref();

    let verb_renderer = |_| Cow::Borrowed("");

    let link_renderer = |(_, (text, dest, title)): (_, (String, String, String))| {
        let mut s = String::new();
        s.push_str(r#"<a href=""#);
        s.push_str(&*dest);
        s.push_str(r#"" title=""#);
        s.push_str(&*title);
        s.push_str(r#"">"#);
        s.push_str(&*text);
        s.push_str("</a>\n");
        s
    };

    render(input, "", "", verb_renderer, link_renderer, false, output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_links2html() {
        let i = r#"abc[text1][label1]abc
abc [text2](destination2 "title2")
  [label3]: destination3 "title3"
  [label1]: destination1 "title1"
abc[label3]abc[label4]abc
"#;

        let expected = r#"<pre>abc<a href="destination1" title="title1">text1</a>abc
abc <a href="destination2" title="title2">text2</a>
  <a href="destination3" title="title3">[label3]: destination3 &quot;title3&quot;</a>
  <a href="destination1" title="title1">[label1]: destination1 &quot;title1&quot;</a>
abc<a href="destination3" title="title3">label3</a>abc[label4]abc
</pre>"#;
        let res = text_links2html(i);
        //eprintln!("{}", res);
        assert_eq!(res, expected);
    }

    #[test]
    fn test_text_links2html2() {
        let i = r#"abc
abc
"#;

        let expected = r#"<pre>abc
abc
</pre>"#;
        let res = text_links2html(i);
        //eprintln!("{}", res);
        assert_eq!(res, expected);
    }

    #[test]
    fn test_text_rawlinks2html() {
        let i = r#"abc[text1][label1]abc
abc [text2](destination2 "title2")
  [label3]: destination3 "title3"
  [label1]: destination1 "title1"
abc[label3]abc[label4]abc
"#;

        let expected = r#"<pre>abc<a href="destination1" title="title1">[text1][label1]</a>abc
abc <a href="destination2" title="title2">[text2](destination2 "title2")</a>
  <a href="destination3" title="title3">[label3]: destination3 "title3"</a>
  <a href="destination1" title="title1">[label1]: destination1 "title1"</a>
abc<a href="destination3" title="title3">[label3]</a>abc[label4]abc
</pre>"#;
        let res = text_rawlinks2html(i);
        //eprintln!("{}", res);
        assert_eq!(res, expected);
    }

    #[test]
    fn test_links2html() {
        let i = r#"abc[text1][label1]abc
abc [text2](destination2 "title2")
  [label3]: destination3 "title3"
  [label1]: destination1 "title1"
abc[label3]abc[label4]abc
"#;

        let expected = r#"<a href="destination1" title="title1">text1</a>
<a href="destination2" title="title2">text2</a>
<a href="destination3" title="title3">label3</a>
"#;
        let res = links2html(i);
        //eprintln!("{}", res);
        assert_eq!(res, expected);
    }
}
