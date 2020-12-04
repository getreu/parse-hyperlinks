//! This module implements parsers to extract hyperlinks and link reference
//! definitions from text input.
#![allow(dead_code)]

pub mod asciidoc;
pub mod html;
pub mod markdown;
pub mod restructured_text;

use crate::parser::asciidoc::adoc_link;
use crate::parser::html::html_link;
use crate::parser::markdown::md_link;
use crate::parser::markdown::md_link_ref;
use crate::parser::restructured_text::rst_link;
use crate::parser::restructured_text::rst_link_ref;
use nom::branch::alt;
use nom::bytes::complete::take_till;
use nom::character::complete::anychar;
use nom::character::complete::space0;
use nom::combinator::*;
use std::borrow::Cow;

/// Consumes the input until it finds a Markdown, RestructuredText, Asciidoc or
/// HTML hyperlink or link reference definition. Returns `Ok(remaining_input,
/// (link_text_or_label, link_destination, link_title)`. The parser recognizes
/// stand alone links and link reference definitions.
///
/// # Limitations:
/// Link reference labels are never resolved into link text. This limitation only
/// concerns this function and the function `first_hyperlink()`. All other
/// parsers are not affected.
///
/// Very often this limitation has no effect at all. This is the case, when the _link text_ and
/// the _link label_ are identical:
///
/// ```md
/// abc [link text/label] abc
///
/// [link text/label]: /url "title"
/// ```
///
/// But in general, the _link text_ and the _link label_ can be different:
///
/// ```md
/// abc [link text][link label] abc
///
/// [link label]: /url "title"
/// ```
///
/// When a link reference definition is found, the parser outputs it's link label
/// instead of the link text, which is strictly speaking only correct when both
/// are identical. Note, the same applies to RestructuredText's link reference
/// definitions too.
///
/// Another limitation is that ReStructuredText's anonymous links are not supported.
///
///
/// # Basic usage
///
/// ```
/// use parse_hyperlinks::parser::take_hyperlink;
/// use std::borrow::Cow;
///
/// let i = r#"[a]: b 'c'
///            .. _d: e
///            ---[f](g 'h')---`i <j>`_---
///            ---<a href="l" title="m">k</a>"#;
///
/// let (i, r) = take_hyperlink(i).unwrap();
/// assert_eq!(r, (Cow::from("a"), Cow::from("b"), Cow::from("c")));
/// let (i, r) = take_hyperlink(i).unwrap();
/// assert_eq!(r, (Cow::from("d"), Cow::from("e"), Cow::from("")));
/// let (i, r) = take_hyperlink(i).unwrap();
/// assert_eq!(r, (Cow::from("f"), Cow::from("g"), Cow::from("h")));
/// let (i, r) = take_hyperlink(i).unwrap();
/// assert_eq!(r, (Cow::from("i"), Cow::from("j"), Cow::from("")));
/// let (i, r) = take_hyperlink(i).unwrap();
/// assert_eq!(r, (Cow::from("k"), Cow::from("l"), Cow::from("m")));
/// ```
/// The parser might silently consume some additional bytes after the actual finding: This happens,
/// when directly after a finding a `md_link_ref` or `rst_link_ref` appears. These must be ignored,
/// as they are only allowed at the beginning of a line. The skip has to happen at this moment, as
/// the next parser does not know if the first byte it gets, is it at the beginning of a line or
/// not.
pub fn take_hyperlink(mut i: &str) -> nom::IResult<&str, (Cow<str>, Cow<str>, Cow<str>)> {
    let mut input_start = true;
    let res = loop {
        // This might not consume bytes and never fails.
        let mut j = if input_start {
            take_till(|c|
            // Here we should check for `md_link_ref`, `rst_link_ref` and `adoc_link`:
            c == '\n' || c == ' ' || c == '\t'
            // Possible start for `rst_link_ref:
            || c == '.'
            // These are candidates for `md_link`and `rst_link`:
            || c == '`' || c == '['
            // Asciidoc links start with `http` `link`.
            || c == 'h' || c == 'l'
            // And this could be an HTML hyperlink:
            || c == '<')(i)?
            .0
        } else {
            take_till(|c|
            // Here we should check for `md_link_ref`, `rst_link_ref` and `adoc_link`
            c == '\n'
            // Possible start for `adoc_link`:
            || c == ' ' || c == '\t'
            // These are candidates for `md_link`and `rst_link`:
            || c == '`' || c == '['
            // And this could be an HTML hyperlink:
            || c == '<')(i)?
            .0
        };

        let mut line_start = false;
        // Are we on a new line character?
        if peek(anychar)(j)?.1 == '\n' {
            line_start = true;
            // Consume the `\n`.
            // Advance one character.
            let (k, _) = anychar(j)?;
            j = k;
        };

        // Start searching for links.

        // Are we at the beginning of a line?
        if line_start || input_start {
            if let Ok(r) = alt((md_link_ref, rst_link_ref, adoc_link))(j) {
                break r;
            };
        };
        input_start = false;

        // Are we on a whitespace? Then, check for `adoc_link`.
        if let Ok(_) = nom::character::complete::space1::<_, nom::error::Error<_>>(j) {
            if let Ok(r) = adoc_link(j) {
                break r;
            }
        }

        // Regular links can start everywhere.
        if let Ok(r) = alt((rst_link, md_link, html_link))(j) {
            break r;
        };

        // This makes sure that we advance.
        let (j, _) = anychar(j)?;
        // To be faster we skip whitespace, if there are any.
        let (j, _) = space0(j)?;
        i = j;
    };

    // Before we return `res`, we need to check again for `md_link_ref` and
    // `rst_link_ref` and consume them silently, without returning their result.
    // These are only allowed at the beginning of a line and we know here, that
    // we are not. We have to act now, because the next parser can not tell if
    // its first byte is at the beginning of the line, because it does not know
    // if it was called for the first time ore not. By consuming more now, we
    // make sure that no `md_link_ref` and `rst_link_ref` is mistakenly
    // recognized in the middle of a line.
    // It is sufficient to do this check once, because both parser guarantee to
    // consume the whole line in case of success.
    if let Ok((i, _)) = alt((rst_link_ref, md_link_ref))(res.0) {
        Ok((i, res.1))
    } else {
        Ok(res)
    }
}

