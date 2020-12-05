//! This module implements parsers to extract hyperlinks and link reference
//! definitions from text input.
#![allow(dead_code)]

pub mod asciidoc;
pub mod html;
pub mod markdown;
pub mod restructured_text;

use crate::parser::asciidoc::adoc_text2dest_link;
use crate::parser::html::html_text2dest_link;
use crate::parser::markdown::md_label2dest_link;
use crate::parser::markdown::md_text2dest_link;
use crate::parser::markdown::md_text2label_link;
use crate::parser::restructured_text::rst_label2dest_link;
use crate::parser::restructured_text::rst_text2dest_link;
use nom::branch::alt;
use nom::bytes::complete::take_till;
use nom::character::complete::anychar;
use nom::character::complete::space0;
use nom::combinator::*;
use std::borrow::Cow;

/// A link can be an _inline link_, a _reference link_ or a _link reference
/// definition_. This is the main return type of this API.
#[derive(Debug, PartialEq, Clone)]
pub enum Link<'a> {
    /// In (stand alone) **inline links** the destination and title are given
    /// immediately after the link text. When the _inline link_ is rendered, only
    /// the `link_text` is visible in the continuos text.
    ///
    /// The tuple definition: `Text2Dest(link_text, link_destination, link_title)`
    Text2Dest(Cow<'a, str>, Cow<'a, str>, Cow<'a, str>),
    /// In **reference links** the destination and title are defined elsewhere in
    /// the document in some _link reference definition_. When a _reference link_
    /// is rendered only the `link_text` is visible in the continuos text.
    ///
    /// Tuple definition: `Text2Label(link_text, link_label)`
    Text2Label(Cow<'a, str>, Cow<'a, str>),
    /// A **link reference definition** refers to a _reference link_ with the
    /// same _link label_. A _link reference definition_ is not visible
    /// when the document is rendered.
    ///
    /// Tuple definition: `Label2Dest(link_label, link_destination, link_title)`
    Label2Dest(Cow<'a, str>, Cow<'a, str>, Cow<'a, str>),
    /// The **reference alias** defines an alternative link label
    /// `alt_link_label` for an existing `link_label` defined elsewhere in the
    /// document. At some point the `link_label` needs to be resolved to a
    /// `link_destination` by some _link_reference_definition_. A _reference
    /// alias_ is not visible when the document is rendered.
    ///
    /// Tuple definition: `Label2Label(alt_link_label, link_label)`
    Label2Label(Cow<'a, str>, Cow<'a, str>),
}

/// Consumes the input until it finds a Markdown, RestructuredText, Asciidoc or
/// HTML formatted _inline link_ (`Text2Dest`) or
/// or _link reference definition_ (`Label2Dest`).
///
/// Returns `Ok((remaining_input, (link_text_or_label, link_destination,
/// link_title)))`. The parser recognizes only stand alone _inline links_ and
/// _link reference definitions_, but no _reference links_.
///
/// # Limitations:
/// Link reference labels are never resolved into link text. This limitation only
/// concerns this parser. Others are not affected.
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
/// use parse_hyperlinks::parser::take_text2dest_label2dest;
/// use std::borrow::Cow;
///
/// let i = r#"[a]: b 'c'
///            .. _d: e
///            ---[f](g 'h')---`i <j>`_---
///            ---<a href="l" title="m">k</a>"#;
///
/// let (i, r) = take_text2dest_label2dest(i).unwrap();
/// assert_eq!(r, (Cow::from("a"), Cow::from("b"), Cow::from("c")));
/// let (i, r) = take_text2dest_label2dest(i).unwrap();
/// assert_eq!(r, (Cow::from("d"), Cow::from("e"), Cow::from("")));
/// let (i, r) = take_text2dest_label2dest(i).unwrap();
/// assert_eq!(r, (Cow::from("f"), Cow::from("g"), Cow::from("h")));
/// let (i, r) = take_text2dest_label2dest(i).unwrap();
/// assert_eq!(r, (Cow::from("i"), Cow::from("j"), Cow::from("")));
/// let (i, r) = take_text2dest_label2dest(i).unwrap();
/// assert_eq!(r, (Cow::from("k"), Cow::from("l"), Cow::from("m")));
/// ```
/// The parser might silently consume some additional bytes after the actual finding: This happens,
/// when directly after a finding a `md_link_ref` or `rst_link_ref` appears. These must be ignored,
/// as they are only allowed at the beginning of a line. The skip has to happen at this moment, as
/// the next parser does not know if the first byte it gets, is it at the beginning of a line or
/// not.
///
/// Technically, this parser is a wrapper around `take_link()`, that erases the
/// link type information and ignores all _reference links_. As it does not
/// resolve _reference links_, it is much faster than the `hyperlink_list()` function.
pub fn take_text2dest_label2dest(i: &str) -> nom::IResult<&str, (Cow<str>, Cow<str>, Cow<str>)> {
    let mut j = i;
    loop {
        match take_link(j) {
            Ok((j, Link::Text2Dest(lte, ld, lti))) => return Ok((j, (lte, ld, lti))),
            Ok((j, Link::Label2Dest(ll, ld, lti))) => return Ok((j, (ll, ld, lti))),
            // We ignore `Link::Ref()` and `Link::RefAlias`. Instead we continue parsing.
            Ok((k, _)) => {
                j = k;
                continue;
            }
            Err(e) => return Err(e),
        };
    }
}

/// Consumes the input until it finds a Markdown, RestructuredText, Asciidoc or
/// HTML formatted _inline link_ (`Text2Dest`), _reference link_ (`Text2Label`),
/// _link reference definition_ (`Label2Dest`) or _reference alias_ (`Label2Label`).
///
/// The parser consumes the finding and returns `Ok((remaining_input, Link))` or some error.
/// # Basic usage
///
/// ```
/// use parse_hyperlinks::parser::Link;
/// use parse_hyperlinks::parser::take_link;
/// use std::borrow::Cow;
///
/// let i = r#"[a]: b 'c'
///            .. _d: e
///            ---[f](g 'h')---`i <j>`_---
///            ---[k][l]---
///            ---<a href="m" title="n">o</a>---
/// "#;
///
/// let (i, r) = take_link(i).unwrap();
/// assert_eq!(r, Link::Label2Dest(Cow::from("a"), Cow::from("b"), Cow::from("c")));
/// let (i, r) = take_link(i).unwrap();
/// assert_eq!(r, Link::Label2Dest(Cow::from("d"), Cow::from("e"), Cow::from("")));
/// let (i, r) = take_link(i).unwrap();
/// assert_eq!(r, Link::Text2Dest(Cow::from("f"), Cow::from("g"), Cow::from("h")));
/// let (i, r) = take_link(i).unwrap();
/// assert_eq!(r, Link::Text2Dest(Cow::from("i"), Cow::from("j"), Cow::from("")));
/// let (i, r) = take_link(i).unwrap();
/// assert_eq!(r, Link::Text2Label(Cow::from("k"), Cow::from("l")));
/// let (i, r) = take_link(i).unwrap();
/// assert_eq!(r, Link::Text2Dest(Cow::from("o"), Cow::from("m"), Cow::from("n")));
/// ```
pub fn take_link(mut i: &str) -> nom::IResult<&str, Link> {
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
            if let Ok((j, r)) = alt((md_label2dest_link, rst_label2dest_link))(j) {
                break (j, r);
            };
            if let Ok((j, r)) = adoc_text2dest_link(j) {
                break (j, r);
            };
        };
        input_start = false;

        // Are we on a whitespace? Then, check for `adoc_link`.
        if let Ok(_) = nom::character::complete::space1::<_, nom::error::Error<_>>(j) {
            if let Ok((j, r)) = adoc_text2dest_link(j) {
                break (j, r);
            };
        }

        // Regular links can start everywhere.
        if let Ok((j, r)) = alt((rst_text2dest_link, md_text2dest_link, html_text2dest_link))(j) {
            break (j, r);
        };

        // Now at the end, we check for _reference links_.
        // TODO: at the moment there is only md.
        if let Ok((j, r)) = md_text2label_link(j) {
            break (j, r);
        };

        // This makes sure that we advance.
        let (j, _) = anychar(j)?;
        // To be faster, we skip whitespace, if there is any.
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
    let (mut i, link) = res;
    if !matches!(&link, Link::Label2Dest{..}) {
        // Just consume, the result does not matter.
        let (j, _) = alt((
            rst_label2dest_link,
            md_label2dest_link,
            // If none was found do nothing.
            nom::combinator::success(link.clone()),
        ))(i)?;
        i = j;
    };

    Ok((i, link))
}

