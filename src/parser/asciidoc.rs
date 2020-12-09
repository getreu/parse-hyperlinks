//! This module implements parsers for Asciidoc hyperlinks.
#![allow(dead_code)]

use crate::parser::Link;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::char;
use nom::character::complete::space0;
use nom::combinator::peek;
use nom::error::ErrorKind;
use percent_encoding::percent_decode_str;
use std::borrow::Cow;

/// Wrapper around `adoc_text2dest()` that packs the result in
/// `Link::Text2Dest`.
pub fn adoc_text2dest_link(i: &str) -> nom::IResult<&str, Link> {
    let (i, (te, de, ti)) = adoc_text2dest(i)?;
    Ok((i, Link::Text2Dest(te, de, ti)))
}

/// Parses an Asciidoc _inline link_.
///
/// This parser expects to start at the first letter of `http://`,
/// `https://`, `link:http://` or `link:https://` (preceded by optional
/// whitespaces) to succeed.
///
/// When it starts at the letter `h` or `l`, the caller must guarantee, that:
/// * the parser is at the beginning of the input _or_
/// * the preceding byte is a newline `\n`.
///
/// When ist starts at a whitespace no further guarantee is required.
///
/// `link_title` is always the empty `Cow::Borrowed("")`.
/// ```
/// use parse_hyperlinks::parser::Link;
/// use parse_hyperlinks::parser::asciidoc::adoc_text2dest;
/// use std::borrow::Cow;
///
/// assert_eq!(
///   adoc_text2dest(r#"https://destination[name]abc"#),
///   Ok(("abc", (Cow::from("name"), Cow::from("https://destination"), Cow::from(""))))
/// );
/// ```
pub fn adoc_text2dest(i: &str) -> nom::IResult<&str, (Cow<str>, Cow<str>, Cow<str>)> {
    let (i, (link_destination, link_text)) = nom::sequence::preceded(
        space0,
        nom::sequence::pair(adoc_link_destination, adoc_link_text),
    )(i)?;
    Ok((i, (link_text, link_destination, Cow::Borrowed(""))))
}

/// Parses the link name. To succeed the first letter must be `[` and the
/// last letter `]`. A sequence of whitespaces including newlines, will be
/// replaced by one space. There must be not contain more than one newline
/// per sequence. The string can contain the `\]` which is replaced by `]`.
fn adoc_link_text(i: &str) -> nom::IResult<&str, Cow<str>> {
    let (i, r) = nom::sequence::preceded(tag("["), remove_newline_take_till(']'))(i)?;
    // Consume the closing `]`.
    let (i, _) = char(']')(i)?;
    Ok((i, r))
}

/// Takes all characters until the character `<pat>`. The escaped character
/// `\<pat>` is taken as normal character. Then parser replaces the escaped character
/// `\<pat>` with `<pat>`. A sequence of whitespaces including one newline, is
/// replaced by one space ` `. Each sequence must not contain more than one
/// newline.
fn remove_newline_take_till<'a>(
    pat: char,
) -> impl Fn(&'a str) -> nom::IResult<&'a str, Cow<'a, str>> {
    move |i: &str| {
        let mut res = Cow::Borrowed("");
        let mut j = i;
        while j != "" {
            // `till()` always succeeds. There are two situations, when it does not
            // advance the parser:
            // 1. Input is the empty string `""`.
            // 2. The first character satisfy the condition of `take_till()`.
            //
            // Case 1.: Can not happen because of the `while` just before.
            // Case 2.: Even if the parser does not advance here, the code below
            // starting with `if let Ok...` it will advance the parser at least
            // one character.
            let (k, s) =
                nom::bytes::complete::take_till(|c| c == pat || c == '\n' || c == '\\')(j)?;

            // Store the result.
            res = match res {
                Cow::Borrowed("") => Cow::Borrowed(s),
                Cow::Borrowed(res_str) => {
                    let mut strg = res_str.to_string();
                    strg.push_str(s);
                    Cow::Owned(strg)
                }
                Cow::Owned(mut strg) => {
                    strg.push_str(s);
                    Cow::Owned(strg)
                }
            };

            // If there is a character left, inspect. Then either quit or advance at least one character.
            // Therefor no endless is loop possible.
            if let (_, Some(c)) =
                nom::combinator::opt(nom::combinator::peek(nom::character::complete::anychar))(k)?
            {
                let m = match c {
                    // We completed our mission and found `pat`.
                    // This is the only Ok exit from the while loop.
                    c if c == pat => return Ok((k, res)),
                    // We stopped at an escaped character.
                    c if c == '\\' => {
                        // Consume the escape `\`.
                        let (l, _) = char('\\')(k)?;
                        // `pat` is the only valid escaped character (not even `\\` is special in
                        // Asciidoc).
                        // If `<pat>` is found, `c=='<pat>'`, otherwise `c=='\\'`
                        let (l, c) = alt((char(pat), nom::combinator::success('\\')))(l)?;

                        // and append the escaped character to `res`.
                        let mut strg = res.to_string();
                        strg.push(c);
                        // Store the result.
                        res = Cow::Owned(strg);
                        // Advance `k`.
                        l
                    }
                    // We stopped at a newline.
                    c if c == '\n' => {
                        // Now consume the `\n`.
                        let (l, _) = char('\n')(k)?;
                        let (l, _) = space0(l)?;
                        // Return error if there is one more `\n`. BTW, `not()` never consumes.
                        let _ = nom::combinator::not(char('\n'))(l)?;

                        // and append one space ` ` character to `res`.
                        let mut strg = res.to_string();
                        strg.push(' ');
                        // Store the result.
                        res = Cow::Owned(strg);
                        // Advance `k`.
                        l
                    }
                    _ => unreachable!(),
                };
                j = m;
            } else {
                // We are here because `k == ""`. We quit the while loop.
                j = k;
            }
        }

        // If we are here, `j` is empty `""`.
        Ok(("", res))
    }
}

