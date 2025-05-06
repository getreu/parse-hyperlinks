//! This module implements parsers for HTML image elements.
#![allow(dead_code)]

use super::markdown::md_link_destination;
use crate::parser::markdown::md_link_destination_enclosed;
use crate::parser::markdown::md_link_text;
use crate::parser::Link;
use crate::take_until_unbalanced;
use html_escape::decode_html_entities;
use nom::combinator::*;
use nom::{bytes::complete::tag, sequence::tuple};
use std::borrow::Cow;

/// Wrapper around `md_img()` that packs the result in
/// `Link::Image`.
pub fn md_img_link(i: &str) -> nom::IResult<&str, Link> {
    let (i, (alt, src)) = md_img(i)?;
    Ok((i, Link::Image(alt, src)))
}

/// Parse a Markdown image.
///
/// It returns either `Ok((i, (img_alt, img_src)))` or some error.
///
/// The parser expects to start at the link start (`!`) to succeed.
/// ```
/// use parse_hyperlinks;
/// use parse_hyperlinks::parser::markdown_img::md_img;
/// use std::borrow::Cow;
///
/// assert_eq!(
///   md_img("![my Dog](/images/my&dog.png)abc"),
///   Ok(("abc", (Cow::from("my Dog"), Cow::from("/images/my&dog.png"))))
/// );
/// ```
pub fn md_img(i: &str) -> nom::IResult<&str, (Cow<str>, Cow<str>)> {
    nom::sequence::preceded(
        tag("!"),
        // Parse inline link.
        nom::sequence::tuple((md_link_text, md_img_link_destination_enclosed)),
    )(i)
}

/// Matches `md_link_destination` in parenthesis.
fn md_img_link_destination_enclosed(i: &str) -> nom::IResult<&str, Cow<str>> {
    map_parser(
        nom::sequence::delimited(tag("("), take_until_unbalanced('(', ')'), tag(")")),
        md_link_destination,
    )(i)
}

/// Wrapper around `md_img()` that packs the result in
/// `Link::Image`.
pub fn md_img2dest_link(i: &str) -> nom::IResult<&str, Link> {
    let (i, (text1, img_alt, img_src, text2, dest, title)) = md_img2dest(i)?;
    Ok((
        i,
        Link::Image2Dest(text1, img_alt, img_src, text2, dest, title),
    ))
}

/// Parse a Markdown link with an embedded image.
///
/// It returns either
// `Ok((i, (text1, img_alt, img_src, text2, dest, title)))` or some error.
///
/// The parser expects to start at the link start (`!`) to succeed.
/// ```
/// use parse_hyperlinks;
/// use parse_hyperlinks::parser::markdown_img::md_img2dest;
/// use std::borrow::Cow;
///
/// assert_eq!(
///   md_img2dest("[111![my dog](/my&dog.png)222]\
///                (<http://page.com> \"my title\")abc"),
///   Ok(("abc",
///    (Cow::from("111"), Cow::from("my dog"), Cow::from("/my&dog.png"),
///     Cow::from("222"), Cow::from("http://page.com"), Cow::from("my title"),
/// ))));
/// ```
#[allow(clippy::type_complexity)]
pub fn md_img2dest(
    i: &str,
) -> nom::IResult<&str, (Cow<str>, Cow<str>, Cow<str>, Cow<str>, Cow<str>, Cow<str>)> {
    map(
        nom::sequence::tuple((
            map_parser(
                nom::sequence::delimited(tag("["), take_until_unbalanced('[', ']'), tag("]")),
                tuple((
                    nom::bytes::complete::take_until("!["),
                    md_img,
                    nom::combinator::rest,
                )),
            ),
            md_link_destination_enclosed,
        )),
        // ((&str, (Cow<'_, str>, Cow<'_, str>), &str), (Cow<'_, str>, Cow<'_, str>)
        |((a, (b, c), d), (e, f))| (decode_html_entities(a), b, c, decode_html_entities(d), e, f),
    )(i)
}
