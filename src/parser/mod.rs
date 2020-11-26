//! This module implement parser and iterator to extract all
//! hyperlinks from a text input.
#![allow(dead_code)]

pub mod markdown;
pub mod restructured_text;

use crate::parser::markdown::md_link;
use crate::parser::markdown::md_link_ref;
use crate::parser::restructured_text::rst_link;
use crate::parser::restructured_text::rst_link_ref;
use nom::branch::alt;
use nom::bytes::complete::take_till;
use nom::character::complete::anychar;
use nom::combinator::*;

/// Skips input until it finds a Markdown or RestructuredText hyperlink.
/// Returns `Ok(remaining_input, (link_name, link_target, link_title)`.
///
/// After the finding, the parser might silently consume some additional bytes:
/// This happens, when directly after a finding a `md_link_ref` or `rst_link_ref`
/// appears. These must be ignored, as they are only allowed at the beginning of
/// a line. The skip has to happen at this moment, as the next parser does not
/// know if the first byte it gets, is it at the beginning of a line or not.
pub fn take_hyperlink(mut i: &str) -> nom::IResult<&str, (String, String, String)> {
    let mut input_start = true;
    let res = loop {
        // This might not consume bytes and never fails.
        i = if input_start {
            take_till(|c|
            // Here we should check for `md_link_ref` and `rst_link_ref`
            c == '\n' || c == ' ' || c == '.'
            // these are candidates for `md_link`and `rst_link`
            || c == '`' || c == '[')(i)?
            .0
        } else {
            take_till(|c|
            // Here we should check for `md_link_ref` and `rst_link_ref`
            c == '\n'
            // these are candidates for `md_link`and `rst_link`
            || c == '`' || c == '[')(i)?
            .0
        };

        let mut line_start = false;
        // Are we on a new line character?
        if peek(anychar)(i)?.1 == '\n' {
            line_start = true;
            // Consume the `\n`.
            // Advance one character.
            let (j, _) = anychar(i)?;
            i = j;
        };

        // Start searching for links.

        // Are we at the beginning of a line?
        if line_start || input_start {
            if let Ok(r) = alt((
                map(md_link_ref, |(ln, lta, lti)| {
                    (ln.to_string(), lta.to_string(), lti.to_string())
                }),
                map(rst_link_ref, |(ln, lt)| (ln, lt, "".to_string())),
            ))(i)
            {
                break r;
            };
        };
        input_start = false;

        // Regular links can start everywhere.
        if let Ok(r) = alt((
            map(rst_link, |(ln, lt)| (ln, lt, "".to_string())),
            map(md_link, |(ln, lta, lti)| {
                (ln.to_string(), lta.to_string(), lti.to_string())
            }),
        ))(i)
        {
            break r;
        };

        // This makes sure that we advance.
        let (j, _) = anychar(i)?;
        i = j;
    };

    // Before we return `res`, we need to check again for `md_link_ref` and
    // `rst_link_ref` and consume them silently, without returning their result.
    // These are only allowed at the beginning of a line and we know here, that
    // we are definately not. The next parser can not tell, because it does not
    // know if it was called for the first time ore not. This way, we make sure
    // that `md_link_ref` and `rst_link_ref` are mistakenly recognized in the
    // middle of a line.
    // We do this check only once, because we know, if one of the parser
    // succeeds, it will consume the whole line.
    if let Ok((i, _)) = alt((
        map(rst_link_ref, |(ln, lt)| (ln, lt, "".to_string())),
        map(md_link_ref, |(ln, lta, lti)| {
            (ln.to_string(), lta.to_string(), lti.to_string())
        }),
    ))(res.0)
    {
        Ok((i, res.1))
    } else {
        Ok(res)
    }
}

/// Returns the parsed first hyperlink found in the input text as:
/// Some((link_name, link_target, link_title))`
/// Recognizes hyperlinks in Markdown or RestructuredText
/// format. Anonymous links in RestructuredText are not supported.
pub fn first_hyperlink(i: &str) -> Option<(String, String, String)> {
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

        let i = r#"[md link name]: md_link_target "md link title"
abc [md link name](md_link_target "md link title")abc
   [md link name]: md_link_target "md link title"[nomd]: no[nomd]: no
abc`rst link name <rst_link_target>`_abc
abc`rst link name <rst_link_target>`_ .. _norst: no .. _norst: no
.. _rst link name: rst_link_target
  .. _rst link name: rst_link_t
     arget
"#;

        let expected = (
            "md link name".to_string(),
            "md_link_target".to_string(),
            "md link title".to_string(),
        );
        let (i, res) = take_hyperlink(i).unwrap();
        assert_eq!(res, expected);
        let (i, res) = take_hyperlink(i).unwrap();
        assert_eq!(res, expected);
        let (i, res) = take_hyperlink(i).unwrap();
        assert_eq!(res, expected);

        let expected = (
            "rst link name".to_string(),
            "rst_link_target".to_string(),
            "".to_string(),
        );
        let (i, res) = take_hyperlink(i).unwrap();
        assert_eq!(res, expected);
        let (i, res) = take_hyperlink(i).unwrap();
        assert_eq!(res, expected);
        let (i, res) = take_hyperlink(i).unwrap();
        assert_eq!(res, expected);
        let (_, res) = take_hyperlink(i).unwrap();
        assert_eq!(res, expected);

        let i = " .. _`My: home page`: http://getreu.net\nabc";
        let expected = (
            "My: home page".to_string(),
            "http://getreu.net".to_string(),
            "".to_string(),
        );
        let (_, res) = take_hyperlink(i).unwrap();
        assert_eq!(res, expected);
    }

    #[test]
    fn test_first_hyperlink() {
        let i = "abc\n   [md link name]: md_link_target \"md link title\"abc";

        let expected = (
            "md link name".to_string(),
            "md_link_target".to_string(),
            "md link title".to_string(),
        );
        let res = first_hyperlink(i).unwrap();
        assert_eq!(res, expected);

        let err = first_hyperlink("no link here");
        assert_eq!(err, None);

        let err = first_hyperlink("");
        assert_eq!(err, None);
    }
}
