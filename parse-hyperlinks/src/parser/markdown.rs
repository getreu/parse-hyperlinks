//! This module implements parsers for Markdown hyperlinks.
#![allow(dead_code)]
#![allow(clippy::type_complexity)]

use crate::parser::parse::LABEL_LEN_MAX;
use crate::parser::Link;
use crate::take_until_unbalanced;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::multispace1;
use nom::combinator::*;
use std::borrow::Cow;

/// The following character are escapable in _link text_, _link label_, _link
/// destination_ and _link title_.
const ESCAPABLE: &str = r#"\'"()[]{}<>"#;

/// Wrapper around `md_text2dest()` that packs the result in
/// `Link::Text2Dest`.
pub fn md_text2dest_link(i: &str) -> nom::IResult<&str, Link> {
    let (i, (te, de, ti)) = md_text2dest(i)?;
    Ok((i, Link::Text2Dest(te, de, ti)))
}

/// Parses a Markdown _inline link_.
///
/// This parser expects to start at the beginning of the link `[` to succeed.
/// ```
/// use parse_hyperlinks::parser::Link;
/// use parse_hyperlinks::parser::markdown::md_text2dest;
/// use std::borrow::Cow;
///
/// assert_eq!(
///   md_text2dest(r#"[text](<destination> "title")abc"#),
///   Ok(("abc", (Cow::from("text"), Cow::from("destination"), Cow::from("title"))))
/// );
/// ```
pub fn md_text2dest(i: &str) -> nom::IResult<&str, (Cow<str>, Cow<str>, Cow<str>)> {
    let (i, link_text) = md_link_text(i)?;
    let (i, (link_destination, link_title)) = md_link_destination_enclosed(i)?;
    Ok((i, (link_text, link_destination, link_title)))
}

/// Wrapper around `md_label2dest()` that packs the result in
/// `Link::Label2Dest`.
pub fn md_label2dest_link(i: &str) -> nom::IResult<&str, Link> {
    let (i, (l, d, t)) = md_label2dest(i)?;
    Ok((i, Link::Label2Dest(l, d, t)))
}

/// Matches a Markdown _link reference definition_.
///
/// The caller must guarantee, that the parser starts at first character of the
/// input or at the first character of a line. The parser consumes all bytes
/// until the end of the line.
/// ```
/// use parse_hyperlinks::parser::Link;
/// use parse_hyperlinks::parser::markdown::md_label2dest;
/// use std::borrow::Cow;
///
/// assert_eq!(
///   md_label2dest("   [label]: <destination> 'title'\nabc"),
///   Ok(("\nabc", (Cow::from("label"), Cow::from("destination"), Cow::from("title"))))
/// );
/// ```
///
/// [CommonMark
/// Spec](https://spec.commonmark.org/0.30/#link-reference-definition)\ A [link
/// reference
/// definition](https://spec.commonmark.org/0.30/#link-reference-definition)
/// consists of a [link label](https://spec.commonmark.org/0.30/#link-label),
/// optionally preceded by up to three spaces of indentation, followed by a
/// colon (`:`), optional spaces or tabs (including up to one [line
/// ending](https://spec.commonmark.org/0.30/#line-ending)), a [link
/// destination](https://spec.commonmark.org/0.30/#link-destination), optional
/// spaces or tabs (including up to one [line
/// ending](https://spec.commonmark.org/0.30/#line-ending)), and an optional
/// [link title](https://spec.commonmark.org/0.30/#link-title), which if it is
/// present must be separated from the [link
/// destination](https://spec.commonmark.org/0.30/#link-destination) by spaces
/// or tabs. No further character may occur.
///
/// A [link reference
/// definition](https://spec.commonmark.org/0.30/#link-reference-definition)
/// does not correspond to a structural element of a document. Instead, it
/// defines a label which can be used in [reference
/// links](https://spec.commonmark.org/0.30/#reference-link) and reference-style
/// [images](https://spec.commonmark.org/0.30/#images) elsewhere in the
/// document. [Link reference
/// definitions](https://spec.commonmark.org/0.30/#link-reference-definition)
/// can come either before or after the links that use them.
pub fn md_label2dest(i: &str) -> nom::IResult<&str, (Cow<str>, Cow<str>, Cow<str>)> {
    // Consume up to three spaces.
    let (i, _) = nom::bytes::complete::take_while_m_n(0, 3, |c| c == ' ')(i)?;
    // Take label.
    let (i, link_text) = md_link_label(i)?;
    let (i, _) = nom::character::complete::char(':')(i)?;
    // Take spaces.
    let (i, _) = verify(nom::character::complete::multispace1, |s: &str| {
        !s.contains("\n\n")
    })(i)?;
    // Take destination.
    let (i, link_destination) = md_link_destination(i)?;
    // Try, but do not fail.
    let (i, link_title) = alt((
        // Take link title.
        md_link_title,
        nom::combinator::success(Cow::from("")),
    ))(i)?;

    // Now consume as much whitespace as possible.
    let (i, _) = nom::character::complete::space0(i)?;

    // Check if there is newline coming. Do not consume.
    if !i.is_empty() {
        let _ = nom::character::complete::newline(i)?;
    }

    Ok((i, (link_text, link_destination, link_title)))
}

