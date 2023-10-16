//! This module implements parsers for HTML image elements.
#![allow(dead_code)]

use crate::parser::markdown::md_link_text;
use crate::parser::Link;
use crate::take_until_unbalanced;
use nom::bytes::complete::tag;
use nom::combinator::*;
use std::borrow::Cow;

use super::markdown::md_link_destination;

/// Wrapper around `md_img()` that packs the result in
/// `Link::Image`.
pub fn md_img_link(i: &str) -> nom::IResult<&str, Link> {
    let (i, (alt, src)) = md_img(i)?;
    Ok((i, Link::Image(alt, src)))
}

/// Parse an Markdown _image_.
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

#[cfg(test)]
mod tests {

    #[test]
    fn test_md_img_link() {
        // input = r#"[![Add to wallet](https://booking.stenaline.de/-/media/Images/DE/Logos/add-to-apple-wallet_143x44.png?w=220&hash=DCF2994B38)](https://booking.stenaline.de/book/Confirmation/PassBook/813609)"#;
        // // TODO
        // let expected = (
        //     "",
        //     vec![
        //         ("", Cow::from("")),
        //         ("src", Cow::from("http://getreu.net/my&dog.png")),
        //         ("", Cow::from("")),
        //         ("alt", Cow::from("My dog")),
        //         ("", Cow::from("")),
        //     ],
        // );
        // assert_eq!(
        //     todo(r#"abc src="http://getreu.net/my&amp;dog.png" abc alt="My dog" abc"#)
        //         .unwrap(),
        //     expected
        // );
    }
}