/// Searches for the first hyperlink or link reference definition in the input
/// text and returns the finding as a tuple:
/// `Some((link_text_or_label, link_destination, link_title))`
/// The function recognizes hyperlinks in Markdown, RestructuredText, Asciidoc or
/// HTML format. See function `take_hyperlink()` for limitations.
///
/// ```
/// use parse_hyperlinks::parser::first_hyperlink;
/// use std::borrow::Cow;
///
/// let i = "abc\n   [u]: v \"w\"abc";
///
/// let r = first_hyperlink(i);
/// assert_eq!(r, Some((Cow::from("u"), Cow::from("v"), Cow::from("w"))));
/// ```
pub fn first_hyperlink(i: &str) -> Option<(Cow<str>, Cow<str>, Cow<str>)> {
    if let Ok((_, result)) = take_hyperlink(i) {
        Some(result)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_take_hyperlink() {
        let expected = nom::Err::Error(nom::error::Error::new("", nom::error::ErrorKind::Eof));
        let err = take_hyperlink("").unwrap_err();
        assert_eq!(err, expected);

        let i = r#"[md link name]: md_link_destination "md link title"
abc [md link name](md_link_destination "md link title")abc
   [md link name]: md_link_destination "md link title"[no-md]: no[no-md]: no
abc`rst link name <rst_link_destination>`_abc
abc`rst link name <rst_link_destination>`_ .. _norst: no .. _norst: no
.. _rst link name: rst_link_destination
  .. _rst link name: rst_link_d
     estination
<a href="html_link_destination"
   title="html link title">html link name</a>
abc https://adoc_link_destination[adoc link name] abc
"#;

        let expected = (
            Cow::from("md link name"),
            Cow::from("md_link_destination"),
            Cow::from("md link title"),
        );
        let (i, res) = take_hyperlink(i).unwrap();
        assert_eq!(res, expected);
        let (i, res) = take_hyperlink(i).unwrap();
        assert_eq!(res, expected);
        let (i, res) = take_hyperlink(i).unwrap();
        assert_eq!(res, expected);

        let expected = (
            Cow::from("rst link name"),
            Cow::from("rst_link_destination"),
            Cow::from(""),
        );
        let (i, res) = take_hyperlink(i).unwrap();
        assert_eq!(res, expected);
        let (i, res) = take_hyperlink(i).unwrap();
        assert_eq!(res, expected);
        let (i, res) = take_hyperlink(i).unwrap();
        assert_eq!(res, expected);
        let (i, res) = take_hyperlink(i).unwrap();
        assert_eq!(res, expected);

        let expected = (
            Cow::from("html link name"),
            Cow::from("html_link_destination"),
            Cow::from("html link title"),
        );
        let (i, res) = take_hyperlink(i).unwrap();
        assert_eq!(res, expected);

        let expected = (
            Cow::from("adoc link name"),
            Cow::from("https://adoc_link_destination"),
            Cow::from(""),
        );
        let (_, res) = take_hyperlink(i).unwrap();
        assert_eq!(res, expected);

        // Do we find at the input start also?
        let i = ".. _`My: home page`: http://getreu.net\nabc";
        let expected = (
            Cow::from("My: home page"),
            Cow::from("http://getreu.net"),
            Cow::from(""),
        );
        let (i, res) = take_hyperlink(i).unwrap();
        assert_eq!(res, expected);
        assert_eq!(i, "\nabc");

        let i = "https://adoc_link_destination[adoc link name]abc";
        let expected = (
            Cow::from("adoc link name"),
            Cow::from("https://adoc_link_destination"),
            Cow::from(""),
        );
        let (i, res) = take_hyperlink(i).unwrap();
        assert_eq!(res, expected);
        assert_eq!(i, "abc");
    }

    #[test]
    fn test_first_hyperlink() {
        let i = "abc\n   [md link name]: md_link_destination \"md link title\"abc";

        let expected = (
            Cow::from("md link name"),
            Cow::from("md_link_destination"),
            Cow::from("md link title"),
        );
        let res = first_hyperlink(i).unwrap();
        assert_eq!(res, expected);

        let err = first_hyperlink("no link here");
        assert_eq!(err, None);

        let err = first_hyperlink("");
        assert_eq!(err, None);
    }
}