/// Wrapper around `md_text2label()` that packs the result in
/// `Link::Text2Label`.
pub fn md_text2label_link(i: &str) -> nom::IResult<&str, Link> {
    let (i, (t, l)) = md_text2label(i)?;
    Ok((i, Link::Text2Label(t, l)))
}

/// Parse a Markdown _reference link_.
///
/// There are three kinds of reference links: full, collapsed, and shortcut.
/// 1. A full reference link consists of a link text immediately followed by a
///    link label that matches a link reference definition elsewhere in the
///    document.
/// 2. A collapsed reference link consists of a link label that matches a link
///    reference definition elsewhere in the document, followed by the string [].
///    The contents of the first link label are parsed as inlines, which are used as
///    the link’s text. The link’s URI and title are provided by the matching
///    reference link definition. Thus, `[foo][]` is equivalent to `[foo][foo]`.
/// 3. A shortcut reference link consists of a link label that matches a link
///    reference definition elsewhere in the document and is not followed by [] or a
///    link label. The contents of the first link label are parsed as inlines, which
///    are used as the link’s text. The link’s URI and title are provided by the
///    matching link reference definition. Thus, `[foo]` is equivalent to `[foo][]`.
///
/// This parser expects to start at the beginning of the link `[` to succeed.
/// It should always run at last position after all other parsers.
/// ```rust
/// use parse_hyperlinks::parser::Link;
/// use parse_hyperlinks::parser::markdown::md_text2label;
/// use std::borrow::Cow;
///
/// assert_eq!(
///   md_text2label("[link text][link label]abc"),
///   Ok(("abc", (Cow::from("link text"), Cow::from("link label"))))
/// );
/// assert_eq!(
///   md_text2label("[link text][]abc"),
///   Ok(("abc", (Cow::from("link text"), Cow::from("link text"))))
/// );
/// assert_eq!(
///   md_text2label("[link text]abc"),
///   Ok(("abc", (Cow::from("link text"), Cow::from("link text"))))
/// );
/// ```
pub fn md_text2label(i: &str) -> nom::IResult<&str, (Cow<str>, Cow<str>)> {
    let (i, (link_text, link_label)) = alt((
        nom::sequence::pair(md_link_text, md_link_label),
        nom::combinator::map(nom::sequence::terminated(md_link_text, tag("[]")), |s| {
            (s.clone(), s)
        }),
        nom::combinator::map(md_link_text, |s| (s.clone(), s)),
    ))(i)?;

    // Check that there is no `[` or `(` following. Do not consume.
    if !i.is_empty() {
        let _ = nom::character::complete::none_of("[(")(i)?;
    }

    Ok((i, (link_text, link_label)))
}

/// Parses _link text_.
/// Brackets are allowed in the
/// [link text](https://spec.commonmark.org/0.29/#link-text) only if (a) they are
/// backslash-escaped or (b) they appear as a matched pair of brackets, with
/// an open bracket `[`, a sequence of zero or more inlines, and a close
/// bracket `]`.
/// [CommonMark Spec](https://spec.commonmark.org/0.29/#link-text)
fn md_link_text(i: &str) -> nom::IResult<&str, Cow<str>> {
    nom::combinator::map_parser(
        nom::sequence::delimited(tag("["), take_until_unbalanced('[', ']'), tag("]")),
        md_escaped_str_transform,
    )(i)
}

