//! This module implements parsers to extract hyperlinks and image elements
//! from HTML text input. The parsers in this module search for HTML only,
//! no other markup languages are recognized.
#![allow(dead_code)]

use crate::parser::html::html_img_link;
use nom::bytes::complete::take_till;
use nom::character::complete::anychar;
use parse_hyperlinks::parser::html::html_text2dest_link;
use parse_hyperlinks::parser::Link;

/// Consumes the input until the parser finds an HTML formatted _inline image_ (`Link::Image`).
///
/// The parser consumes the finding and returns
/// `Ok((remaining_input, (skipped_input, Link)))` or some error.
///
///
/// # HTML
///
/// ```
/// use parse_hyperlinks::parser::Link;
/// use parse_hyperlinks_extras::parser::parse_html::take_img;
/// use std::borrow::Cow;
///
/// let i = r#"abc<img src="destination1" alt="text1">abc
/// abc<img src="destination2" alt="text2">abc
/// "#;
///
/// let (i, r) = take_img(i).unwrap();
/// assert_eq!(r.0, "abc");
/// assert_eq!(r.1, Link::Image(Cow::from("text1"), Cow::from("destination1")));
/// let (i, r) = take_img(i).unwrap();
/// assert_eq!(r.0, "abc\nabc");
/// assert_eq!(r.1, Link::Image(Cow::from("text2"), Cow::from("destination2")));
/// ```
pub fn take_img(i: &str) -> nom::IResult<&str, (&str, Link)> {
    let mut j = i;
    let mut skip_count = 0;

    let res = loop {
        // Start searching for inline images.

        // Regular `Link::Image` can start everywhere.
        if let Ok((k, r)) = html_img_link(j) {
            break (k, r);
        };

        // This makes sure that we advance.
        let (k, _) = anychar(j)?;
        skip_count += j.len() - k.len();
        j = k;

        // This might not consume bytes and never fails.
        let (k, _) = take_till(|c| c == '<')(j)?;

        skip_count += j.len() - k.len();
        j = k;
    };

    // We found a link. Return it.
    let (l, link) = res;

    let skipped_input = &i[0..skip_count];

    Ok((l, (skipped_input, link)))
}

/// Consumes the input until the parser finds an HTML formatted hyperlink _text2dest_
/// (`Link::Text2Dest`).
///
/// The parser consumes the finding and returns
/// `Ok((remaining_input, (skipped_input, Link)))` or some error.
///
///
/// # HTML
///
/// ```
/// use parse_hyperlinks::parser::Link;
/// use parse_hyperlinks_extras::parser::parse_html::take_link;
/// use std::borrow::Cow;
///
/// let i = "abc<a href=\"dest1\" title=\"title1\">text1</a>abc\
///          abc<a href=\"dest2\" title=\"title2\">text2</a>abc";
///
/// let (i, r) = take_link(i).unwrap();
/// ```
pub fn take_link(i: &str) -> nom::IResult<&str, (&str, Link)> {
    let mut j = i;
    let mut skip_count = 0;

    let res = loop {
        // Start searching for inline hyperlinks.

        // Regular `Link::Text2Dest` can start everywhere.
        if let Ok((k, r)) = html_text2dest_link(j) {
            break (k, r);
        };

        // This makes sure that we advance.
        let (k, _) = anychar(j)?;
        skip_count += j.len() - k.len();
        j = k;

        // This might not consume bytes and never fails.
        let (k, _) = take_till(|c| c == '<')(j)?;

        skip_count += j.len() - k.len();
        j = k;
    };

    // We found a link. Return it.
    let (l, link) = res;

    let skipped_input = &i[0..skip_count];

    Ok((l, (skipped_input, link)))
}

/// Consumes the input until the parser finds an HTML formatted _inline image_ (`Link::Image`).
/// and HTML formatted hyperlinks _text2dest_ (`Link::Text2Dest`).

///
/// The parser consumes the finding and returns
/// `Ok((remaining_input, (skipped_input, Link)))` or some error.
///
///
/// # HTML
///
/// ```
/// use parse_hyperlinks::parser::Link;
/// use parse_hyperlinks_extras::parser::parse_html::take_img_link;
/// use std::borrow::Cow;
///
/// let i = r#"abc<img src="dest1" alt="text1">abc
/// abc<a href="dest2" title="title2">text2</a>abc
/// abc<img src="dest3" alt="text3">abc
/// abc<a href="dest4" title="title4">text4</a>abc";
/// "#;
///
/// let (i, r) = take_img_link(i).unwrap();
/// assert_eq!(r.0, "abc");
/// assert_eq!(r.1, Link::Image(Cow::from("text1"), Cow::from("dest1")));
/// let (i, r) = take_img_link(i).unwrap();
/// assert_eq!(r.0, "abc\nabc");
/// assert_eq!(r.1, Link::Text2Dest(Cow::from("text2"), Cow::from("dest2"), Cow::from("title2")));
/// let (i, r) = take_img_link(i).unwrap();
/// assert_eq!(r.0, "abc\nabc");
/// assert_eq!(r.1, Link::Image(Cow::from("text3"), Cow::from("dest3")));
/// let (i, r) = take_img_link(i).unwrap();
/// assert_eq!(r.0, "abc\nabc");
/// assert_eq!(r.1, Link::Text2Dest(Cow::from("text4"), Cow::from("dest4"), Cow::from("title4")));
/// ```
pub fn take_img_link(i: &str) -> nom::IResult<&str, (&str, Link)> {
    let mut j = i;
    let mut skip_count = 0;

    let res = loop {
        // Start searching for inline images.

        // Regular `Link::Image` can start everywhere.
        if let Ok((k, r)) = html_img_link(j) {
            break (k, r);
        };
        // Regular `Link::Text2Dest` can start everywhere.
        if let Ok((k, r)) = html_text2dest_link(j) {
            break (k, r);
        };

        // This makes sure that we advance.
        let (k, _) = anychar(j)?;
        skip_count += j.len() - k.len();
        j = k;

        // This might not consume bytes and never fails.
        let (k, _) = take_till(|c| c == '<')(j)?;

        skip_count += j.len() - k.len();
        j = k;
    };

    // We found a link. Return it.
    let (l, link) = res;

    let skipped_input = &i[0..skip_count];

    Ok((l, (skipped_input, link)))
}
