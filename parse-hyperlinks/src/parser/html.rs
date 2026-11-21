//! This module implements parsers for HTML hyperlinks.
#![allow(dead_code)]
#![allow(clippy::type_complexity)]

use crate::parser::Link;
use html_escape::decode_html_entities;
use nom::AsChar;
use nom::Parser;
use nom::branch::alt;
use nom::bytes::complete::is_not;
use nom::bytes::complete::tag;
use nom::error::Error;
use nom::error::ErrorKind;
use std::borrow::Cow;
use std::ops::Range;

/// The elements whose content HTML does not parse as markup, with the
/// prefix of their end tag.
const RAW_TEXT_ELEMENTS: &[(&str, &str); 2] = &[("<script", "</script"), ("<style", "</style")];

/// Case insensitive `str::find()` for an ASCII `needle`. A match can only
/// start on a character boundary, as every byte of an ASCII needle is
/// below 0x80 while every UTF-8 continuation byte is not.
fn find_no_case(haystack: &str, needle: &str) -> Option<usize> {
    haystack
        .as_bytes()
        .windows(needle.len())
        .position(|w| w.eq_ignore_ascii_case(needle.as_bytes()))
}

/// Consumes one HTML construct whose content is not markup and returns
/// that content: the text of a comment, or the raw text of a `<script>` or
/// `<style>` element. Returns an error when `i` does not start with one.
/// Wrap the parser in `nom::combinator::recognize()` to obtain the whole
/// construct, delimiters included, instead.
///
/// Scanners searching HTML for a construct of their own interest should
/// skip what this parser consumes: whatever looks like a tag in there is
/// none. `<pre>` and `<code>` are deliberately not included; their content
/// is ordinary markup.
///
/// An unterminated construct extends to the end of the input, as the HTML
/// specification prescribes for end-of-file inside a comment or inside the
/// raw text of such an element.
///
/// ```
/// use nom::Parser;
/// use nom::combinator::recognize;
/// use parse_hyperlinks::parser::html::html_inert;
///
/// // The parser returns the content: the comment delimiters are gone,
/// // and whatever markup the comment holds is text, not markup.
/// let (i, r) = html_inert(r#"<!-- <a href="d">t</a> -->abc"#).unwrap();
/// assert_eq!((i, r), ("abc", r#" <a href="d">t</a> "#));
///
/// // `recognize()` returns the input the parser consumed instead, which
/// // is the whole construct. Same input, same remainder, other result.
/// let (i, r) = recognize(html_inert).parse(r#"<!-- <a href="d">t</a> -->abc"#).unwrap();
/// assert_eq!((i, r), ("abc", r#"<!-- <a href="d">t</a> -->"#));
///
/// // A raw text element, likewise.
/// let (i, r) = html_inert("<script>var a = '<a href=\"d\">';</script>abc").unwrap();
/// assert_eq!((i, r), ("abc", "var a = '<a href=\"d\">';"));
///
/// assert!(html_inert("<a href=\"d\">t</a>").is_err());
/// ```
pub fn html_inert(i: &'_ str) -> nom::IResult<&'_ str, &'_ str> {
    /// Returns `content` and the input past the `consumed` first bytes,
    /// the way a nom parser does.
    fn split(i: &str, consumed: usize, content: Range<usize>) -> nom::IResult<&str, &str> {
        Ok((&i[consumed..], &i[content]))
    }

    // A comment: `<!--`, then everything up to and including the first
    // `-->`. HTML has no nested comments.
    const COMMENT_OPEN: &str = "<!--";
    const COMMENT_CLOSE: &str = "-->";
    if i.starts_with(COMMENT_OPEN) {
        return match i[COMMENT_OPEN.len()..].find(COMMENT_CLOSE) {
            Some(p) => split(
                i,
                COMMENT_OPEN.len() + p + COMMENT_CLOSE.len(),
                COMMENT_OPEN.len()..COMMENT_OPEN.len() + p,
            ),
            None => split(i, i.len(), COMMENT_OPEN.len()..i.len()),
        };
    }

    // A raw text element: its opening tag, its text, and its end tag.
    for (open, close) in RAW_TEXT_ELEMENTS {
        if i.len() < open.len() || !i[..open.len()].eq_ignore_ascii_case(open) {
            continue;
        }
        // The tag name must end here, so `<styles>` is not `<style>`.
        if !i[open.len()..]
            .chars()
            .next()
            .is_none_or(|c| c.is_whitespace() || c == '>' || c == '/')
        {
            continue;
        }
        // The raw text starts after the opening tag.
        let Some(text_start) = i[open.len()..].find('>').map(|p| open.len() + p + 1) else {
            // An opening tag that never ends: no content at all.
            return split(i, i.len(), i.len()..i.len());
        };
        return match find_no_case(&i[text_start..], close) {
            Some(p) => {
                let text_end = text_start + p;
                // Consume the end tag's own `>` as well, which `close`
                // does not contain: `</script >` is an end tag too.
                let after_name = text_end + close.len();
                let consumed = match i[after_name..].find('>') {
                    Some(q) => after_name + q + 1,
                    None => i.len(),
                };
                split(i, consumed, text_start..text_end)
            }
            None => split(i, i.len(), text_start..i.len()),
        };
    }

    Err(nom::Err::Error(Error::new(i, ErrorKind::Tag)))
}

