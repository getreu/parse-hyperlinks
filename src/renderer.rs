//! A set of functions providing markup source code to HTML renderer, that make
//! hyperlinks clickable.

use crate::iterator::Hyperlink;
use html_escape::encode_double_quoted_attribute;
use html_escape::encode_safe;
use html_escape::encode_text;
use std::borrow::Cow;

#[inline]
fn render<'a, O, P>(input: &'a str, verb_renderer: O, link_renderer: P) -> String
where
    O: Fn(Cow<'a, str>) -> Cow<'a, str>,
    P: Fn((Cow<'a, str>, (String, String, String))) -> String,
{
    let mut rest = Cow::from("");

    let mut s = String::from("<pre>");
    for ((skipped2, consumed2, remaining2), (text2, dest2, title2)) in Hyperlink::new(&input) {
        let skipped = encode_text(skipped2);
        let consumed = encode_text(consumed2);
        let remaining = encode_text(remaining2);
        let text = encode_safe(&text2).to_string();
        let dest = encode_double_quoted_attribute(&dest2).to_string();
        let title = encode_double_quoted_attribute(&title2).to_string();
        s.push_str(&verb_renderer(skipped));
        let rendered_link = link_renderer((consumed, (text, dest, title)));
        s.push_str(&rendered_link);
        rest = remaining;
    }
    s.push_str(&verb_renderer(rest));
    s.push_str("</pre>");
    s
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
/// [text3]: dest3 \"title3\"
/// [label1]: dest1 \"title1\"
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
/// [text3]: dest3 "title3"
/// [label1]: dest1 "title1"
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
/// .. _label1: dest1
/// .. _text2: dest2
/// .. __: dest3
/// __ dest5
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
/// .. _label1: dest1
/// .. _text2: dest2
/// .. __: dest3
/// __ dest5
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
/// abc {label2}[text2]abc
/// abc {label3}abc
/// :label2: https://dest2
/// :label3: https://dest3
/// "#;
///
/// let expected = "\
/// <pre>abc <a href=\"https://dest0\" title=\"\">text0</a>abc
/// abc <a href=\"https://dest1\" title=\"\">text1</a>abc
/// abc <a href=\"https://dest2\" title=\"\">text2</a>abc
/// abc <a href=\"https://dest3\" title=\"\">https:&#x2F;&#x2F;dest3</a>abc
/// :label2: https://dest2
/// :label3: https://dest3
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
/// abc <a href="https://dest2" title="">text2</a>abc
/// abc <a href="https://dest3" title="">https://dest3</a>abc
/// :label2: https://dest2\n:label3: https://dest3
/// </pre>
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
pub fn text_links2html(input: &str) -> String {
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

    render(input, verb_renderer, link_renderer)
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
/// [text3]: dest3 \"title3\"
/// [label1]: dest1 \"title1\"
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
/// [text3]: dest3 "title3"
/// [label1]: dest1 "title1"
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
/// <pre>\nabc <a href=\"dest1\" title=\"\">`text1 &lt;label1_&gt;`_</a>abc
/// abc <a href=\"dest2\" title=\"\">text2_</a> abc
/// abc <a href=\"dest3\" title=\"\">text3__</a> abc
/// abc text_label4_ abc\nabc <a href=\"dest5\" title=\"\">text5__</a> abc
/// .. _label1: dest1
/// .. _text2: dest2
/// .. __: dest3
/// __ dest5
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
/// abc <a href="dest1" title="">`text1 <label1_>`_</a>abc
/// abc <a href="dest2" title="">text2_</a> abc
/// abc <a href="dest3" title="">text3__</a> abc
/// abc text_label4_ abc
/// abc <a href="dest5" title="">text5__</a> abc
/// .. _label1: dest1
/// .. _text2: dest2
/// .. __: dest3\n__ dest5
/// </pre>
///
/// ## Asciidoc
///
/// ```
/// use parse_hyperlinks::renderer::text_rawlinks2html;
/// use std::borrow::Cow;
///
/// let i = r#"abc
/// abc https://dest0[text0]abc
/// abc link:https://dest1[text1]abc
/// abc {label2}[text2]abc
/// abc {label3}abc
/// :label2: https://dest2
/// :label3: https://dest3
/// "#;
///
/// let expected = "\
/// <pre>abc
/// abc <a href=\"https://dest0\" title=\"\">https://dest0[text0]</a>abc
/// abc <a href=\"https://dest1\" title=\"\">link:https://dest1[text1]</a>abc
/// abc <a href=\"https://dest2\" title=\"\">{label2}[text2]</a>abc
/// abc <a href=\"https://dest3\" title=\"\">{label3}</a>abc
/// :label2: https://dest2\n:label3: https://dest3
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
/// <pre>abc
/// abc <a href="https://dest0" title="">https://dest0[text0]</a>abc
/// abc <a href="https://dest1" title="">link:https://dest1[text1]</a>abc
/// abc <a href="https://dest2" title="">{label2}[text2]</a>abc
/// abc <a href="https://dest3" title="">{label3}</a>abc
/// :label2: https://dest2\n:label3: https://dest3
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
pub fn text_rawlinks2html<'a>(input: &'a str) -> String {
    let verb_renderer = |verb: Cow<'a, str>| verb;

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

    render(input, verb_renderer, link_renderer)
}

/// # Hyperlink extractor
///
/// Text to HTML renderer that prints only links with markup as
/// a list, one per line. Links are clickable and only their
/// _link text_ is shown (the part enclosed with `<a>` and `</a>`).
///
/// ## Markdown
/// ```
/// use parse_hyperlinks::renderer::link_list2html;
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
/// <pre><a href=\"dest0\" title=\"title0\">text0</a>
/// <a href=\"dest1\" title=\"title1\">text1</a>
/// <a href=\"dest2\" title=\"title2\">text2</a>
/// <a href=\"dest3\" title=\"title3\">text3</a>
/// </pre>";
/// let res = link_list2html(i);
/// assert_eq!(res, expected);
/// ```
///
/// ### Rendered text
///
/// This is how the rendered text looks like in the browser:
///
/// <pre><a href="dest0" title="title0">text0</a>
/// <a href="dest1" title="title1">text1</a>
/// <a href="dest2" title="title2">text2</a>
/// <a href="dest3" title="title3">text3</a>
/// </pre>
///
/// ## reStructuredText
/// ```
/// use parse_hyperlinks::renderer::link_list2html;
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
/// <pre><a href=\"dest1\" title=\"\">text1</a>
/// <a href=\"dest2\" title=\"\">text2</a>
/// <a href=\"dest3\" title=\"\">text3</a>
/// <a href=\"dest5\" title=\"\">text5</a>
/// </pre>\
/// ";
///
/// let res = link_list2html(i);
/// assert_eq!(res, expected);
/// ```
///
/// ### Rendered text
///
/// This is how the rendered text looks like in the browser:
///
/// <pre><a href="dest1" title="">text1</a>
/// <a href="dest2" title="">text2</a>
/// <a href="dest3" title="">text3</a>
/// <a href="dest5" title="">text5</a>
/// </pre>
///
/// ## Asciidoc
///
/// ```
/// use parse_hyperlinks::renderer::link_list2html;
/// use std::borrow::Cow;
///
/// let i = r#"abc
/// abc https://dest0[text0]abc
/// abc link:https://dest1[text1]abc
/// abc {label2}[text2]abc
/// abc {label3}abc
/// :label2: https://dest2
/// :label3: https://dest3
/// "#;
///
/// let expected = "\
/// <pre><a href=\"https://dest0\" title=\"\">text0</a>
/// <a href=\"https://dest1\" title=\"\">text1</a>
/// <a href=\"https://dest2\" title=\"\">text2</a>
/// <a href=\"https://dest3\" title=\"\">https:&#x2F;&#x2F;dest3</a>
/// </pre>";
///
/// let res = link_list2html(i);
/// assert_eq!(res, expected);
/// ```
///
/// ### Rendered text
///
/// This is how the rendered text looks like in the browser:
///
/// <pre><a href="https://dest0" title="">text0</a>
/// <a href="https://dest1" title="">text1</a>
/// <a href="https://dest2" title="">text2</a>
/// <a href="https://dest3" title="">https://dest3</a>
/// </pre>
///
///
/// ## HTML
///
/// HTML _inline links_ are sanitized and listed, one per line.
///
/// ```
/// use parse_hyperlinks::renderer::link_list2html;
/// use std::borrow::Cow;
///
/// let i = r#"abc<a href="dest1" title="title1">text1</a>abc
/// abc<a href="dest2" title="title2">text2</a>abc"#;
///
/// let expected = "<pre>\
/// <a href=\"dest1\" title=\"title1\">text1</a>
/// <a href=\"dest2\" title=\"title2\">text2</a>
/// </pre>";
///
/// let res = link_list2html(i);
/// assert_eq!(res, expected);
/// ```
///
/// ### Rendered text
///
/// This is how the rendered text looks like in the browser:
///
/// <pre>
/// <a href="dest1" title="title1">text1</a>
/// <a href="dest2" title="title2">text2</a>
/// </pre>
///
pub fn link_list2html(input: &str) -> String {
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

    render(input, verb_renderer, link_renderer)
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
  [label3]: destination3 "title3"
  [label1]: destination1 "title1"
abc<a href="destination3" title="title3">label3</a>abc[label4]abc
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
  [label3]: destination3 "title3"
  [label1]: destination1 "title1"
abc<a href="destination3" title="title3">[label3]</a>abc[label4]abc
</pre>"#;
        let res = text_rawlinks2html(i);
        //eprintln!("{}", res);
        assert_eq!(res, expected);
    }

    #[test]
    fn test_link_list2html() {
        let i = r#"abc[text1][label1]abc
abc [text2](destination2 "title2")
  [label3]: destination3 "title3"
  [label1]: destination1 "title1"
abc[label3]abc[label4]abc
"#;

        let expected = r#"<pre><a href="destination1" title="title1">text1</a>
<a href="destination2" title="title2">text2</a>
<a href="destination3" title="title3">label3</a>
</pre>"#;
        let res = link_list2html(i);
        //eprintln!("{}", res);
        assert_eq!(res, expected);
    }
}