/// Parses _link label_.
/// A link label begins with a left bracket ([) and ends with the first right
/// bracket (]) that is not backslash-escaped. Between these brackets there must
/// be at least one non-whitespace character. Unescaped square bracket characters
/// are not allowed inside the opening and closing square brackets of link
/// labels. A link label can have at most 999 characters inside the square
/// brackets (TODO).
/// [CommonMark Spec](https://spec.commonmark.org/0.29/#link-label)
fn md_link_label(i: &str) -> nom::IResult<&str, Cow<str>> {
    nom::combinator::map_parser(
        nom::combinator::verify(
            nom::sequence::delimited(
                tag("["),
                nom::bytes::complete::escaped(
                    nom::character::complete::none_of(r#"\[]"#),
                    '\\',
                    nom::character::complete::one_of(ESCAPABLE),
                ),
                tag("]"),
            ),
            |l: &str| l.len() <= LABEL_LEN_MAX,
        ),
        md_escaped_str_transform,
    )(i)
}

/// This is a wrapper around `md_parse_link_destination()`. It takes its result
/// and removes the `\` before the escaped characters `ESCAPABLE`.
fn md_link_destination(i: &str) -> nom::IResult<&str, Cow<str>> {
    nom::combinator::map_parser(md_parse_link_destination, md_escaped_str_transform)(i)
}

/// A [link destination](https://spec.commonmark.org/0.30/#link-destination)
/// consists of either
///
/// * a sequence of zero or more characters between an opening `<` and a
/// closing `>` that contains no line endings or unescaped `<` or `>`
/// characters, or
/// * a nonempty sequence of characters that does not start with `<`, does not
/// include [ASCII control
/// characters](https://spec.commonmark.org/0.30/#ascii-control-character) or
/// [space](https://spec.commonmark.org/0.30/#space) character, and includes
/// parentheses only if (a) they are backslash-escaped or (b) they are part of a
/// balanced pair of unescaped parentheses. (Implementations may impose limits
/// on parentheses nesting to avoid performance issues, but at least three
/// levels of nesting should be supported.)
fn md_parse_link_destination(i: &str) -> nom::IResult<&str, &str> {
    alt((
        nom::sequence::delimited(
            tag("<"),
            nom::bytes::complete::escaped(
                nom::character::complete::none_of(r#"\<>"#),
                '\\',
                nom::character::complete::one_of(ESCAPABLE),
            ),
            tag(">"),
        ),
        map(nom::bytes::complete::tag("<>"), |_| ""),
        alt((
            nom::bytes::complete::is_not(" \t\r\n"),
            nom::combinator::success(""),
        )),
    ))(i)
}

/// Matches `md_link_destination` in parenthesis.
fn md_link_destination_enclosed(i: &str) -> nom::IResult<&str, (Cow<str>, Cow<str>)> {
    let (rest, inner) =
        nom::sequence::delimited(tag("("), take_until_unbalanced('(', ')'), tag(")"))(i)?;
    let (i, link_destination) = md_link_destination(inner)?;
    let (_i, link_title) = alt((
        // Take link title.
        md_link_title,
        nom::combinator::success(Cow::from("")),
    ))(i)?;

    Ok((rest, (link_destination, link_title)))
}

/// This is a wrapper around `md_parse_link_title()`. It takes its result
/// and removes the `\` before the escaped characters `ESCAPABLE`.
fn md_link_title(i: &str) -> nom::IResult<&str, Cow<str>> {
    nom::combinator::map_parser(md_parse_link_title, md_escaped_str_transform)(i)
}

/// A link title is always preceded one or more whitespace inluding
/// one newline.
/// [CommonMark Spec](https://spec.commonmark.org/0.29/#link-title)
/// A [link title](https://spec.commonmark.org/0.29/#link-title) consists of either
///
///  - a sequence of zero or more characters between straight double-quote
///    characters (`"`), including a `"` character only if it is
///    backslash-escaped, or
///  - a sequence of zero or more characters between straight single-quote
///    characters (`'`), including a `'` character only if it is
///    backslash-escaped, or
///  - a sequence of zero or more characters between matching parentheses
///    (`(...)`), including a `(` or `)` character only if it is
///    backslash-escaped.
///
///  Although [link titles](https://spec.commonmark.org/0.29/#link-title) may
///  span multiple lines, they may not contain a [blank
///  line](https://spec.commonmark.org/0.29/#blank-line).
fn md_parse_link_title(i: &str) -> nom::IResult<&str, &str> {
    nom::sequence::preceded(
        verify(multispace1, |s: &str| !s.contains("\n\n")),
        verify(
            alt((
                nom::sequence::delimited(tag("("), take_until_unbalanced('(', ')'), tag(")")),
                nom::sequence::delimited(
                    tag("'"),
                    nom::bytes::complete::escaped(
                        nom::character::complete::none_of(r#"\'"#),
                        '\\',
                        nom::character::complete::one_of(ESCAPABLE),
                    ),
                    tag("'"),
                ),
                nom::sequence::delimited(
                    tag("\""),
                    nom::bytes::complete::escaped(
                        nom::character::complete::none_of(r#"\""#),
                        '\\',
                        nom::character::complete::one_of(ESCAPABLE),
                    ),
                    tag("\""),
                ),
            )),
            |s: &str| !s.contains("\n\n"),
        ),
    )(i)
}

/// Remove the `\` before the escaped characters `ESCAPABLE`.
fn md_escaped_str_transform(i: &str) -> nom::IResult<&str, Cow<str>> {
    nom::combinator::map(
        nom::bytes::complete::escaped_transform(
            nom::bytes::complete::is_not("\\"),
            '\\',
            nom::character::complete::one_of(ESCAPABLE),
        ),
        |s| if s == i { Cow::from(i) } else { Cow::from(s) },
    )(i)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom::error::ErrorKind;

    #[test]
    fn test_md_text2dest() {
        assert_eq!(
            md_text2dest("[text](url)abc"),
            Ok(("abc", (Cow::from("text"), Cow::from("url"), Cow::from(""))))
        );
        assert_eq!(
            md_text2dest("[text[i]](url)abc"),
            Ok((
                "abc",
                (Cow::from("text[i]"), Cow::from("url"), Cow::from(""))
            ))
        );
        assert_eq!(
            md_text2dest("[text[i]](ur(l))abc"),
            Ok((
                "abc",
                (Cow::from("text[i]"), Cow::from("ur(l)"), Cow::from(""))
            ))
        );
        assert_eq!(
            md_text2dest("[text(url)"),
            Err(nom::Err::Error(nom::error::Error::new("", ErrorKind::Tag)))
        );
        assert_eq!(
            md_text2dest("[text](<url>)abc"),
            Ok(("abc", (Cow::from("text"), Cow::from("url"), Cow::from(""))))
        );
        assert_eq!(
            md_text2dest("[text](<url> \"link title\")abc"),
            Ok((
                "abc",
                (Cow::from("text"), Cow::from("url"), Cow::from("link title"))
            ))
        );
        assert_eq!(
            md_text2dest("[text](url \"link title\")abc"),
            Ok((
                "abc",
                (Cow::from("text"), Cow::from("url"), Cow::from("link title"))
            ))
        );
        // [Example 483](https://spec.commonmark.org/0.30/#example-483)
        assert_eq!(
            md_text2dest("[](./target.md)abc"),
            Ok((
                "abc",
                (Cow::from(""), Cow::from("./target.md"), Cow::from(""))
            ))
        );
        // [Example 484](https://spec.commonmark.org/0.30/#example-484)
        assert_eq!(
            md_text2dest("[link]()abc"),
            Ok(("abc", (Cow::from("link"), Cow::from(""), Cow::from(""))))
        );
        // [Example 485](https://spec.commonmark.org/0.30/#example-485)
        assert_eq!(
            md_text2dest("[link](<>)abc"),
            Ok(("abc", (Cow::from("link"), Cow::from(""), Cow::from(""))))
        );
        // [Example 486](https://spec.commonmark.org/0.30/#example-486)
        assert_eq!(
            md_text2dest("[]()abc"),
            Ok(("abc", (Cow::from(""), Cow::from(""), Cow::from(""))))
        );
    }

    #[test]
    fn test_md_text2label() {
        assert_eq!(
            md_text2label("[link text][link label]abc"),
            Ok(("abc", (Cow::from("link text"), Cow::from("link label"))))
        );
        assert_eq!(
            md_text2label("[link text][]abc"),
            Ok(("abc", (Cow::from("link text"), Cow::from("link text"))))
        );
        assert_eq!(
            md_text2label("[link text]abc"),
            Ok(("abc", (Cow::from("link text"), Cow::from("link text"))))
        );
        assert_eq!(
            md_text2label("[]abc"),
            Ok(("abc", (Cow::from(""), Cow::from(""))))
        );
        assert_eq!(
            md_text2label(""),
            Err(nom::Err::Error(nom::error::Error::new("", ErrorKind::Tag)))
        );
        // Check end of input position.
        assert_eq!(
            md_text2label("[text]"),
            Ok(("", (Cow::from("text"), Cow::from("text"))))
        );
        assert_eq!(
            md_text2label("[text][text]"),
            Ok(("", (Cow::from("text"), Cow::from("text"))))
        );
        assert_eq!(
            md_text2label("[text][label url"),
            Err(nom::Err::Error(nom::error::Error::new(
                "[label url",
                ErrorKind::NoneOf
            )))
        );
        assert_eq!(
            md_text2label("[text](url)abc"),
            Err(nom::Err::Error(nom::error::Error::new(
                "(url)abc",
                ErrorKind::NoneOf
            )))
        );
    }

    #[test]
    fn test_md_label2dest() {
        assert_eq!(
            md_label2dest("[text]: url\nabc"),
            Ok((
                "\nabc",
                (Cow::from("text"), Cow::from("url"), Cow::from(""))
            ))
        );
        assert_eq!(
            md_label2dest("[text]: url  \nabc"),
            Ok((
                "\nabc",
                (Cow::from("text"), Cow::from("url"), Cow::from(""))
            ))
        );
        assert_eq!(
            md_label2dest("[text]: <url url> \nabc"),
            Ok((
                "\nabc",
                (Cow::from("text"), Cow::from("url url"), Cow::from(""))
            ))
        );
        assert_eq!(
            md_label2dest("[text]: url \"title\"\nabc"),
            Ok((
                "\nabc",
                (Cow::from("text"), Cow::from("url"), Cow::from("title"))
            ))
        );
        assert_eq!(
            md_label2dest("[text]: url\n\"title\"\nabc"),
            Ok((
                "\nabc",
                (Cow::from("text"), Cow::from("url"), Cow::from("title"))
            ))
        );
        assert_eq!(
            md_label2dest("   [text]: url\n\"title\"\nabc"),
            Ok((
                "\nabc",
                (Cow::from("text"), Cow::from("url"), Cow::from("title"))
            ))
        );
        assert_eq!(
            md_label2dest("abc[text]: url\n\"title\""),
            Err(nom::Err::Error(nom::error::Error::new(
                "abc[text]: url\n\"title\"",
                ErrorKind::Tag
            )))
        );
        assert_eq!(
            md_label2dest("    [text]: url\n\"title\" abc"),
            Err(nom::Err::Error(nom::error::Error::new(
                " [text]: url\n\"title\" abc",
                ErrorKind::Tag
            )))
        );
        // Nested brackets.
        assert_eq!(
            md_label2dest("[text\\[i\\]]: ur(l)url\nabc"),
            Ok((
                "\nabc",
                (Cow::from("text[i]"), Cow::from("ur(l)url"), Cow::from(""))
            ))
        );
        // Nested but balanced not allowed for link labels.
        assert_eq!(
            md_label2dest("[text[i]]: ur(l)(url"),
            Err(nom::Err::Error(nom::error::Error::new(
                "[i]]: ur(l)(url",
                ErrorKind::Tag
            )))
        );
        // Whitespace can have one newline.
        assert_eq!(
            md_label2dest("[text]: \nurl"),
            Ok(("", (Cow::from("text"), Cow::from("url"), Cow::from(""))))
        );
        // But only one newline is allowed.
        assert_eq!(
            md_label2dest("[text]: \n\nurl"),
            Err(nom::Err::Error(nom::error::Error::new(
                " \n\nurl",
                ErrorKind::Verify
            )))
        );
        assert_eq!(
            md_label2dest("[text: url"),
            Err(nom::Err::Error(nom::error::Error::new("", ErrorKind::Tag)))
        );
        assert_eq!(
            md_label2dest("[text] url"),
            Err(nom::Err::Error(nom::error::Error::new(
                " url",
                ErrorKind::Char
            )))
        );
        assert_eq!(
            md_label2dest("[text]: url \"link title\"\nabc"),
            Ok((
                "\nabc",
                (Cow::from("text"), Cow::from("url"), Cow::from("link title"))
            ))
        );
        assert_eq!(
            md_label2dest("[text]: url \"link\ntitle\"\nabc"),
            Ok((
                "\nabc",
                (
                    Cow::from("text"),
                    Cow::from("url"),
                    Cow::from("link\ntitle")
                )
            ))
        );
        assert_eq!(
            md_label2dest("[text]: url \"link\ntitle\"abc"),
            Err(nom::Err::Error(nom::error::Error::new(
                "abc",
                ErrorKind::Char
            )))
        );
        assert_eq!(
            md_label2dest("[text]:\nurl \"link\ntitle\"\nabc"),
            Ok((
                "\nabc",
                (
                    Cow::from("text"),
                    Cow::from("url"),
                    Cow::from("link\ntitle")
                )
            ))
        );
        assert_eq!(
            md_label2dest("[text]: url \"link\n\ntitle\"\nabc"),
            Err(nom::Err::Error(nom::error::Error::new(
                "\"link\n\ntitle\"\nabc",
                ErrorKind::Char
            )))
        );
        assert_eq!(
            md_label2dest("[text]:\n\nurl \"link title\"\nabc"),
            Err(nom::Err::Error(nom::error::Error::new(
                "\n\nurl \"link title\"\nabc",
                ErrorKind::Verify
            )))
        );
    }

    #[test]
    fn test_md_link_text() {
        assert_eq!(
            md_link_text("[text](url)"),
            Ok(("(url)", Cow::from("text")))
        );
        assert_eq!(
            md_link_text("[text[i]](url)"),
            Ok(("(url)", Cow::from("text[i]")))
        );
        assert_eq!(
            md_link_text(r#"[text\[i\]](url)"#),
            Ok(("(url)", Cow::from("text[i]")))
        );
        assert_eq!(
            md_link_text("[text(url)"),
            Err(nom::Err::Error(nom::error::Error::new("", ErrorKind::Tag)))
        );
    }

    #[test]
    fn test_md_link_label() {
        assert_eq!(
            md_link_label("[text]: url"),
            Ok((": url", Cow::from("text")))
        );
        assert_eq!(
            md_link_label(r#"[text\[i\]]: url"#),
            Ok((": url", Cow::from("text[i]")))
        );
        assert_eq!(
            md_link_label("[text: url"),
            Err(nom::Err::Error(nom::error::Error::new("", ErrorKind::Tag)))
        );
        assert_eq!(
            md_link_label("[t[ext: url"),
            Err(nom::Err::Error(nom::error::Error::new(
                "[ext: url",
                ErrorKind::Tag
            )))
        );
    }

    #[test]
    fn test_md_link_destination() {
        assert_eq!(
            md_link_destination("url  abc"),
            Ok(("  abc", Cow::from("url")))
        );
        assert_eq!(md_link_destination("url"), Ok(("", Cow::from("url"))));
        assert_eq!(
            md_link_destination("url\nabc"),
            Ok(("\nabc", Cow::from("url")))
        );
        assert_eq!(
            md_link_destination("<url>abc"),
            Ok(("abc", Cow::from("url")))
        );
        assert_eq!(
            md_link_destination(r#"<u\<r\>l>abc"#),
            Ok(("abc", Cow::from(r#"u<r>l"#)))
        );
        assert_eq!(
            md_link_destination(r#"u\)r\(l abc"#),
            Ok((" abc", Cow::from(r#"u)r(l"#)))
        );
        assert_eq!(
            md_link_destination(r#"u(r)l abc"#),
            Ok((" abc", Cow::from(r#"u(r)l"#)))
        );
        assert_eq!(
            md_link_destination("u(r)l\nabc"),
            Ok(("\nabc", Cow::from(r#"u(r)l"#)))
        );
    }

    #[test]
    fn test_md_parse_link_destination() {
        assert_eq!(md_parse_link_destination("<url>abc"), Ok(("abc", "url")));
        assert_eq!(
            md_parse_link_destination(r#"<u\<r\>l>abc"#),
            Ok(("abc", r#"u\<r\>l"#))
        );
        assert_eq!(md_parse_link_destination("<url> abc"), Ok((" abc", "url")));
        assert_eq!(
            md_parse_link_destination("<url>\nabc"),
            Ok(("\nabc", "url"))
        );
        assert_eq!(
            md_parse_link_destination("<url 2>abc"),
            Ok(("abc", "url 2"))
        );
        assert_eq!(md_parse_link_destination("url abc"), Ok((" abc", "url")));
        assert_eq!(
            md_parse_link_destination("<url(1)> abc"),
            Ok((" abc", "url(1)"))
        );
        assert_eq!(
            md_parse_link_destination(r#"<[1a]\[1b\](2a)\(2b\)\<3b\>{4a}\{4b\}> abc"#),
            Ok((" abc", r#"[1a]\[1b\](2a)\(2b\)\<3b\>{4a}\{4b\}"#))
        );
        assert_eq!(
            md_parse_link_destination("ur()l abc"),
            Ok((" abc", "ur()l"))
        );
        assert_eq!(
            md_parse_link_destination("ur()l\nabc"),
            Ok(("\nabc", "ur()l"))
        );
        assert_eq!(md_parse_link_destination("<>abc"), Ok(("abc", "")));
        assert_eq!(md_parse_link_destination("<>\nabc"), Ok(("\nabc", "")));
        assert_eq!(md_parse_link_destination("url"), Ok(("", "url")));
        assert_eq!(md_parse_link_destination(""), Ok(("", "")));
        assert_eq!(md_parse_link_destination("\nabc"), Ok(("\nabc", "")));
    }

    #[test]
    fn test_md_escaped_str_transform() {
        assert_eq!(md_escaped_str_transform(""), Ok(("", Cow::from(""))));
        // Different than the link destination version.
        assert_eq!(md_escaped_str_transform("   "), Ok(("", Cow::from("   "))));
        assert_eq!(
            md_escaped_str_transform(r#"abc`:<>abc"#),
            Ok(("", Cow::from(r#"abc`:<>abc"#)))
        );
        assert_eq!(
            md_escaped_str_transform(r#"\<\>\\"#),
            Ok(("", Cow::from(r#"<>\"#)))
        );
        assert_eq!(
            md_escaped_str_transform(r#"\(\)\\"#),
            Ok(("", Cow::from(r#"()\"#)))
        );
    }

    #[test]
    fn test_md_link_title() {
        assert_eq!(
            md_link_title(" (title)abc"),
            Ok(("abc", Cow::from("title")))
        );
        assert_eq!(
            md_link_title(" (ti(t)le)abc"),
            Ok(("abc", Cow::from("ti(t)le")))
        );
        assert_eq!(
            md_link_title(r#" (ti\(t\)le)abc"#),
            Ok(("abc", Cow::from("ti(t)le")))
        );
        assert_eq!(
            md_link_title(r#" "1\\23\"4\'56"abc"#),
            Ok(("abc", Cow::from(r#"1\23"4'56"#)))
        );
        assert_eq!(
            md_link_title(" \"tu\nvwxy\"abc"),
            Ok(("abc", Cow::from("tu\nvwxy")))
        );
        assert_eq!(
            md_link_title(" 'tu\nv\\\'wxy'abc"),
            Ok(("abc", Cow::from("tu\nv\'wxy")))
        );
        assert_eq!(
            md_link_title(" (ti\n\ntle)abc"),
            Err(nom::Err::Error(nom::error::Error::new(
                "(ti\n\ntle)abc",
                ErrorKind::Verify
            )))
        );
    }

    #[test]
    fn test_md_parse_link_title() {
        assert_eq!(md_parse_link_title(" (title)abc"), Ok(("abc", "title")));
        assert_eq!(md_parse_link_title(" (ti(t)le)abc"), Ok(("abc", "ti(t)le")));
        assert_eq!(
            md_parse_link_title(r#" "1\\23\"4\'56"abc"#),
            Ok(("abc", r#"1\\23\"4\'56"#))
        );
        assert_eq!(
            md_parse_link_title(" \"tu\nvwxy\"abc"),
            Ok(("abc", "tu\nvwxy"))
        );
        assert_eq!(
            md_parse_link_title(" 'tu\nv\\\'wxy'abc"),
            Ok(("abc", "tu\nv\\\'wxy"))
        );
        assert_eq!(
            md_parse_link_title(" (ti\n\ntle)abc"),
            Err(nom::Err::Error(nom::error::Error::new(
                "(ti\n\ntle)abc",
                ErrorKind::Verify
            )))
        );
    }
}