/// Byte ranges of everything in `html` that `html_inert()` consumes, in
/// document order, each covering the whole construct with its delimiters
/// - what a scanner wants to jump over. For a scanner that walks byte
/// offsets instead of parsing, e.g. to leave a commented out heading
/// alone.
///
/// ```
/// use parse_hyperlinks::parser::html::inert_regions;
///
/// let html = "a<!-- b --><h1>c</h1><style>d</style>";
/// assert_eq!(inert_regions(html), vec![1..11, 21..37]);
/// ```
pub fn inert_regions(html: &str) -> Vec<Range<usize>> {
    let mut regions = Vec::new();
    let mut i = 0;
    while let Some(rel) = html[i..].find('<') {
        let start = i + rel;
        match html_inert(&html[start..]) {
            // The parser returns the content, so measure the construct by
            // what is left of the input.
            Ok((remaining, _)) => {
                let end = html.len() - remaining.len();
                regions.push(start..end);
                i = end;
            }
            Err(_) => i = start + 1,
        }
    }
    regions
}

/// Wrapper around `html_text2dest()` that packs the result in
/// `Link::Text2Dest`.
pub fn html_text2dest_link(i: &'_ str) -> nom::IResult<&'_ str, Link<'_>> {
    let (i, (te, de, ti)) = html_text2dest(i)?;
    Ok((i, Link::Text2Dest(te, de, ti)))
}

/// Parse an HTML _inline hyperlink_.
///
/// It returns either `Ok((i, (link_text, link_destination, link_title)))` or some error.
///
/// The parser expects to start at the link start (`<`) to succeed.
/// ```
/// use parse_hyperlinks::parser::Link;
/// use parse_hyperlinks::parser::html::html_text2dest;
/// use std::borrow::Cow;
///
/// assert_eq!(
///   html_text2dest(r#"<a href="destination" title="title">name</a>abc"#),
///   Ok(("abc", (Cow::from("name"), Cow::from("destination"), Cow::from("title"))))
/// );
/// ```
pub fn html_text2dest(
    i: &'_ str,
) -> nom::IResult<&'_ str, (Cow<'_, str>, Cow<'_, str>, Cow<'_, str>)> {
    let (i, ((link_destination, link_title), link_text)) = nom::sequence::terminated(
        nom::sequence::pair(
            tag_a_opening,
            alt((
                nom::bytes::complete::take_until("</a>"),
                nom::bytes::complete::take_until("</A>"),
            )),
        ),
        // HTML is case insensitive. XHTML, that is being XML is case sensitive.
        // Here we deal with HTML.
        alt((tag("</a>"), tag("</A>"))),
    )
    .parse(i)?;
    let link_text = decode_html_entities(link_text);
    Ok((i, (link_text, link_destination, link_title)))
}

