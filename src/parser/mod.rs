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
pub fn take_hyperlink(mut i: &str) -> nom::IResult<&str, (String, String, String)> {
    let len = i.len();
    loop {
        // This might not consume bytes and never fails.
        let (j, _) = take_till(|c| c == '\n' || c == '`' || c == '[')(i)?;
        i = j;

        // Here we exit when there is no input left.
        let (_, current_char) = peek(anychar)(i)?;

        // Are we on a new line character?
        if current_char == '\n' {
            // Consume the `\n`.
            // Advance one character.
            let (j, _) = anychar(i)?;
            i = j;
        };

        // Here it is worth to have a look.

        // Are we at the beginning of a line?
        if current_char == '\n' || i.len() == len {
            if let Ok(r) = alt((
                map(md_link_ref, |(ln, lta, lti)| {
                    (ln.to_string(), lta.to_string(), lti.to_string())
                }),
                map(rst_link_ref, |(ln, lt)| (ln, lt, "".to_string())),
            ))(i)
            {
                return Ok(r);
            };
        };

        // Regular links can start everywhere.
        if let Ok(r) = alt((
            map(rst_link, |(ln, lt)| (ln, lt, "".to_string())),
            map(md_link, |(ln, lta, lti)| {
                (ln.to_string(), lta.to_string(), lti.to_string())
            }),
        ))(i)
        {
            return Ok(r);
        };

        // This makes sure that we advance.
        let (j, _) = anychar(i)?;
        i = j;
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
   [md link name]: md_link_target "md link title"
abc`rst link name <rst_link_target>`_abc
abc`rst link name <rst_link_target>`_abc
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
