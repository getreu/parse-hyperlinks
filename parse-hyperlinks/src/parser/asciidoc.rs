//! This module implements parsers for Asciidoc hyperlinks.
#![allow(dead_code)]
#![allow(clippy::type_complexity)]

use crate::parser::parse::LABEL_LEN_MAX;
use crate::parser::Link;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::bytes::complete::tag_no_case;
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
/// * the preceding byte is a newline `\n` _or_
/// * the preceding bytes are whitespaces _or_
/// * the preceding bytes are whitespaces or newline, followed by one of `[(<`
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
///   adoc_text2dest("https://destination[name]abc"),
///   Ok(("abc", (Cow::from("name"), Cow::from("https://destination"), Cow::from(""))))
/// );
/// assert_eq!(
///   adoc_text2dest("https://destination[]abc"),
///   Ok(("abc", (Cow::from("https://destination"), Cow::from("https://destination"), Cow::from(""))))
/// );
/// assert_eq!(
///   adoc_text2dest("https://destination abc"),
///   Ok((" abc", (Cow::from("https://destination"), Cow::from("https://destination"), Cow::from(""))))
/// );
/// ```
pub fn adoc_text2dest(i: &str) -> nom::IResult<&str, (Cow<str>, Cow<str>, Cow<str>)> {
    let (i, (link_destination, link_text)) = nom::sequence::preceded(
        space0,
        nom::sequence::pair(
            adoc_inline_link_destination,
            nom::combinator::opt(adoc_link_text),
        ),
    )(i)?;

    let link_text = if let Some(lt) = link_text {
        if lt.is_empty() {
            link_destination.clone()
        } else {
            lt
        }
    } else {
        link_destination.clone()
    };

    Ok((i, (link_text, link_destination, Cow::Borrowed(""))))
}

/// Wrapper around `adoc_label2dest()` that packs the result in
/// `Link::Label2Dest`.
pub fn adoc_label2dest_link(i: &str) -> nom::IResult<&str, Link> {
    let (i, (te, de, ti)) = adoc_label2dest(i)?;
    Ok((i, Link::Label2Dest(te, de, ti)))
}

/// Parses an Asciidoc _link reference definition_.
///
/// This parser expects to start at the first letter of `:`,
/// ` `, or `\t` to succeed.
///
/// The caller must guarantee, that:
/// * the parser is at the beginning of the input _or_
/// * the preceding byte is a newline `\n`.
///
/// `link_label` is always of type `Cow::Borrowed(&str)`.
/// `link_title` is always the empty `Cow::Borrowed("")`.
/// ```
/// use parse_hyperlinks::parser::Link;
/// use parse_hyperlinks::parser::asciidoc::adoc_label2dest;
/// use std::borrow::Cow;
///
/// assert_eq!(
///   adoc_label2dest(":label: https://destination\nabc"),
///   Ok(("\nabc", (Cow::from("label"), Cow::from("https://destination"), Cow::from(""))))
/// );
/// ```
pub fn adoc_label2dest(i: &str) -> nom::IResult<&str, (Cow<str>, Cow<str>, Cow<str>)> {
    let (i, (link_label, link_destination)) = nom::sequence::preceded(
        space0,
        nom::sequence::pair(
            adoc_parse_colon_reference,
            nom::sequence::delimited(
                nom::character::complete::space1,
                adoc_link_reference_definition_destination,
                nom::character::complete::space0,
            ),
        ),
    )(i)?;

    if !i.is_empty() {
        let _ = peek::<&str, _, nom::error::Error<_>, _>(nom::character::complete::newline)(i)?;
    };

    Ok((
        i,
        (
            Cow::Borrowed(link_label),
            link_destination,
            Cow::Borrowed(""),
        ),
    ))
}

