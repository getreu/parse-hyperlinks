//! This module implements parsers for Markdown hyperlinks.
#![allow(dead_code)]

use crate::take_until_unbalanced;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::*;
use nom::error::ErrorKind;
use std::borrow::Cow;

/// Parse a markdown link.
/// returns either `Ok((i, (link_text, link_destination, link_title)))` or some
/// error.
///
/// This parser expects to start at the beginning of the link `[` to succeed. It
/// ```
/// use parse_hyperlinks::parser::markdown::md_link;
/// use std::borrow::Cow;
///
/// assert_eq!(
///   md_link(r#"[name](<destination> "title")abc"#),
///   Ok(("abc", (Cow::from("name"), Cow::from("destination"), Cow::from("title"))))
/// );
/// ```
pub fn md_link(i: &str) -> nom::IResult<&str, (Cow<str>, Cow<str>, Cow<str>)> {
    let (i, link_text) = md_link_text(i)?;
    let (i, (link_destination, link_title)) = md_link_destination_enclosed(i)?;
    Ok((
        i,
        (
            Cow::Borrowed(link_text),
            link_destination,
            Cow::Borrowed(link_title),
        ),
    ))
}

/// Matches a Markdown link reference definition.
/// It returns either `Ok((i, (link_label, link_destination, link_title)))` or
/// some error.
///
/// The caller must guarantee, that the parser starts at first character of the
/// input or at the first character of a line.
/// ```
/// use parse_hyperlinks::parser::markdown::md_link_ref;
/// use std::borrow::Cow;
///
/// assert_eq!(
///   md_link_ref("   [label]: <destination> 'title'\nabc"),
///   Ok(("\nabc", (Cow::from("label"), Cow::from("destination"), Cow::from("title"))))
/// );
/// ```
pub fn md_link_ref(i: &str) -> nom::IResult<&str, (Cow<str>, Cow<str>, Cow<str>)> {
    // Consume up to three spaces.
    let (i, _) = nom::bytes::complete::take_while_m_n(0, 3, |c| c == ' ')(i)?;
    let (i, link_text) = md_link_ref_text(i)?;
    let (i, _) = verify(nom::character::complete::multispace1, |s: &str| {
        s.find("\n\n").is_none()
    })(i)?;
    let (i, link_destination) = md_link_destination(i)?;
    if let Ok((i, _)) = verify(
        nom::character::complete::multispace1::<_, (_, ErrorKind)>,
        |s: &str| s.find("\n\n").is_none(),
    )(i)
    {
        let (i, link_title) = verify(md_link_title, |s: &str| s.find("\n\n").is_none())(i)?;
        Ok((
            i,
            (
                Cow::Borrowed(link_text),
                link_destination,
                Cow::Borrowed(link_title),
            ),
        ))
    } else {
        Ok((
            i,
            (
                Cow::Borrowed(link_text),
                link_destination,
                Cow::Borrowed(""),
            ),
        ))
    }
}

/// [CommonMark Spec](https://spec.commonmark.org/0.29/#link-text)
///
/// Brackets are allowed in the
/// [link text](https://spec.commonmark.org/0.29/#link-text) only if (a) they are
/// backslash-escaped or (b) they appear as a matched pair of brackets, with
/// an open bracket `[`, a sequence of zero or more inlines, and a close
/// bracket `]`.
fn md_link_text(i: &str) -> nom::IResult<&str, &str> {
    nom::sequence::delimited(tag("["), take_until_unbalanced('[', ']'), tag("]"))(i)
}

/// CommonMark Spec: A [link reference definition] consists of a [link
/// label], indented up to three spaces, followed by a colon (`:`), optional
/// [whitespace] (including up to one [line ending]), a [link destination],
/// optional [whitespace] (including up to one [line ending]), and an
/// optional [link title], which if it is present must be separated from the
/// [link destination] by [whitespace]. No further [non-whitespace
/// characters] may occur on the line.
///
/// [link reference definition]: https://spec.commonmark.org/0.29/#link-reference-definition
/// [link label]: https://spec.commonmark.org/0.29/#link-label
/// [whitespace]: https://spec.commonmark.org/0.29/#whitespace
/// [line ending]: https://spec.commonmark.org/0.29/#line-ending
/// [link destination]: https://spec.commonmark.org/0.29/#link-destination
/// [whitespace]: https://spec.commonmark.org/0.29/#whitespace
/// [line ending]: https://spec.commonmark.org/0.29/#line-ending
/// [link title]: https://spec.commonmark.org/0.29/#link-title
/// [link destination]: https://spec.commonmark.org/0.29/#link-destination
/// [whitespace]: https://spec.commonmark.org/0.29/#whitespace
/// [non-whitespace characters]: https://spec.commonmark.org/0.29/#non-whitespace-character
fn md_link_ref_text(i: &str) -> nom::IResult<&str, &str> {
    nom::sequence::delimited(tag("["), take_until_unbalanced('[', ']'), tag("]:"))(i)
}

