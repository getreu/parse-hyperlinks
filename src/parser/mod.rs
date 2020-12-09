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
use crate::parser::restructured_text::rst_text2label_link;
use crate::parser::restructured_text::rst_text_label2dest_link;
use nom::branch::alt;
use nom::bytes::complete::take_till;
use nom::character::complete::anychar;
use std::borrow::Cow;

/// A link can be an _inline link_, a _reference link_, a _link reference
/// definition_, a combined _inline link / link reference definition_ or
/// a _reference alias_. This is the main return type of this API.
///
/// The _link title_ in Markdown is optional, when not given the string
/// is set to the empty string `""`.
/// The back ticks \` in reStructuredText can be omitted when only one
/// word is enclosed without spaces.

#[derive(Debug, PartialEq, Clone)]
pub enum Link<'a> {
    /// In (stand alone) **inline links** the destination and title are given
    /// immediately after the link text. When an _inline link_ is rendered, only
    /// the `link_text` is visible in the continuos text.
    /// * Markdown example:
    ///   ```md
    ///       [link_text](link_dest "link title")
    ///   ```
    ///
    /// * reStructuredText example:
    ///   ```rst
    ///       `link_text <link_dest>`__
    ///   ```
    ///
    /// *  Asciidoc example:
    ///    ```adoc
    ///    http://link_dest[link_text]
    ///    ```
    /// The tuple is defined as follows:
    /// ```text
    /// Text2Dest(link_text, link_destination, link_title)
    /// ```
    Text2Dest(Cow<'a, str>, Cow<'a, str>, Cow<'a, str>),

    /// In **reference links** the destination and title are defined elsewhere in
    /// the document in some _link reference definition_. When a _reference link_
    /// is rendered only `link_text` is visible.
    /// * Markdown examples:
    ///   ```md
    ///   [link_text][link_label]
    ///
    ///   [link_text]
    ///   ```
    ///   When only _link text_ is given, _link label_ is set to the same string.
    /// * reStructuredText examples:
    ///   ```rst
    ///   `link_text <link_label_>`_
    ///
    ///   `link_text`_
    ///   ```
    ///   When only _link text_ is given, _link label_ is set to the same string.
    /// * Asciidoc example:
    ///   ```adoc
    ///   {link_label}[link_text]
    ///   ```
    ///
    /// The tuple is defined as follows:
    /// ```text
    /// Text2Label(link_text, link_label)
    /// ```
    Text2Label(Cow<'a, str>, Cow<'a, str>),

    /// A **link reference definition** refers to a _reference link_ with the
    /// same _link label_. A _link reference definition_ is not visible
    /// when the document is rendered.
    /// _link title_ is optional.
    /// * Markdown example:
    ///   ```md
    ///   [link_label]: link_dest "link title"
    ///   ```
    /// * reStructuredText examples:
    ///   ```rst
    ///   .. _`link_label`: link_dest
    ///
    ///   .. __: link_dest
    ///
    ///   __ link_dest
    ///   ```
    ///   When `__` is given, the _link label_ is set to `"_"`, which is a marker
    ///   for an anonymous _link label_.
    /// * Asciidoc example:
    ///   ```adoc
    ///   :link_label: http://link_dest
    ///   ```
    ///
    /// The tuple is defined as follows:
    /// ```text
    /// Label2Dest(link_label, link_destination, link_title)
    /// ```
    Label2Dest(Cow<'a, str>, Cow<'a, str>, Cow<'a, str>),

    /// This type represents a combined **inline link** and **link reference
    /// definition**.
    /// Semantically `TextLabel2Dest` is a shorthand for two links `Text2Dest` and
    /// `Label2Dest` in one object, where _link text_ and _link label_ are the
    /// same string. When rendered, _link text_ is visible.
    ///
    /// * Consider the following reStructuredText link:
    ///   ```rst
    ///   `link_text_label <link_dest>`_
    ///
    ///   `a <b>`_
    ///   ```
    ///   In this link is `b` the _link destination_ and `a` has a double role: it
    ///   defines _link text_ of the first link `Text2Dest("a", "b", "")` and _link
    ///   label_ of the second link `Label2Dest("a", "b", "")`.
    ///
    /// The tuple is defined as follows:
    /// ```text
    /// Label2Dest(link_text_label, link_destination, link_title)
    /// ```
    TextLabel2Dest(Cow<'a, str>, Cow<'a, str>, Cow<'a, str>),

    /// The **reference alias** defines an alternative link label
    /// `alt_link_label` for an existing `link_label` defined elsewhere in the
    /// document. At some point, the `link_label` must be resolved to a
    /// `link_destination` by a _link_reference_definition_. A _reference
    /// alias_ is not visible when the document is rendered.
    /// This link type is only available in reStructuredText, e.g.
    /// ```rst
    /// .. _`alt_link_label`: `link_label`_
    /// ```
    ///
    /// The tuple is defined as follows:
    /// ```text
    /// Label2Label(alt_link_label, link_label)
    /// ```
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
            Ok((j, (_, Link::Text2Dest(lte, ld, lti)))) => return Ok((j, (lte, ld, lti))),
            Ok((j, (_, Link::TextLabel2Dest(lte, ld, lti)))) => return Ok((j, (lte, ld, lti))),
            Ok((j, (_, Link::Label2Dest(ll, ld, lti)))) => return Ok((j, (ll, ld, lti))),
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
/// The parser consumes the finding and returns
/// `Ok((remaining_input, (skipped_input, Link)))` or some error.
/// # Basic usage
///
/// ```
/// use parse_hyperlinks::parser::Link;
/// use parse_hyperlinks::parser::take_link;
/// use std::borrow::Cow;
///
/// let i = r#"foo
/// [a]: b 'c'
/// .. _d: e
/// ---[f](g 'h')---`i <j>`__---
/// ---[k][l]---
/// ---<a href="m" title="n">o</a>---
/// ---`p <q>`_---
/// "#;
///
/// let (i, r) = take_link(i).unwrap();
/// assert_eq!(r.0, "foo\n");
/// assert_eq!(r.1, Link::Label2Dest(Cow::from("a"), Cow::from("b"), Cow::from("c")));
/// let (i, r) = take_link(i).unwrap();
/// assert_eq!(r.1, Link::Label2Dest(Cow::from("d"), Cow::from("e"), Cow::from("")));
/// let (i, r) = take_link(i).unwrap();
/// assert_eq!(r.1, Link::Text2Dest(Cow::from("f"), Cow::from("g"), Cow::from("h")));
/// let (i, r) = take_link(i).unwrap();
/// assert_eq!(r.1, Link::Text2Dest(Cow::from("i"), Cow::from("j"), Cow::from("")));
/// let (i, r) = take_link(i).unwrap();
/// assert_eq!(r.1, Link::Text2Label(Cow::from("k"), Cow::from("l")));
/// let (i, r) = take_link(i).unwrap();
/// assert_eq!(r.0, "---\n---");
/// assert_eq!(r.1, Link::Text2Dest(Cow::from("o"), Cow::from("m"), Cow::from("n")));
/// let (i, r) = take_link(i).unwrap();
/// assert_eq!(r.0, "---\n---");
/// assert_eq!(r.1, Link::TextLabel2Dest(Cow::from("p"), Cow::from("q"), Cow::from("")));
/// ```
pub fn take_link(i: &str) -> nom::IResult<&str, (&str, Link)> {
    let mut j = i;
    let mut skip_count = 0;
    let mut input_start = true;
    let mut line_start;
    let mut whitespace;
    let res = loop {
        // Are we on a new line character? consume it.
        line_start = false;
        if let Ok((k, _)) = nom::character::complete::newline::<_, nom::error::Error<_>>(j) {
            skip_count += j.len() - k.len();
            j = k;
            line_start = true;
        };

        // Are we at the beginning of a line?
        if line_start || input_start {
            if let Ok((k, r)) = alt((
                md_label2dest_link,
                rst_label2dest_link,
                rst_text2label_link,
                adoc_text2dest_link,
            ))(j)
            {
                break (k, r);
            };
            input_start = false;
        };
        // Start searching for links.

        // Are we on a whitespace? Consume them.
        whitespace = false;
        if let Ok((k, _)) = nom::character::complete::space1::<_, nom::error::Error<_>>(j) {
            skip_count += j.len() - k.len();
            j = k;
            whitespace = true;
        }

        // Regular `text` links can start everywhere.
        if let Ok((k, r)) = alt((
            md_text2dest_link,
            rst_text2dest_link,
            rst_text_label2dest_link,
            html_text2dest_link,
            // This should be the last
            md_text2label_link,
        ))(j)
        {
            break (k, r);
        };

        if whitespace {
            if let Ok((k, r)) = alt((rst_text2label_link, adoc_text2dest_link))(j) {
                break (k, r);
            };
        };

        // This makes sure that we advance.
        let (k, _) = anychar(j)?;
        skip_count += j.len() - k.len();
        j = k;

        // This might not consume bytes and never fails.
        let (k, _) = take_till(|c|
            // After this, we should check for: `md_label2dest`, `rst_label2dest`, `rst_text2label`, `adoc_text2dest`.
            c == '\n'
            // After this, possible start for `adoc_text2dest` or `rst_text2label`:
            || c == ' ' || c == '\t'
            // These are candidates for `rst_text2label`, `rst_text_label2dest` `rst_text2dest`:
            || c == '`'
            // These could be the start of all `md_*`link types.
            || c == '['
            // And this could be an HTML hyperlink:
            || c == '<')(j)?;

        skip_count += j.len() - k.len();
        j = k;
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
    let (mut l, link) = res;
    if !matches!(&link, Link::Label2Dest{..}) {
        // Just consume, the result does not matter.
        let (m, _) = alt((
            rst_label2dest_link,
            md_label2dest_link,
            // If none was found do nothing.
            nom::combinator::success(link.clone()),
        ))(l)?;
        l = m;
    };

    let skipped_input = &i[0..skip_count];

    Ok((l, (skipped_input, link)))
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

        let i = r#"[md text]: md_destination "md title"
abc [md text](md_destination "md title")[md text]: abc[md text]: abc
   [md text]: md_destination "md title"
abc `rst text <rst_destination>`__abc
abc `rst text <rst_label_>`_ .. _norst: no .. _norst: no
.. _rst text: rst_destination
  .. _rst text: rst_d
     estination
<a href="html_destination"
   title="html title">html text</a>
abc https://adoc_destination[adoc text] abc
"#;

        let expected = Link::Label2Dest(
            Cow::from("md text"),
            Cow::from("md_destination"),
            Cow::from("md title"),
        );
        let (i, (_, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);

        let expected = Link::Text2Dest(
            Cow::from("md text"),
            Cow::from("md_destination"),
            Cow::from("md title"),
        );
        let (i, (skipped, res)) = take_link(i).unwrap();
        assert_eq!(skipped, "\nabc ");
        assert_eq!(res, expected);

        let expected = Link::Text2Label(Cow::from("md text"), Cow::from("md text"));
        let (i, (_, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);
        let (i, (_, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);

        let expected = Link::Label2Dest(
            Cow::from("md text"),
            Cow::from("md_destination"),
            Cow::from("md title"),
        );
        let (i, (_, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);

        let expected = Link::Text2Dest(
            Cow::from("rst text"),
            Cow::from("rst_destination"),
            Cow::from(""),
        );
        let (i, (_, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);

        let expected = Link::Text2Label(Cow::from("rst text"), Cow::from("rst_label"));
        let (i, (_, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);

        let expected = Link::Label2Dest(
            Cow::from("rst text"),
            Cow::from("rst_destination"),
            Cow::from(""),
        );
        let (i, (_, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);
        let (i, (_, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);

        let expected = Link::Text2Dest(
            Cow::from("html text"),
            Cow::from("html_destination"),
            Cow::from("html title"),
        );
        let (i, (_, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);

        let expected = Link::Text2Dest(
            Cow::from("adoc text"),
            Cow::from("https://adoc_destination"),
            Cow::from(""),
        );
        let (_, (_, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);

        // Do we find at the input start also?
        let i = ".. _`My: home page`: http://getreu.net\nabc";
        let expected = Link::Label2Dest(
            Cow::from("My: home page"),
            Cow::from("http://getreu.net"),
            Cow::from(""),
        );
        let (i, (_, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);
        assert_eq!(i, "\nabc");

        let i = "https://adoc_link_destination[adoc link name]abc";
        let expected = Link::Text2Dest(
            Cow::from("adoc link name"),
            Cow::from("https://adoc_link_destination"),
            Cow::from(""),
        );
        let (i, (_, res)) = take_link(i).unwrap();
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