/// Parses a link destination.
/// The parser succeeds, if one of the variants:
/// `adoc_parse_http_link_destination()`, `adoc_parse_literal_link_destination()`
/// or `adoc_parse_escaped_link_destination()` succeeds and returns its result.
fn adoc_link_destination(i: &str) -> nom::IResult<&str, Cow<str>> {
    alt((
        adoc_parse_http_link_destination,
        adoc_parse_literal_link_destination,
        adoc_parse_escaped_link_destination,
    ))(i)
}

/// Parses a link destination in URL form starting with `http://` or `https://`
/// and ending with `[`. The latter is peeked, but no consumed.
fn adoc_parse_http_link_destination(i: &str) -> nom::IResult<&str, Cow<str>> {
    let (j, s) = nom::sequence::delimited(
        peek(alt((tag("http://"), (tag("https://"))))),
        nom::bytes::complete::take_till1(|c| {
            c == '[' || c == ' ' || c == '\t' || c == '\r' || c == '\n'
        }),
        peek(char('[')),
    )(i)?;
    Ok((j, Cow::Borrowed(s)))
}

/// A parser that decodes percent encoded URLS.
/// Fails when the percent codes can not be mapped to valid UTF8.
/// ```text
/// use std::borrow::Cow;
///
/// let res = percent_decode("https://getreu.net/?q=%5Ba%20b%5D").unwrap();
/// assert_eq!(res, ("", Cow::Owned("https://getreu.net/?q=[a b]".to_string())));
///```
fn percent_decode(i: &str) -> nom::IResult<&str, Cow<str>> {
    let decoded = percent_decode_str(i)
        .decode_utf8()
        .map_err(|_| nom::Err::Error(nom::error::Error::new(i, ErrorKind::EscapedTransform)))?;
    Ok(("", decoded))
}

/// Parses a link destination starting with `link:http://` or `link:https://` ending
/// with `]`. The later is peeked, but not consumed. The URL can contain percent
/// encoded characters, which are decoded.
fn adoc_parse_escaped_link_destination(i: &str) -> nom::IResult<&str, Cow<str>> {
    nom::combinator::map_parser(
        nom::sequence::delimited(
            nom::sequence::pair(tag("link:"), peek(alt((tag("http://"), (tag("https://")))))),
            nom::bytes::complete::take_till1(|c| {
                c == '[' || c == ' ' || c == '\t' || c == '\r' || c == '\n'
            }),
            peek(char('[')),
        ),
        percent_decode,
    )(i)
}