/// Wrapper around `adoc_text2label()` that packs the result in
/// `Link::Text2Label`.
pub fn adoc_text2label_link(i: &str) -> nom::IResult<&str, Link> {
    let (i, (te, la)) = adoc_text2label(i)?;
    Ok((i, Link::Text2Label(te, la)))
}

/// Parse a Asciidoc _reference link_.
///
/// There are three kinds of reference links `Text2Label`: full, collapsed, and
/// shortcut.
/// 1. A full reference link `{label}[text]` consists of a link label immediately
/// followed by a link text. The label matches a link reference definition
/// elsewhere in the document.
/// 2. A collapsed reference link `{label}[]` consists of a link label that matches
///    a link reference definition elsewhere in the document, followed by the string
///    `[]`. In this case, the function returns an empty _link text_ `""`,
///    indicating, that the empty string must be replaced later by the link
///    destination `link_dest` of the matching _link reference definition_
///    (`Label2Dest`).
/// 3. A shortcut reference link consists of a link label that matches a link
///    reference definition elsewhere in the document and is not followed by `[]` or
///    a link text `[link text]`. This is a shortcut of case 2. above.
///
/// This parser expects to start at the beginning of the link `[` to succeed.
/// It should always run at last position after all other parsers.
/// ```rust
/// use parse_hyperlinks::parser::Link;
/// use parse_hyperlinks::parser::asciidoc::adoc_text2label;
/// use std::borrow::Cow;
///
/// assert_eq!(
///   adoc_text2label("{link-label}[link text]abc"),
///   Ok(("abc", (Cow::from("link text"), Cow::from("link-label"))))
/// );
/// assert_eq!(
///   adoc_text2label("{link-label}[]abc"),
///   Ok(("abc", (Cow::from(""), Cow::from("link-label"))))
/// );
/// assert_eq!(
///   adoc_text2label("{link-label}abc"),
///   Ok(("abc", (Cow::from(""), Cow::from("link-label"))))
/// );
/// ```
pub fn adoc_text2label(i: &str) -> nom::IResult<&str, (Cow<str>, Cow<str>)> {
    let (i, (link_label, link_text)) = alt((
        nom::sequence::pair(adoc_parse_curly_bracket_reference, adoc_link_text),
        nom::combinator::map(adoc_parse_curly_bracket_reference, |s| (s, Cow::from(""))),
    ))(i)?;

    // Check that there is no `[` or `{` following. Do not consume.
    if !i.is_empty() {
        let _ = nom::character::complete::none_of("[{")(i)?;
    }

    Ok((i, (link_text, link_label)))
}

