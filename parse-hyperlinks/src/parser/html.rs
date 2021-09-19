//! This module implements parsers for HTML hyperlinks.
#![allow(dead_code)]
#![allow(clippy::type_complexity)]

use crate::parser::Link;
use html_escape::decode_html_entities;
use nom::branch::alt;
use nom::bytes::complete::is_not;
use nom::bytes::complete::tag;
use nom::character::complete::alphanumeric1;
use nom::error::Error;
use nom::error::ErrorKind;
use std::borrow::Cow;

/// Wrapper around `html_text2dest()` that packs the result in
/// `Link::Text2Dest`.
pub fn html_text2dest_link(i: &str) -> nom::IResult<&str, Link> {
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
pub fn html_text2dest(i: &str) -> nom::IResult<&str, (Cow<str>, Cow<str>, Cow<str>)> {
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
    )(i)?;
    let link_text = decode_html_entities(link_text);
    Ok((i, (link_text, link_destination, link_title)))
}

/// Parses a `<a ...>` opening tag and returns
/// either `Ok((i, (link_destination, link_title)))` or some error.
fn tag_a_opening(i: &str) -> nom::IResult<&str, (Cow<str>, Cow<str>)> {
    nom::sequence::delimited(
        // HTML is case insensitive. XHTML, that is being XML is case sensitive.
        // Here we deal with HTML.
        alt((tag("<a "), tag("<A "))),
        nom::combinator::map_parser(is_not(">"), parse_attributes),
        tag(">"),
    )(i)
}

/// Parses attributes and returns `Ok((name, value))`.
/// Boolean attributes are ignored, but silently consumed.
fn attribute(i: &str) -> nom::IResult<&str, (&str, Cow<str>)> {
    alt((
        nom::sequence::pair(
            nom::combinator::verify(alphanumeric1, |s: &str| {
                nom::character::is_alphabetic(s.as_bytes()[0])
            }),
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
        nom::combinator::value(
            ("", Cow::from("")),
            nom::combinator::verify(alphanumeric1, |s: &str| {
                nom::character::is_alphabetic(s.as_bytes()[0])
            }),
        ),
    ))(i)
}

/// Parses a whitespace separated list of attributes and returns a vector of (name, value).
pub fn attribute_list<'a>(i: &'a str) -> nom::IResult<&'a str, Vec<(&'a str, Cow<str>)>> {
    let i = i.trim();
    nom::multi::separated_list1(nom::character::complete::multispace1, attribute)(i)
}

/// Extracts the `href` and `title` attributes and returns
/// `Ok((link_destination, link_title))`. `link_title` can be empty,
/// `link_destination` not.
fn parse_attributes(i: &str) -> nom::IResult<&str, (Cow<str>, Cow<str>)> {
    let (i, attributes) = attribute_list(i)?;
    let mut href = Cow::Borrowed("");
    let mut title = Cow::Borrowed("");

    for (name, value) in attributes {
        if name == "href" {
            // Make sure `href` is empty, it can appear only
            // once.
            if !(&*href).is_empty() {
                return Err(nom::Err::Error(Error::new(name, ErrorKind::ManyMN)));
            }
            href = value;
        } else if name == "title" {
            // Make sure `title` is empty, it can appear only
            // once.
            if !(&*title).is_empty() {
                return Err(nom::Err::Error(Error::new(name, ErrorKind::ManyMN)));
            }
            title = value;
        }
    }

    // Assure that `href` is not empty.
    if (&*href).is_empty() {
        return Err(nom::Err::Error(Error::new(i, ErrorKind::Eof)));
    };

    Ok((i, (href, title)))
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let expected = nom::Err::Error(nom::error::Error::new(
            r#"<a href="url" title="" >name</a abc"#,
            nom::error::ErrorKind::AlphaNumeric,
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
}