/// Parses a `<a ...>` opening tag and returns
/// either `Ok((i, (link_destination, link_title)))` or some error.
pub(crate) fn tag_a_opening(i: &'_ str) -> nom::IResult<&'_ str, (Cow<'_, str>, Cow<'_, str>)> {
    nom::sequence::delimited(
        // HTML is case insensitive. XHTML, that is being XML is case sensitive.
        // Here we deal with HTML.
        alt((tag("<a "), tag("<A "))),
        nom::combinator::map_parser(is_not(">"), parse_attributes),
        tag(">"),
    )
    .parse(i)
}

/// Parses an attribute name. HTML forbids only whitespace, `"`, `'`, `>`,
/// `/` and `=` in a name, so a name may well contain a hyphen or a colon
/// (`data-id`, `aria-label`, `xml:lang`). `alphanumeric1` stopped at the
/// first such character, and that did not merely skip the one attribute:
/// `attribute_list()` ends at the first element it cannot parse, so a
/// single `data-*` attribute hid every attribute behind it. The first
/// character must still be a letter, so stray punctuation inside a tag
/// does not parse as a name.
fn attribute_name(i: &'_ str) -> nom::IResult<&'_ str, &'_ str> {
    nom::combinator::verify(
        nom::bytes::complete::take_while1(|c: char| {
            !c.is_whitespace() && !matches!(c, '"' | '\'' | '>' | '/' | '=')
        }),
        |s: &str| s.chars().next().is_some_and(|c| c.is_alpha()),
    )
    .parse(i)
}

/// Parses attributes and returns `Ok((name, value))`.
/// Boolean attributes are ignored, but silently consumed.
fn attribute(i: &'_ str) -> nom::IResult<&'_ str, (&'_ str, Cow<'_, str>)> {
    alt((
        nom::sequence::pair(
            attribute_name,
            alt((
                nom::combinator::value(Cow::from(""), tag(r#"="""#)),
                nom::combinator::value(Cow::from(""), tag(r#"=''"#)),
                nom::combinator::map(
                    nom::sequence::delimited(tag("=\""), is_not("\""), tag("\"")),
                    |s: &str| decode_html_entities(s),
                ),
                nom::combinator::map(
                    nom::sequence::delimited(tag("='"), is_not("'"), tag("'")),
                    |s: &str| decode_html_entities(s),
                ),
                nom::combinator::map(nom::sequence::preceded(tag("="), is_not(" ")), |s: &str| {
                    decode_html_entities(s)
                }),
            )),
        ),
        // Consume boolean attributes.
        nom::combinator::value(("", Cow::from("")), attribute_name),
    ))
    .parse(i)
}

/// Parses a whitespace separated list of attributes and returns a vector of (name, value).
pub fn attribute_list(i: &'_ str) -> nom::IResult<&'_ str, Vec<(&'_ str, Cow<'_, str>)>> {
    let i = i.trim();
    nom::multi::separated_list1(nom::character::complete::multispace1, attribute).parse(i)
}

/// Extracts the `href` and `title` attributes and returns
/// `Ok((link_destination, link_title))`. `link_title` can be empty,
/// `link_destination` not.
fn parse_attributes(i: &'_ str) -> nom::IResult<&'_ str, (Cow<'_, str>, Cow<'_, str>)> {
    let (i, attributes) = attribute_list(i)?;
    let mut href = Cow::Borrowed("");
    let mut title = Cow::Borrowed("");

    for (name, value) in attributes {
        if name == "href" {
            // Make sure `href` is empty, it can appear only
            // once.
            if !(*href).is_empty() {
                return Err(nom::Err::Error(Error::new(name, ErrorKind::ManyMN)));
            }
            href = value;
        } else if name == "title" {
            // Make sure `title` is empty, it can appear only
            // once.
            if !(*title).is_empty() {
                return Err(nom::Err::Error(Error::new(name, ErrorKind::ManyMN)));
            }
            title = value;
        }
    }

    // Assure that `href` is not empty.
    if (*href).is_empty() {
        return Err(nom::Err::Error(Error::new(i, ErrorKind::Eof)));
    };

    Ok((i, (href, title)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom::combinator::recognize;

    #[test]
    fn test_html_text2dest() {
        let expected = (
            "abc",
            (
                Cow::from("W3Schools"),
                Cow::from("https://www.w3schools.com/"),
                Cow::from("W3S"),
            ),
        );
        assert_eq!(
            html_text2dest(r#"<a title="W3S" href="https://www.w3schools.com/">W3Schools</a>abc"#)
                .unwrap(),
            expected
        );
        assert_eq!(
            html_text2dest(r#"<A title="W3S" href="https://www.w3schools.com/">W3Schools</A>abc"#)
                .unwrap(),
            expected
        );

        let expected = ("abc", (Cow::from("<n>"), Cow::from("h"), Cow::from("t")));
        assert_eq!(
            html_text2dest(r#"<a title="t" href="h">&lt;n&gt;</a>abc"#).unwrap(),
            expected
        );

        let expected = ("abc", (Cow::from("name"), Cow::from("url"), Cow::from("")));
        assert_eq!(
            html_text2dest(r#"<a href="url" title="" >name</a>abc"#).unwrap(),
            expected
        );

        let expected = (
            "abc",
            (Cow::from("na</me"), Cow::from("url"), Cow::from("")),
        );
        assert_eq!(
            html_text2dest(r#"<a href="url" title="" >na</me</A>abc"#).unwrap(),
            expected
        );

        // The leading `<` is no attribute name: `attribute_name()` rejects
        // it because a name must start with a letter. (The error code is
        // `Verify`, the one that guard reports, not `AlphaNumeric`.)
        let expected = nom::Err::Error(nom::error::Error::new(
            r#"<a href="url" title="" >name</a abc"#,
            nom::error::ErrorKind::Verify,
        ));
        assert_eq!(
            parse_attributes(r#"<a href="url" title="" >name</a abc"#).unwrap_err(),
            expected
        );

        let expected = (
            "abc",
            (
                Cow::from(
                    "<img src=\"w3html.gif\" alt=\"W3Schools.com \"width=\"100\" height=\"132\">",
                ),
                Cow::from("https://blog.getreu.net"),
                Cow::from(""),
            ),
        );
        assert_eq!(
            html_text2dest(
                "<a href=\"https://blog.getreu.net\">\
                              <img src=\"w3html.gif\" alt=\"W3Schools.com \"\
                              width=\"100\" height=\"132\">\
                              </a>abc"
            )
            .unwrap(),
            expected
        );
    }

    #[test]
    fn test_tag_a_opening() {
        let expected = (
            "abc",
            (Cow::from("http://getreu.net"), Cow::from("My blog")),
        );
        assert_eq!(
            tag_a_opening(r#"<a href="http://getreu.net" title="My blog">abc"#).unwrap(),
            expected
        );
        assert_eq!(
            tag_a_opening(r#"<A href="http://getreu.net" title="My blog">abc"#).unwrap(),
            expected
        );
    }

    #[test]
    fn test_parse_attributes() {
        let expected = ("", (Cow::from("http://getreu.net"), Cow::from("My blog")));
        assert_eq!(
            parse_attributes(r#"abc href="http://getreu.net" abc title="My blog" abc"#).unwrap(),
            expected
        );

        let expected = nom::Err::Error(nom::error::Error::new(
            "href",
            nom::error::ErrorKind::ManyMN,
        ));
        assert_eq!(
            parse_attributes(r#" href="http://getreu.net" href="http://blog.getreu.net" "#)
                .unwrap_err(),
            expected
        );

        let expected = nom::Err::Error(nom::error::Error::new(
            "title",
            nom::error::ErrorKind::ManyMN,
        ));
        assert_eq!(
            parse_attributes(r#" href="http://getreu.net" title="a" title="b" "#).unwrap_err(),
            expected
        );

        let expected = nom::Err::Error(nom::error::Error::new("", nom::error::ErrorKind::Eof));
        assert_eq!(
            parse_attributes(r#" title="title" "#).unwrap_err(),
            expected
        );
    }

    #[test]
    fn test_attribute_list() {
        let expected = (
            "",
            vec![
                ("", Cow::from("")),
                ("href", Cow::from("http://getreu.net")),
                ("", Cow::from("")),
                ("title", Cow::from("My blog")),
                ("", Cow::from("")),
            ],
        );
        assert_eq!(
            attribute_list(r#"abc href="http://getreu.net" abc title="My blog" abc"#).unwrap(),
            expected
        );
    }

    #[test]
    fn test_attribute() {
        let expected = (" abc", ("href", Cow::from("http://getreu.net")));
        assert_eq!(
            attribute(r#"href="http://getreu.net" abc"#).unwrap(),
            expected
        );
        assert_eq!(
            attribute(r#"href='http://getreu.net' abc"#).unwrap(),
            expected
        );
        // Only allowed when no space in value.
        assert_eq!(
            attribute(r#"href=http://getreu.net abc"#).unwrap(),
            expected
        );

        let expected = (" abc", ("href", Cow::from("http://getreu.net/<>")));
        assert_eq!(
            attribute(r#"href="http://getreu.net/&lt;&gt;" abc"#).unwrap(),
            expected
        );
        assert_eq!(
            attribute(r#"href='http://getreu.net/&lt;&gt;' abc"#).unwrap(),
            expected
        );
        // Only allowed when no space in value.
        assert_eq!(
            attribute(r#"href=http://getreu.net/&lt;&gt; abc"#).unwrap(),
            expected
        );

        let expected = (" abc", ("", Cow::from("")));
        assert_eq!(attribute("bool abc").unwrap(), expected);

        let expected = nom::Err::Error(nom::error::Error::new(
            "1name",
            nom::error::ErrorKind::Verify,
        ));
        assert_eq!(attribute("1name").unwrap_err(), expected);

        let expected = nom::Err::Error(nom::error::Error::new(
            r#"1name="http://getreu.net"#,
            nom::error::ErrorKind::Verify,
        ));
        assert_eq!(
            attribute(r#"1name="http://getreu.net"#).unwrap_err(),
            expected
        );
    }

    #[test]
    fn test_html_inert() {
        // The content of a comment, delimiters excluded.
        assert_eq!(
            html_inert(r#"<!-- <a href="d">t</a> -->abc"#).unwrap(),
            ("abc", r#" <a href="d">t</a> "#)
        );
        // The whole construct, for a caller that asks for it.
        assert_eq!(
            recognize(html_inert)
                .parse(r#"<!-- <a href="d">t</a> -->abc"#)
                .unwrap(),
            ("abc", r#"<!-- <a href="d">t</a> -->"#)
        );
        // Unterminated: the HTML specification closes it at end of file.
        assert_eq!(html_inert("<!-- abc").unwrap(), ("", " abc"));
        // Raw text elements, case insensitive, attributes and all.
        assert_eq!(html_inert("<script>a<b</script>abc").unwrap(), ("abc", "a<b"));
        assert_eq!(
            html_inert(r#"<SCRIPT type="text/javascript">a</SCRIPT>abc"#).unwrap(),
            ("abc", "a")
        );
        assert_eq!(html_inert("<style>a</style >abc").unwrap(), ("abc", "a"));
        assert_eq!(html_inert("<script>abc").unwrap(), ("", "abc"));
        // An opening tag that never ends has no content.
        assert_eq!(html_inert("<script src=").unwrap(), ("", ""));
        // The tag name must end: these are other elements.
        assert!(html_inert("<styles>a</styles>").is_err());
        assert!(html_inert("<scripting>a</scripting>").is_err());
        // Ordinary markup is not inert, `<pre>` and `<code>` included.
        assert!(html_inert(r#"<a href="d">t</a>"#).is_err());
        assert!(html_inert("<pre><h1>t</h1></pre>").is_err());
        assert!(html_inert("abc").is_err());
    }

    #[test]
    fn test_inert_regions() {
        let html = "a<!-- b --><h1>c</h1><style>d</style>";
        assert_eq!(inert_regions(html), vec![1..11, 21..37]);
        // A `<` that opens nothing inert is passed over.
        assert_eq!(inert_regions("a < b <h1>c</h1>"), vec![]);
        // Regions come in document order and do not overlap.
        assert_eq!(
            inert_regions("<!--a--><!--b-->"),
            vec![0..8, 8..16]
        );
    }

    /// A name may contain any character but whitespace, `"`, `'`, `>`,
    /// `/` and `=`, so hyphenated and namespaced attributes parse as one
    /// name each.
    #[test]
    fn test_attribute_hyphenated_name() {
        assert_eq!(
            attribute(r#"data-id="x" abc"#).unwrap(),
            (" abc", ("data-id", Cow::from("x")))
        );
        assert_eq!(
            attribute(r#"aria-label="Close" abc"#).unwrap(),
            (" abc", ("aria-label", Cow::from("Close")))
        );
        assert_eq!(
            attribute(r#"xml:lang="en" abc"#).unwrap(),
            (" abc", ("xml:lang", Cow::from("en")))
        );
        // Unquoted values stop at whitespace, hyphen in the name and all.
        assert_eq!(
            attribute("data-level=2 abc").unwrap(),
            (" abc", ("data-level", Cow::from("2")))
        );
    }

    /// `attribute_list()` stops at the first attribute it cannot parse,
    /// so a name it rejects does not just get skipped - it hides every
    /// attribute behind it. A hyphenated name must therefore not end the
    /// list.
    #[test]
    fn test_attribute_list_keeps_going_after_hyphenated_name() {
        let expected = (
            "",
            vec![
                ("data-id", Cow::from("x")),
                ("id", Cow::from("real")),
                ("class", Cow::from("c")),
            ],
        );
        assert_eq!(
            attribute_list(r#"data-id="x" id="real" class="c""#).unwrap(),
            expected
        );
    }

    /// A name must not swallow the characters that end it, or the tag's
    /// `>` and the next attribute would be read as part of the name.
    #[test]
    fn test_attribute_name_stops_at_delimiters() {
        assert_eq!(attribute_name("id=\"x\"").unwrap(), ("=\"x\"", "id"));
        assert_eq!(attribute_name("bool>rest").unwrap(), (">rest", "bool"));
        assert_eq!(attribute_name("bool/>rest").unwrap(), ("/>rest", "bool"));
        assert_eq!(attribute_name("a'b").unwrap(), ("'b", "a"));
    }
}