/// Parses the link label. To succeed the first letter must be `[` and the
/// last letter `]`. A sequence of whitespaces including newlines, will be
/// replaced by one space. There must be not contain more than one newline
/// per sequence. The string can contain the `\]` which is replaced by `]`.
fn adoc_link_text(i: &str) -> nom::IResult<&str, Cow<str>> {
    nom::sequence::delimited(char('['), remove_newline_take_till(']'), char(']'))(i)
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
        while !j.is_empty() {
            // `till()` always succeeds. There are two situations, when it does not
            // advance the parser:
            // 1. Input is the empty string `""`.
            // 2. The first character satisfy the condition of `take_till()`.
            //
            // Case 1.: Can not happen because of the `while` just before.
            // Case 2.: Even if the parser does not advance here, the code below
            // starting with `if let Ok...` it will advance the parser at least
            // one character.
            let (k, s1) =
                nom::bytes::complete::take_till(|c| c == pat || c == '\n' || c == '\\')(j)?;

            // Store the result.
            res = match res {
                Cow::Borrowed("") => Cow::Borrowed(s1),
                Cow::Borrowed(res_str) => {
                    let mut strg = res_str.to_string();
                    strg.push_str(s1);
                    Cow::Owned(strg)
                }
                Cow::Owned(mut strg) => {
                    strg.push_str(s1);
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

/// Parses an link reference definition destination.
/// The parser takes URLs until `[`, whitespace or newline.
/// The parser succeeds, if one of the variants:
/// `adoc_parse_http_link_destination()` or
/// `adoc_parse_escaped_link_destination()` succeeds and returns its result.
fn adoc_link_reference_definition_destination(i: &str) -> nom::IResult<&str, Cow<str>> {
    alt((
        adoc_parse_http_link_destination,
        adoc_parse_escaped_link_destination,
    ))(i)
}

/// Parses an inline link destination.
/// The parser succeeds, if one of the variants:
/// `adoc_parse_http_link_destination()`, `adoc_parse_literal_link_destination()`
/// or `adoc_parse_escaped_link_destination()` succeeds and returns its result.
fn adoc_inline_link_destination(i: &str) -> nom::IResult<&str, Cow<str>> {
    alt((
        adoc_parse_http_link_destination,
        adoc_parse_literal_link_destination,
        adoc_parse_escaped_link_destination,
    ))(i)
}

/// Parses a link destination in URL form starting with `http://` or `https://`
/// and ending with `[`. The latter is peeked, but no consumed.
fn adoc_parse_http_link_destination(i: &str) -> nom::IResult<&str, Cow<str>> {
    let (j, s) = nom::sequence::preceded(
        peek(alt((tag_no_case("http://"), (tag_no_case("https://"))))),
        nom::bytes::complete::take_till1(|c| c == '[' || c == ' ' || c == '\t' || c == '\n'),
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
/// with `]`, whitespace or newline. The later is peeked, but not consumed. The URL can contain percent
/// encoded characters, which are decoded.
fn adoc_parse_escaped_link_destination(i: &str) -> nom::IResult<&str, Cow<str>> {
    nom::combinator::map_parser(
        nom::sequence::preceded(
            nom::sequence::pair(
                tag("link:"),
                peek(alt((tag_no_case("http://"), (tag_no_case("https://"))))),
            ),
            nom::bytes::complete::take_till1(|c| {
                c == '[' || c == ' ' || c == '\t' || c == '\r' || c == '\n'
            }),
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

/// Parses the _link text_ (`label`) of `Label2Text` link.
///
/// The parser expects to start at the opening `{` to succeed.
/// The result is always a borrowed reference.
fn adoc_parse_curly_bracket_reference(i: &str) -> nom::IResult<&str, Cow<str>> {
    nom::combinator::map(
        nom::combinator::verify(
            nom::sequence::delimited(
                char('{'),
                nom::bytes::complete::take_till1(|c| {
                    c == '}' || c == ' ' || c == '\t' || c == '\r'
                }),
                char('}'),
            ),
            |s: &str| s.len() <= LABEL_LEN_MAX,
        ),
        Cow::Borrowed,
    )(i)
}

/// Parses the label of a link reference definition.
///
/// The parser expects to start at the first colon `:` or at some whitespace to
/// succeed.
/// The caller must guaranty, that the byte before was a newline. The parser
/// consumes all whitespace before the first colon and after the second.
fn adoc_parse_colon_reference(i: &str) -> nom::IResult<&str, &str> {
    nom::combinator::verify(
        nom::sequence::delimited(
            char(':'),
            nom::bytes::complete::take_till1(|c| c == ':' || c == ' ' || c == '\t' || c == '\r'),
            char(':'),
        ),
        |s: &str| s.len() <= LABEL_LEN_MAX,
    )(i)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom::error::ErrorKind;
    use std::matches;

    #[test]
    fn test_adoc_text2dest() {
        assert_eq!(
            adoc_text2dest("http://getreu.net[]"),
            Ok((
                "",
                (
                    Cow::from("http://getreu.net"),
                    Cow::from("http://getreu.net"),
                    Cow::from("")
                )
            ))
        );

        assert_eq!(
            adoc_text2dest("http://getreu.net[]abc"),
            Ok((
                "abc",
                (
                    Cow::from("http://getreu.net"),
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
    fn test_adoc_label2dest() {
        assert_eq!(
            adoc_label2dest(":label: http://getreu.net\n"),
            Ok((
                "\n",
                (
                    Cow::from("label"),
                    Cow::from("http://getreu.net"),
                    Cow::from("")
                )
            ))
        );

        assert_eq!(
            adoc_label2dest("  :label: \thttp://getreu.net \t "),
            Ok((
                "",
                (
                    Cow::from("label"),
                    Cow::from("http://getreu.net"),
                    Cow::from("")
                )
            ))
        );

        assert_eq!(
            adoc_label2dest("  :label: \thttp://getreu.net \t abc").unwrap_err(),
            nom::Err::Error(nom::error::Error::new("abc", ErrorKind::Char))
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
        assert!(matches!(res.1, Cow::Borrowed { .. }));

        let res = remove_newline_take_till(']')("text text]abc").unwrap();
        assert_eq!(res, ("]abc", Cow::from("text text")));
        assert!(matches!(res.1, Cow::Borrowed { .. }));

        let res = remove_newline_take_till(']')("text text").unwrap();
        assert_eq!(res, ("", Cow::from("text text")));
        assert!(matches!(res.1, Cow::Borrowed { .. }));

        let res = remove_newline_take_till(']')(r#"te\]xt]abc"#).unwrap();
        assert_eq!(res, ("]abc", Cow::from("te]xt")));
        assert!(matches!(res.1, Cow::Owned { .. }));

        let res = remove_newline_take_till(']')(r#"text\]]abc"#).unwrap();
        assert_eq!(res, ("]abc", Cow::from("text]")));
        assert!(matches!(res.1, Cow::Owned { .. }));

        let res = remove_newline_take_till(']')(r#"te\xt]abc"#).unwrap();
        assert_eq!(res, ("]abc", Cow::from(r#"te\xt"#)));
        assert!(matches!(res.1, Cow::Owned { .. }));

        let res = remove_newline_take_till(']')("text\n   text]abc").unwrap();
        assert_eq!(res, ("]abc", Cow::from("text text")));
        assert!(matches!(res.1, Cow::Owned { .. }));

        let res = remove_newline_take_till(']')("text\n   text]abc").unwrap();
        assert_eq!(res, ("]abc", Cow::from("text text")));
        assert!(matches!(res.1, Cow::Owned { .. }));

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
    fn test_adoc_parse_http_link_destination() {
        let res = adoc_parse_http_link_destination("http://destination/").unwrap();
        assert_eq!(res, ("", Cow::from("http://destination/")));
        assert!(matches!(res.1, Cow::Borrowed { .. }));

        let res = adoc_parse_http_link_destination("http://destination/\nabc").unwrap();
        assert_eq!(res, ("\nabc", Cow::from("http://destination/")));
        assert!(matches!(res.1, Cow::Borrowed { .. }));

        let res = adoc_parse_http_link_destination("http://destination/ abc").unwrap();
        assert_eq!(res, (" abc", Cow::from("http://destination/")));
        assert!(matches!(res.1, Cow::Borrowed { .. }));

        let res = adoc_parse_http_link_destination("http://destination/[abc").unwrap();
        assert_eq!(res, ("[abc", Cow::from("http://destination/")));
        assert!(matches!(res.1, Cow::Borrowed { .. }));

        let res = adoc_parse_http_link_destination("https://destination/[abc").unwrap();
        assert_eq!(res, ("[abc", Cow::from("https://destination/")));
        assert!(matches!(res.1, Cow::Borrowed { .. }));

        assert_eq!(
            adoc_parse_http_link_destination("http:/destination/[abc").unwrap_err(),
            nom::Err::Error(nom::error::Error::new(
                "http:/destination/[abc",
                ErrorKind::Tag
            ))
        );
    }

    #[test]
    fn test_adoc_parse_escaped_link_destination() {
        let res = adoc_parse_escaped_link_destination("link:http://destination/").unwrap();
        assert_eq!(res, ("", Cow::from("http://destination/")));
        assert!(matches!(res.1, Cow::Borrowed { .. }));

        let res = adoc_parse_escaped_link_destination("link:http://destination/[abc").unwrap();
        assert_eq!(res, ("[abc", Cow::from("http://destination/")));
        assert!(matches!(res.1, Cow::Borrowed { .. }));

        let res = adoc_parse_escaped_link_destination("link:http://destination/ abc").unwrap();
        assert_eq!(res, (" abc", Cow::from("http://destination/")));
        assert!(matches!(res.1, Cow::Borrowed { .. }));

        let res = adoc_parse_escaped_link_destination("link:http://destination/\nabc").unwrap();
        assert_eq!(res, ("\nabc", Cow::from("http://destination/")));
        assert!(matches!(res.1, Cow::Borrowed { .. }));

        assert_eq!(
            adoc_parse_escaped_link_destination("link:httpX:/destination/[abc").unwrap_err(),
            nom::Err::Error(nom::error::Error::new(
                "httpX:/destination/[abc",
                ErrorKind::Tag
            ))
        );

        let res = adoc_parse_escaped_link_destination("link:https://getreu.net/?q=%5Ba%20b%5D[abc")
            .unwrap();
        assert_eq!(res, ("[abc", Cow::from("https://getreu.net/?q=[a b]")));
        assert!(matches!(res.1, Cow::Owned { .. }));

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

    #[test]
    fn test_adoc_text2label() {
        let res = adoc_text2label("{label}[link text]abc").unwrap();
        assert_eq!(res, ("abc", (Cow::from("link text"), Cow::from("label"))));

        let res = adoc_text2label("{label}[]abc").unwrap();
        assert_eq!(res, ("abc", (Cow::from(""), Cow::from("label"))));

        let res = adoc_text2label("{label}abc").unwrap();
        assert_eq!(res, ("abc", (Cow::from(""), Cow::from("label"))));

        let res = adoc_text2label("{label}").unwrap();
        assert_eq!(res, ("", (Cow::from(""), Cow::from("label"))));

        let res = adoc_text2label("{label} [link text]abc").unwrap();
        assert_eq!(
            res,
            (" [link text]abc", (Cow::from(""), Cow::from("label")))
        );

        assert_eq!(
            adoc_text2label("{label}[abc").unwrap_err(),
            nom::Err::Error(nom::error::Error::new("[abc", ErrorKind::NoneOf))
        );
    }

    #[test]
    fn test_adoc_parse_curly_bracket_reference() {
        let res = adoc_parse_curly_bracket_reference("{label}").unwrap();
        assert_eq!(res, ("", Cow::from("label")));

        let res = adoc_parse_curly_bracket_reference("{label}[link text]").unwrap();
        assert_eq!(res, ("[link text]", Cow::from("label")));

        assert_eq!(
            adoc_parse_curly_bracket_reference("").unwrap_err(),
            nom::Err::Error(nom::error::Error::new("", ErrorKind::Char))
        );

        assert_eq!(
            adoc_parse_curly_bracket_reference("{label }").unwrap_err(),
            nom::Err::Error(nom::error::Error::new(" }", ErrorKind::Char))
        );
        assert_eq!(
            adoc_parse_curly_bracket_reference("").unwrap_err(),
            nom::Err::Error(nom::error::Error::new("", ErrorKind::Char))
        );
    }

    #[test]
    fn test_adoc_parse_colon_reference() {
        let res = adoc_parse_colon_reference(":label:abc").unwrap();
        assert_eq!(res, ("abc", "label"));

        assert_eq!(
            adoc_parse_colon_reference(":label abc").unwrap_err(),
            nom::Err::Error(nom::error::Error::new(" abc", ErrorKind::Char))
        );
    }
}
