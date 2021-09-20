//! This module implements parsers for Wikitext hyperlinks.
#![allow(dead_code)]
#![allow(clippy::type_complexity)]

use crate::parser::Link;
use nom::branch::alt;
use nom::bytes::complete::is_not;
use nom::bytes::complete::tag;
use percent_encoding::percent_decode_str;
use std::borrow::Cow;

/// Wrapper around `wikitext_text2dest()` that packs the result in
/// `Link::Text2Dest`.
pub fn wikitext_text2dest_link(i: &str) -> nom::IResult<&str, Link> {
    let (i, (te, de, ti)) = wikitext_text2dest(i)?;
    Ok((i, Link::Text2Dest(te, de, ti)))
}

/// Parse an Wikitext _inline hyperlink_.
///
/// It returns either `Ok((i, (link_text, link_destination, Cow::from("")))`
/// or some error.
///
/// The parser expects to start at the link start (`[`) to succeed.
/// ```
/// use parse_hyperlinks::parser::Link;
/// use parse_hyperlinks::parser::wikitext::wikitext_text2dest;
/// use std::borrow::Cow;
///
/// let expected = (
///     "abc",
///     (
///         Cow::from("W3Schools"),
///         Cow::from("https://www.w3schools.com/"),
///         Cow::from(""),
///     ),
/// );
/// assert_eq!(
///     wikitext_text2dest("[https://www.w3schools.com/ W3Schools]abc").unwrap(),
///     expected
/// );
/// ```
pub fn wikitext_text2dest(i: &str) -> nom::IResult<&str, (Cow<str>, Cow<str>, Cow<str>)> {
    let (i, (link_text, link_destination)) = nom::sequence::delimited(
        // HTML is case insensitive. XHTML, that is being XML is case sensitive.
        // Here we deal with HTML.
        tag("["),
        nom::combinator::map_parser(is_not("]\n\r"), parse_inner),
        tag("]"),
    )(i)?;
    Ok((i, (link_text, link_destination, Cow::from(""))))
}

/// Parse link destination and link text.
fn parse_inner(i: &str) -> nom::IResult<&str, (Cow<str>, Cow<str>)> {
    let (i, link_destination) = nom::sequence::terminated(
        nom::combinator::map_parser(
            nom::bytes::complete::take_till(|c| c == ' ' || c == '\t'),
            parse_url,
        ),
        nom::character::complete::space0,
    )(i)?;
    let link_text = i;
    Ok((i, (Cow::from(link_text), link_destination)))
}

/// Parse URL.
fn parse_url(i: &str) -> nom::IResult<&str, Cow<str>> {
    nom::combinator::peek(alt((tag("http:"), tag("https:"), tag("mailto:"))))(i)?;
    // We can safely unwrap here because `str` is guaranteed to be
    // UTF-8.
    let url = percent_decode_str(i).decode_utf8().unwrap();
    Ok(("", url))
}

#[test]
fn test_wikitext_text2dest() {
    let expected = (
        "abc",
        (
            Cow::from("W3Schools"),
            Cow::from("https://www.w3schools.com/"),
            Cow::from(""),
        ),
    );
    assert_eq!(
        wikitext_text2dest(r#"[https://www.w3schools.com/ W3Schools]abc"#).unwrap(),
        expected
    );
    assert_eq!(
        wikitext_text2dest(r#"[https://www.w3schools.com/   W3Schools]abc"#).unwrap(),
        expected
    );
    let expected = (
        "abc",
        (
            Cow::from("W3Schools"),
            Cow::from("http://www.w3schools.com/"),
            Cow::from(""),
        ),
    );
    assert_eq!(
        wikitext_text2dest(r#"[http://www.w3schools.com/ W3Schools]abc"#).unwrap(),
        expected
    );
    let expected = (
        "abc",
        (
            Cow::from("W3Schools website"),
            Cow::from("http://www.w3schools.com/"),
            Cow::from(""),
        ),
    );
    assert_eq!(
        wikitext_text2dest(r#"[http://www.w3schools.com/ W3Schools website]abc"#).unwrap(),
        expected
    );
    assert_eq!(
        wikitext_text2dest("[http://www.w3schools.com/\tW3Schools website]abc").unwrap(),
        expected
    );
    let expected = (
        "abc",
        (
            Cow::from(""),
            Cow::from("http://www.w3schools.com/"),
            Cow::from(""),
        ),
    );
    assert_eq!(
        wikitext_text2dest(r#"[http://www.w3schools.com/]abc"#).unwrap(),
        expected
    );
    assert_eq!(
        wikitext_text2dest(r#"[http://www.w3schools.com/ ]abc"#).unwrap(),
        expected
    );
    assert_eq!(
        wikitext_text2dest("[http://www.w3schools.com/\t ]abc").unwrap(),
        expected
    );
    let expected = (
        "abc",
        (
            Cow::from("John Don"),
            Cow::from("mailto:john.don@somemail.com"),
            Cow::from(""),
        ),
    );
    assert_eq!(
        wikitext_text2dest(r#"[mailto:john.don@somemail.com John Don]abc"#).unwrap(),
        expected
    );

    assert_eq!(
        wikitext_text2dest(r#"[httpx://www.w3schools.com/ W3Schools]abc"#).unwrap_err(),
        nom::Err::Error(nom::error::Error::new(
            "httpx://www.w3schools.com/",
            nom::error::ErrorKind::Tag
        ))
    );
}