/// Parses a link destination starting with `link:+++` ending with `++`. Everything in
/// between is taken as it is without any transformation.
fn adoc_parse_literal_link_destination(i: &str) -> nom::IResult<&str, Cow<str>> {
    let (j, s) = nom::sequence::preceded(
        tag("link:"),
        nom::sequence::delimited(tag("++"), nom::bytes::complete::take_until("++"), tag("++")),
    )(i)?;
    Ok((j, Cow::Borrowed(s)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom::error::ErrorKind;
    use std::matches;

    #[test]
    fn test_adoc_text2dest() {
        assert_eq!(
            adoc_text2dest("http://getreu.net[My blog]abc"),
            Ok((
                "abc",
                (
                    Cow::from("My blog"),
                    Cow::from("http://getreu.net"),
                    Cow::from("")
                )
            ))
        );

        assert_eq!(
            adoc_text2dest("  \t  http://getreu.net[My blog]abc"),
            Ok((
                "abc",
                (
                    Cow::from("My blog"),
                    Cow::from("http://getreu.net"),
                    Cow::from("")
                )
            ))
        );

        assert_eq!(
            adoc_text2dest(r#"http://getreu.net[My blog[1\]]abc"#),
            Ok((
                "abc",
                (
                    Cow::from("My blog[1]"),
                    Cow::from("http://getreu.net"),
                    Cow::from("")
                )
            ))
        );

        assert_eq!(
            adoc_text2dest("http://getreu.net[My\n    blog]abc"),
            Ok((
                "abc",
                (
                    Cow::from("My blog"),
                    Cow::from("http://getreu.net"),
                    Cow::from("")
                )
            ))
        );

        assert_eq!(
            adoc_text2dest("link:http://getreu.net[My blog]abc"),
            Ok((
                "abc",
                (
                    Cow::from("My blog"),
                    Cow::from("http://getreu.net"),
                    Cow::from("")
                )
            ))
        );

        assert_eq!(
            adoc_text2dest("link:https://getreu.net/?q=%5Ba%20b%5D[My blog]abc"),
            Ok((
                "abc",
                (
                    Cow::from("My blog"),
                    Cow::from("https://getreu.net/?q=[a b]"),
                    Cow::from("")
                )
            ))
        );

        assert_eq!(
            adoc_text2dest("link:++https://getreu.net/?q=[a b]++[My blog]abc"),
            Ok((
                "abc",
                (
                    Cow::from("My blog"),
                    Cow::from("https://getreu.net/?q=[a b]"),
                    Cow::from("")
                )
            ))
        );
    }

    #[test]
    fn test_adoc_link_text() {
        assert_eq!(adoc_link_text("[text]abc"), Ok(("abc", Cow::from("text"))));

        assert_eq!(
            adoc_link_text("[te\nxt]abc"),
            Ok(("abc", Cow::from("te xt")))
        );

        assert_eq!(
            adoc_link_text("[te\n\nxt]abc"),
            Err(nom::Err::Error(nom::error::Error::new(
                "\nxt]abc",
                ErrorKind::Not
            )))
        );

        assert_eq!(
            adoc_link_text(r#"[text[i\]]abc"#),
            Ok(("abc", Cow::from(r#"text[i]"#.to_string())))
        );

        assert_eq!(
            adoc_link_text("[textabc"),
            Err(nom::Err::Error(nom::error::Error::new("", ErrorKind::Char)))
        );
    }

    #[test]
    fn test_remove_newline_take_till() {
        let res = remove_newline_take_till(']')("").unwrap();
        assert_eq!(res, ("", Cow::from("")));
        assert!(matches!(res.1,
            Cow::Borrowed{..}
        ));

        let res = remove_newline_take_till(']')("text text]abc").unwrap();
        assert_eq!(res, ("]abc", Cow::from("text text")));
        assert!(matches!(res.1,
            Cow::Borrowed{..}
        ));

        let res = remove_newline_take_till(']')("text text").unwrap();
        assert_eq!(res, ("", Cow::from("text text")));
        assert!(matches!(res.1,
            Cow::Borrowed{..}
        ));

        let res = remove_newline_take_till(']')(r#"te\]xt]abc"#).unwrap();
        assert_eq!(res, ("]abc", Cow::from("te]xt")));
        assert!(matches!(res.1,
            Cow::Owned{..}
        ));

        let res = remove_newline_take_till(']')(r#"text\]]abc"#).unwrap();
        assert_eq!(res, ("]abc", Cow::from("text]")));
        assert!(matches!(res.1,
            Cow::Owned{..}
        ));

        let res = remove_newline_take_till(']')(r#"te\xt]abc"#).unwrap();
        assert_eq!(res, ("]abc", Cow::from(r#"te\xt"#)));
        assert!(matches!(res.1,
            Cow::Owned{..}
        ));

        let res = remove_newline_take_till(']')("text\n   text]abc").unwrap();
        assert_eq!(res, ("]abc", Cow::from("text text")));
        assert!(matches!(res.1,
            Cow::Owned{..}
        ));

        let res = remove_newline_take_till(']')("text\n   text]abc").unwrap();
        assert_eq!(res, ("]abc", Cow::from("text text")));
        assert!(matches!(res.1,
            Cow::Owned{..}
        ));

        assert_eq!(
            remove_newline_take_till(']')("text\n\ntext]abc").unwrap_err(),
            nom::Err::Error(nom::error::Error::new("\ntext]abc", ErrorKind::Not))
        );

        assert_eq!(
            remove_newline_take_till(']')("text\n  \n  text]abc").unwrap_err(),
            nom::Err::Error(nom::error::Error::new("\n  text]abc", ErrorKind::Not))
        );
    }

    #[test]
    fn test_adoc_parse_html_link_destination() {
        let res = adoc_parse_http_link_destination("http://destination/[abc").unwrap();
        assert_eq!(res, ("[abc", Cow::from("http://destination/")));
        assert!(matches!(res.1,
            Cow::Borrowed{..}
        ));

        let res = adoc_parse_http_link_destination("https://destination/[abc").unwrap();
        assert_eq!(res, ("[abc", Cow::from("https://destination/")));
        assert!(matches!(res.1,
            Cow::Borrowed{..}
        ));

        assert_eq!(
            adoc_parse_http_link_destination("http:/destination/[abc").unwrap_err(),
            nom::Err::Error(nom::error::Error::new(
                "http:/destination/[abc",
                ErrorKind::Tag
            ))
        );

        assert_eq!(
            adoc_parse_http_link_destination("http://destination/(abc").unwrap_err(),
            nom::Err::Error(nom::error::Error::new("", ErrorKind::Char))
        );
    }

    #[test]
    fn test_adoc_parse_escaped_link_destination() {
        let res = adoc_parse_escaped_link_destination("link:http://destination/[abc").unwrap();
        assert_eq!(res, ("[abc", Cow::from("http://destination/")));
        assert!(matches!(res.1,
            Cow::Borrowed{..}
        ));

        assert_eq!(
            adoc_parse_escaped_link_destination("link:httpX:/destination/[abc").unwrap_err(),
            nom::Err::Error(nom::error::Error::new(
                "httpX:/destination/[abc",
                ErrorKind::Tag
            ))
        );

        assert_eq!(
            adoc_link_destination("http://destination/(abc").unwrap_err(),
            nom::Err::Error(nom::error::Error::new(
                "http://destination/(abc",
                ErrorKind::Tag
            ))
        );

        let res = adoc_parse_escaped_link_destination("link:https://getreu.net/?q=%5Ba%20b%5D[abc")
            .unwrap();
        assert_eq!(res, ("[abc", Cow::from("https://getreu.net/?q=[a b]")));
        assert!(matches!(res.1,
            Cow::Owned{..}
        ));

        assert_eq!(
            adoc_parse_escaped_link_destination("link:https://getreu.net/?q=%FF%FF[abc")
                .unwrap_err(),
            nom::Err::Error(nom::error::Error::new(
                "https://getreu.net/?q=%FF%FF",
                ErrorKind::EscapedTransform
            ))
        );
    }

    #[test]
    fn test_adoc_parse_literal_link_destination() {
        let res = adoc_parse_literal_link_destination("link:++https://getreu.net/?q=[a b]++[abc")
            .unwrap();
        assert_eq!(res, ("[abc", Cow::from("https://getreu.net/?q=[a b]")));

        assert_eq!(
            adoc_parse_literal_link_destination("link:++https://getreu.net/?q=[a b]+[abc")
                .unwrap_err(),
            nom::Err::Error(nom::error::Error::new(
                "https://getreu.net/?q=[a b]+[abc",
                ErrorKind::TakeUntil
            ))
        );
    }
}
