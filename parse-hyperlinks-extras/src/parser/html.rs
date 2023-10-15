//! This module implements parsers for HTML image elements.
#![allow(dead_code)]

use nom::branch::alt;
use nom::bytes::complete::is_not;
use nom::bytes::complete::tag;
use nom::error::Error;
use nom::error::ErrorKind;
use parse_hyperlinks::parser::html::attribute_list;
use parse_hyperlinks::parser::Link;
use std::borrow::Cow;

/// Wrapper around `html_img()` that packs the result in
/// `Link::Image`.
pub fn html_img_link(i: &str) -> nom::IResult<&str, Link> {
    let (i, (alt, src)) = html_img(i)?;
    Ok((i, Link::Image(alt, src)))
}

/// Parse an HTML _image_.
///
/// It returns either `Ok((i, (img_alt, img_src)))` or some error.
///
/// The parser expects to start at the link start (`<`) to succeed.
/// ```
/// use parse_hyperlinks;
/// use parse_hyperlinks_extras::parser::html::html_img;
/// use std::borrow::Cow;
///
/// assert_eq!(
///   html_img(r#"<img src="/images/my&amp;dog.png" alt="my Dog" width="500">abc"#),
///   Ok(("abc", (Cow::from("my Dog"), Cow::from("/images/my&dog.png"))))
/// );
/// ```
pub fn html_img(i: &str) -> nom::IResult<&str, (Cow<str>, Cow<str>)> {
    tag_img(i)
}

/// Parses a `<img ...>` tag and returns
/// either `Ok((i, (img_alt, img_src)))` or some error.
#[inline]
fn tag_img(i: &str) -> nom::IResult<&str, (Cow<str>, Cow<str>)> {
    nom::sequence::delimited(
        // HTML is case insensitive. XHTML, that is being XML is case sensitive.
        // Here we deal with HTML.
        alt((tag("<img "), tag("<IMG "))),
        nom::combinator::map_parser(is_not(">"), parse_attributes),
        tag(">"),
    )(i)
}

/// Extracts the `src` and `alt` attributes and returns
/// `Ok((img_alt, img_src))`. `img_alt` can be empty,
/// `img_src` not.
fn parse_attributes(i: &str) -> nom::IResult<&str, (Cow<str>, Cow<str>)> {
    let (i, attributes) = attribute_list(i)?;
    let mut src = Cow::Borrowed("");
    let mut alt = Cow::Borrowed("");

    for (name, value) in attributes {
        if name == "src" {
            // Make sure `src` is empty, it can appear only
            // once.
            if !(&*src).is_empty() {
                return Err(nom::Err::Error(Error::new(name, ErrorKind::ManyMN)));
            }
            src = value;
        } else if name == "alt" {
            // Make sure `title` is empty, it can appear only
            // once.
            if !(&*alt).is_empty() {
                return Err(nom::Err::Error(Error::new(name, ErrorKind::ManyMN)));
            }
            alt = value;
        }
    }

    // Assure that `href` is not empty.
    if (&*src).is_empty() {
        return Err(nom::Err::Error(Error::new(i, ErrorKind::Eof)));
    };

    Ok((i, (alt, src)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use parse_hyperlinks::parser::html::attribute_list;

    #[test]
    fn test_tag_img() {
        let expected = (
            "abc",
            (
                Cow::from("My dog"),
                Cow::from("http://getreu.net/my&dog.png"),
            ),
        );
        assert_eq!(
            tag_img(r#"<img src="http://getreu.net/my&amp;dog.png" alt="My dog">abc"#).unwrap(),
            expected
        );
        assert_eq!(
            tag_img(r#"<IMG src="http://getreu.net/my&amp;dog.png" alt="My dog">abc"#).unwrap(),
            expected
        );
        assert_eq!(
            tag_img(r#"<IMG src="http://getreu.net/my&amp;dog.png" alt="My dog"/>abc"#).unwrap(),
            expected
        );
        assert_eq!(
            tag_img(r#"<IMG src="http://getreu.net/my&amp;dog.png" alt="My dog" />abc"#).unwrap(),
            expected
        );

        let expected = (
            "abc",
            (Cow::from("Some picture"), Cow::from("t%20m%20p.jpg")),
        );
        assert_eq!(
            tag_img(r#"<img src="t%20m%20p.jpg" alt="Some picture" />abc"#).unwrap(),
            expected
        );
    }

    #[test]
    fn test_parse_attributes() {
        let expected = (
            "",
            (
                Cow::from("My dog"),
                Cow::from("http://getreu.net/my&dog.png"),
            ),
        );
        assert_eq!(
            parse_attributes(r#"abc src="http://getreu.net/my&amp;dog.png" abc alt="My dog" abc"#)
                .unwrap(),
            expected
        );

        let expected =
            nom::Err::Error(nom::error::Error::new("src", nom::error::ErrorKind::ManyMN));
        assert_eq!(
            parse_attributes(r#" src="http://getreu.net" src="http://blog.getreu.net" "#)
                .unwrap_err(),
            expected
        );

        let expected =
            nom::Err::Error(nom::error::Error::new("alt", nom::error::ErrorKind::ManyMN));
        assert_eq!(
            parse_attributes(r#" src="http://getreu.net" alt="a" alt="b" "#).unwrap_err(),
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
                ("src", Cow::from("http://getreu.net/my&dog.png")),
                ("", Cow::from("")),
                ("alt", Cow::from("My dog")),
                ("", Cow::from("")),
            ],
        );
        assert_eq!(
            attribute_list(r#"abc src="http://getreu.net/my&amp;dog.png" abc alt="My dog" abc"#)
                .unwrap(),
            expected
        );
    }

    #[test]
    fn test_image_link() {
        // input = r#"[![Add to wallet](https://booking.stenaline.de/-/media/Images/DE/Logos/add-to-apple-wallet_143x44.png?w=220&hash=DCF2971402505063EB697C19F884E5A325794B38)](https://booking.stenaline.de/book/Confirmation/PassBook/895137609)"#;
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