/// Recognizes hyperlinks in Markdown, RestructuredText, Asciidoc or
/// HTML format and returns the first hyperlink found as tuple:
/// `Some((link_text_or_label, link_destination, link_title))`.
///
/// It returns `None` if no hyperlinks were found.
/// See function `take_text2dest_label2dest()` for limitations.
/// ```
/// use parse_hyperlinks::parser::first_hyperlink;
/// use std::borrow::Cow;
///
/// let i = "abc\n   [u]: v \"w\"\nabc";
///
/// let r = first_hyperlink(i);
/// assert_eq!(r, Some((Cow::from("u"), Cow::from("v"), Cow::from("w"))));
/// ```
pub fn first_hyperlink(i: &str) -> Option<(Cow<str>, Cow<str>, Cow<str>)> {
    if let Ok((_, result)) = take_text2dest_label2dest(i) {
        Some(result)
    } else {
        None
    }
}

/*
/// Parses through the input text, resolves all _reference links_ and returns a
/// vector of the extracted hyperlinks `Vec<Link>` or some error.
pub fn hyperlink_list(_i: &str) -> Result<Vec<Link>, nom::error::Error<&str>> {
    unimplemented!();
    // returns something like.
    // Ok(vec![Link::Text2Dest(Cow::from(""), Cow::from(""), Cow::from(""))])
}
*/

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_take_link() {
        let expected = nom::Err::Error(nom::error::Error::new("", nom::error::ErrorKind::Eof));
        let err = take_link("").unwrap_err();
        assert_eq!(err, expected);

        let i = r#"[md link name]: md_link_destination "md link title"
abc [md link name](md_link_destination "md link title")[no md]: abc[no md]: abc
   [md link name]: md_link_destination "md link title"
abc`rst link name <rst_link_destination>`_abc
abc`rst link name <rst_link_destination>`_ .. _norst: no .. _norst: no
.. _rst link name: rst_link_destination
  .. _rst link name: rst_link_d
     estination
<a href="html_link_destination"
   title="html link title">html link name</a>
abc https://adoc_link_destination[adoc link name] abc
"#;

        let expected = Link::Label2Dest(
            Cow::from("md link name"),
            Cow::from("md_link_destination"),
            Cow::from("md link title"),
        );
        let (i, res) = take_link(i).unwrap();
        assert_eq!(res, expected);

        let expected = Link::Text2Dest(
            Cow::from("md link name"),
            Cow::from("md_link_destination"),
            Cow::from("md link title"),
        );
        let (i, res) = take_link(i).unwrap();
        assert_eq!(res, expected);

        let expected = Link::Text2Label(Cow::from("no md"), Cow::from("no md"));
        let (i, res) = take_link(i).unwrap();
        assert_eq!(res, expected);
        let (i, res) = take_link(i).unwrap();
        assert_eq!(res, expected);

        let expected = Link::Label2Dest(
            Cow::from("md link name"),
            Cow::from("md_link_destination"),
            Cow::from("md link title"),
        );
        let (i, res) = take_link(i).unwrap();
        assert_eq!(res, expected);

        let expected = Link::Text2Dest(
            Cow::from("rst link name"),
            Cow::from("rst_link_destination"),
            Cow::from(""),
        );
        let (i, res) = take_link(i).unwrap();
        assert_eq!(res, expected);
        let (i, res) = take_link(i).unwrap();
        assert_eq!(res, expected);

        let expected = Link::Label2Dest(
            Cow::from("rst link name"),
            Cow::from("rst_link_destination"),
            Cow::from(""),
        );
        let (i, res) = take_link(i).unwrap();
        assert_eq!(res, expected);
        let (i, res) = take_link(i).unwrap();
        assert_eq!(res, expected);

        let expected = Link::Text2Dest(
            Cow::from("html link name"),
            Cow::from("html_link_destination"),
            Cow::from("html link title"),
        );
        let (i, res) = take_link(i).unwrap();
        assert_eq!(res, expected);

        let expected = Link::Text2Dest(
            Cow::from("adoc link name"),
            Cow::from("https://adoc_link_destination"),
            Cow::from(""),
        );
        let (_, res) = take_link(i).unwrap();
        assert_eq!(res, expected);

        // Do we find at the input start also?
        let i = ".. _`My: home page`: http://getreu.net\nabc";
        let expected = Link::Label2Dest(
            Cow::from("My: home page"),
            Cow::from("http://getreu.net"),
            Cow::from(""),
        );
        let (i, res) = take_link(i).unwrap();
        assert_eq!(res, expected);
        assert_eq!(i, "\nabc");

        let i = "https://adoc_link_destination[adoc link name]abc";
        let expected = Link::Text2Dest(
            Cow::from("adoc link name"),
            Cow::from("https://adoc_link_destination"),
            Cow::from(""),
        );
        let (i, res) = take_link(i).unwrap();
        assert_eq!(res, expected);
        assert_eq!(i, "abc");
    }

    #[test]
    fn test_first_hyperlink() {
        let i = "abc\n   [md link name]: md_link_destination \"md link title\"  \nabc";

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