/// This is a wrapper around `md_parse_link_destination()`. It takes its result
/// and transforms the escaped characters `\\`, \<` and `\>` into `\`, `<` and
/// `>` with the help of `md_escaped_link_destination_transform()`.
fn md_link_destination(i: &str) -> nom::IResult<&str, Cow<str>> {
    nom::combinator::map_parser(
        md_parse_link_destination,
        md_escaped_link_destination_transform,
    )(i)
}

/// A [link destination](https://spec.commonmark.org/0.29/#link-destination)
/// consists of either
/// - a sequence of zero or more characters between an opening `<` and a
///   closing `>` that contains no line breaks or unescaped `<` or `>`
///   characters, or
/// - a nonempty sequence of characters that does not start with `<`, does not
///   include ASCII space or control characters, and includes parentheses only if
///   (a) they are backslash-escaped or (b) they are part of a balanced pair of
///   unescaped parentheses. (Implementations may impose limits on parentheses
///   nesting to avoid performance issues, but at least three levels of nesting
///   should be supported.)
fn md_parse_link_destination(i: &str) -> nom::IResult<&str, &str> {
    alt((
        nom::sequence::delimited(
            tag("<"),
            nom::bytes::complete::escaped(
                nom::character::complete::none_of(r#"\<>"#),
                '\\',
                nom::character::complete::one_of(r#"<>"#),
            ),
            tag(">"),
        ),
        map_parser(
            nom::bytes::complete::is_not(" \t\r\n"),
            all_consuming(take_until_unbalanced('(', ')')),
        ),
    ))(i)
}

/// Replace the following escaped characters:
///     \\\<\>
/// with:
///     \<>
/// Preserves usual whitespace, but removes `\ `.
fn md_escaped_link_destination_transform(i: &str) -> nom::IResult<&str, Cow<str>> {
    nom::combinator::map(
        nom::bytes::complete::escaped_transform(
            nom::bytes::complete::is_not("\\"),
            '\\',
            alt((
                value("\\", tag("\\")),
                value("<", tag("<")),
                value(">", tag(">")),
            )),
        ),
        |s| if s == i { Cow::from(i) } else { Cow::from(s) },
    )(i)
}

/// Matches `md_link_destination` in parenthesis.
fn md_link_destination_enclosed(i: &str) -> nom::IResult<&str, (Cow<str>, &str)> {
    let (rest, inner) =
        nom::sequence::delimited(tag("("), take_until_unbalanced('(', ')'), tag(")"))(i)?;
    let (i, link_destination) = md_link_destination(inner)?;
    if let Ok((i, _)) = nom::character::complete::multispace1::<_, (_, ErrorKind)>(i) {
        let (_, link_title) = md_link_title(i)?;
        Ok((rest, (link_destination, link_title)))
    } else {
        Ok((rest, (link_destination, "")))
    }
}

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
fn md_link_title(i: &str) -> nom::IResult<&str, &str> {
    verify(
        alt((
            nom::sequence::delimited(tag("("), take_until_unbalanced('(', ')'), tag(")")),
            nom::sequence::delimited(
                tag("'"),
                nom::bytes::complete::escaped(
                    nom::character::complete::none_of(r#"\'"#),
                    '\\',
                    nom::character::complete::one_of(r#"n"'()"#),
                ),
                tag("'"),
            ),
            nom::sequence::delimited(
                tag("\""),
                nom::bytes::complete::escaped(
                    nom::character::complete::none_of(r#"\""#),
                    '\\',
                    nom::character::complete::one_of(r#"n"'()"#),
                ),
                tag("\""),
            ),
        )),
        |s: &str| s.find("\n\n").is_none(),
    )(i)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom::error::ErrorKind;

    #[test]
    fn test_md_link() {
        assert_eq!(
            md_link("[text](url)abc"),
            Ok(("abc", (Cow::from("text"), Cow::from("url"), Cow::from(""))))
        );
        assert_eq!(
            md_link("[text[i]](url)abc"),
            Ok((
                "abc",
                (Cow::from("text[i]"), Cow::from("url"), Cow::from(""))
            ))
        );
        assert_eq!(
            md_link("[text[i]](ur(l))abc"),
            Ok((
                "abc",
                (Cow::from("text[i]"), Cow::from("ur(l)"), Cow::from(""))
            ))
        );
        assert_eq!(
            md_link("[text(url)"),
            Err(nom::Err::Error(nom::error::Error::new("", ErrorKind::Tag)))
        );
        assert_eq!(
            md_link("[text](<url>)abc"),
            Ok(("abc", (Cow::from("text"), Cow::from("url"), Cow::from(""))))
        );
        assert_eq!(
            md_link("[text](<url> \"link title\")abc"),
            Ok((
                "abc",
                (Cow::from("text"), Cow::from("url"), Cow::from("link title"))
            ))
        );
        assert_eq!(
            md_link("[text](url \"link title\")abc"),
            Ok((
                "abc",
                (Cow::from("text"), Cow::from("url"), Cow::from("link title"))
            ))
        );
    }

    #[test]
    fn test_md_link_ref() {
        assert_eq!(
            md_link_ref("[text]: url\n\"abc\""),
            Ok(("", (Cow::from("text"), Cow::from("url"), Cow::from("abc"))))
        );
        assert_eq!(
            md_link_ref("   [text]: url\n\"abc\""),
            Ok(("", (Cow::from("text"), Cow::from("url"), Cow::from("abc"))))
        );
        assert_eq!(
            md_link_ref("abc[text]: url\n\"abc\""),
            Err(nom::Err::Error(nom::error::Error::new(
                "abc[text]: url\n\"abc\"",
                ErrorKind::Tag
            )))
        );
        assert_eq!(
            md_link_ref("    [text]: url\n\"abc\""),
            Err(nom::Err::Error(nom::error::Error::new(
                " [text]: url\n\"abc\"",
                ErrorKind::Tag
            )))
        );
        // Nested brackets.
        assert_eq!(
            md_link_ref("[text[i]]: ur(l)url"),
            Ok((
                "",
                (Cow::from("text[i]"), Cow::from("ur(l)url"), Cow::from(""))
            ))
        );
        // Nested but balanced.
        assert_eq!(
            md_link_ref("[text[i]]: ur(l)(url"),
            Err(nom::Err::Error(nom::error::Error::new(
                "ur(l)(url",
                ErrorKind::TakeUntil
            )))
        );
        // Whitespace can have one newline.
        assert_eq!(
            md_link_ref("[text]: \nurl"),
            Ok(("", (Cow::from("text"), Cow::from("url"), Cow::from(""))))
        );
        // But only one newline is allowed.
        assert_eq!(
            md_link_ref("[text]: \n\nurl"),
            Err(nom::Err::Error(nom::error::Error::new(
                " \n\nurl",
                ErrorKind::Verify
            )))
        );
        assert_eq!(
            md_link_ref("[text: url"),
            Err(nom::Err::Error(nom::error::Error::new("", ErrorKind::Tag)))
        );
        assert_eq!(
            md_link_ref("[text] url"),
            Err(nom::Err::Error(nom::error::Error::new(
                "] url",
                ErrorKind::Tag
            )))
        );
        assert_eq!(
            md_link_ref("[text]: url \"link title\"\nabc"),
            Ok((
                "\nabc",
                (Cow::from("text"), Cow::from("url"), Cow::from("link title"))
            ))
        );
        assert_eq!(
            md_link_ref("[text]: url \"link\ntitle\"\nabc"),
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
            md_link_ref("[text]: url \"link\n\ntitle\"\nabc"),
            Err(nom::Err::Error(nom::error::Error::new(
                "\"link\n\ntitle\"\nabc",
                ErrorKind::Verify
            )))
        );
        assert_eq!(
            md_link_ref("[text]:\nurl \"link\ntitle\"\nabc"),
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
            md_link_ref("[text]:\n\nurl \"link title\"\nabc"),
            Err(nom::Err::Error(nom::error::Error::new(
                "\n\nurl \"link title\"\nabc",
                ErrorKind::Verify
            )))
        );
    }

    #[test]
    fn test_md_link_text() {
        assert_eq!(md_link_text("[text](url)"), Ok(("(url)", "text")));
        assert_eq!(md_link_text("[text[i]](url)"), Ok(("(url)", "text[i]")));
        assert_eq!(
            md_link_text("[text(url)"),
            Err(nom::Err::Error(nom::error::Error::new("", ErrorKind::Tag)))
        );
    }

    #[test]
    fn test_md_link_ref_text() {
        assert_eq!(md_link_ref_text("[text]: url"), Ok((" url", "text")));
        assert_eq!(md_link_ref_text("[text[i]]: url"), Ok((" url", "text[i]")));
        assert_eq!(
            md_link_ref_text("[text: url"),
            Err(nom::Err::Error(nom::error::Error::new("", ErrorKind::Tag)))
        );
        assert_eq!(
            md_link_ref_text("[t[ext: url"),
            Err(nom::Err::Error(nom::error::Error::new(
                "t[ext: url",
                ErrorKind::TakeUntil
            )))
        );
    }

    #[test]
    fn test_md_link_destination() {
        assert_eq!(
            md_link_destination("<url>abc"),
            Ok(("abc", Cow::from("url")))
        );
        assert_eq!(
            md_link_destination(r#"<u\<r\>l>abc"#),
            Ok(("abc", Cow::from(r#"u<r>l"#)))
        );
    }

    #[test]
    fn test_md_parse_link_destination() {
        assert_eq!(md_parse_link_destination("<url>abc"), Ok(("abc", "url")));
        assert_eq!(
            md_parse_link_destination(r#"<u\<r\>l>abc"#),
            Ok(("abc", r#"u\<r\>l"#))
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
            md_parse_link_destination("ur()l abc"),
            Ok((" abc", "ur()l"))
        );
        assert_eq!(
            md_link_title("<url>>abc"),
            Err(nom::Err::Error(nom::error::Error::new(
                "<url>>abc",
                ErrorKind::Tag
            )))
        );
        assert_eq!(
            md_link_title("<u<rl>abc"),
            Err(nom::Err::Error(nom::error::Error::new(
                "<u<rl>abc",
                ErrorKind::Tag
            )))
        );
    }

    #[test]
    fn test_md_escaped_link_destination_transform() {
        assert_eq!(
            md_escaped_link_destination_transform(""),
            Ok(("", Cow::from("")))
        );
        // Different than the link destination version.
        assert_eq!(
            md_escaped_link_destination_transform("   "),
            Ok(("", Cow::from("   ")))
        );
        assert_eq!(
            md_escaped_link_destination_transform(r#"abc`:<>abc"#),
            Ok(("", Cow::from(r#"abc`:<>abc"#)))
        );
        assert_eq!(
            md_escaped_link_destination_transform(r#"\<\>\\"#),
            Ok(("", Cow::from(r#"<>\"#)))
        );
    }

    #[test]
    fn test_md_link_title() {
        assert_eq!(md_link_title("(title)abc"), Ok(("abc", "title")));
        assert_eq!(md_link_title("(ti(t)le)abc"), Ok(("abc", "ti(t)le")));
        assert_eq!(
            md_link_title(r#""123\"456"abc"#),
            Ok(("abc", r#"123\"456"#))
        );
        assert_eq!(
            md_link_title(r#""tu\nv\"wxy"abc"#),
            Ok(("abc", r#"tu\nv\"wxy"#))
        );
        assert_eq!(
            md_link_title(r#"'tu\nv\'wxy'abc"#),
            Ok(("abc", r#"tu\nv\'wxy"#))
        );
        assert_eq!(
            md_link_title("(ti\n\ntle)abc"),
            Err(nom::Err::Error(nom::error::Error::new(
                "(ti\n\ntle)abc",
                ErrorKind::Verify
            )))
        );
    }
}
