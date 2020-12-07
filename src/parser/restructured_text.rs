//! This module implements parsers for RestructuredText hyperlinks.
#![allow(dead_code)]

use crate::parser::Link;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::*;
use nom::IResult;
use std::borrow::Cow;

/// Character that can be escaped with `\`.
const ESCAPABLE: &str = r#" `:<>_\"#;

/// Wrapper around `rst_text2dest()` that packs the result in
/// `Link::Text2Dest`.
pub fn rst_text2dest_link(i: &str) -> nom::IResult<&str, Link> {
    let (i, (te, de, ti)) = rst_text2dest(i)?;
    Ok((i, Link::Text2Dest(te, de, ti)))
}

/// Parse a RestructuredText _inline hyperlink_.
///
/// The parser expects to start at the link start (\`) to succeed.
/// As rst does not know about link titles,
/// the parser always returns an empty `link_title` as `Cow::Borrowed("")`.
/// ```
/// use parse_hyperlinks::parser::Link;
/// use parse_hyperlinks::parser::restructured_text::rst_text2dest;
/// use std::borrow::Cow;
///
/// assert_eq!(
///   rst_text2dest("`name <destination>`_abc"),
///   Ok(("abc", (Cow::from("name"), Cow::from("destination"), Cow::from(""))))
/// );
/// ```
/// A hyperlink reference may directly embed a destination URI or (since Docutils
/// 0.11) a hyperlink reference within angle brackets `<>` as shown in the
/// following example:
/// ```rst
/// abc `Python home page <http://www.python.org>`_ abc
/// ```
/// The bracketed URI must be preceded by whitespace and be the last text
/// before the end string. For more details see the
/// [reStructuredText Markup
/// Specification](https://docutils.sourceforge.io/docs/ref/rst/restructuredtext.html#embedded-uris-and-aliases)
pub fn rst_text2dest(i: &str) -> nom::IResult<&str, (Cow<str>, Cow<str>, Cow<str>)> {
    let (i, (ln, ld)) = rst_parse_text2dest(i)?;
    let ln = rst_escaped_link_text_transform(ln)?.1;
    let ld = rst_escaped_link_destination_transform(ld)?.1;

    Ok((i, (ln, ld, Cow::Borrowed(""))))
}

/// This parser used by `rst_link()`, does all the work that can be
/// done without allocating new strings.
/// Removing of escaped characters is not performed here.
fn rst_parse_text2dest(i: &str) -> nom::IResult<&str, (&str, &str)> {
    let (i, j) = nom::sequence::delimited(
        tag("`"),
        nom::bytes::complete::escaped(
            nom::character::complete::none_of(r#"\`"#),
            '\\',
            nom::character::complete::one_of(ESCAPABLE),
        ),
        tag("`_"),
    )(i)?;
    // TODO: double `__` is not allowed for inline links:
    // Consume another optional pending `_`, if there is one. This can not fail.
    let (i, _) = nom::combinator::opt(nom::bytes::complete::tag("_"))(i)?;

    // From here on, we only deal with the inner result of the above.
    // Take everything until the first unescaped `<`
    let (j, link_text): (&str, &str) = nom::bytes::complete::escaped(
        nom::character::complete::none_of(r#"\<"#),
        '\\',
        nom::character::complete::one_of(ESCAPABLE),
    )(j)?;
    // Trim trailing whitespace.
    let link_text = link_text.trim_end();
    let (j, link_destination) = nom::sequence::delimited(
        tag("<"),
        nom::bytes::complete::escaped(
            nom::character::complete::none_of(r#"\<>"#),
            '\\',
            nom::character::complete::one_of(ESCAPABLE),
        ),
        tag(">"),
    )(j)?;
    // Fail if there are bytes left between `>` and `\``.
    let (_, _) = nom::combinator::eof(j)?;

    Ok((i, (link_text, link_destination)))
}

/// Wrapper around `rst_text2dest()` that packs the result in
/// `Link::Text2Dest`.
pub fn rst_text2label_link(i: &str) -> nom::IResult<&str, Link> {
    let (i, (te, la)) = rst_text2label(i)?;
    Ok((i, Link::Text2Label(te, la)))
}

/// TODO
///
/// ```rust
/// use parse_hyperlinks::parser::Link;
/// use parse_hyperlinks::parser::restructured_text::rst_text2label;
/// use std::borrow::Cow;
///
/// assert_eq!(
///   rst_text2label("linktext_ abc"),
///   Ok((" abc", (Cow::from("linktext"), Cow::from("linktext"))))
/// );
/// assert_eq!(
///   rst_text2label("`link text`_ abc"),
///   Ok((" abc", (Cow::from("link text"), Cow::from("link text"))))
/// );
/// assert_eq!(
///   rst_text2label("`link text`__ abc"),
///   Ok((" abc", (Cow::from("link text"), Cow::from("_"))))
/// );
/// ```
///
pub fn rst_text2label(i: &str) -> nom::IResult<&str, (Cow<str>, Cow<str>)> {
    let (i, (te, la)) = rst_parse_text2label(i)?;
    let te = rst_escaped_link_text_transform(te)?.1;
    let la = rst_escaped_link_text_transform(la)?.1;

    Ok((i, (te, la)))
}

/// Parses a _reference link_. (Doctree element `reference`).
///
/// Named hyperlink references:
/// No start-string, end-string = `_.
/// Start-string = "`", end-string = `\`_`. (Phrase references.)
/// Anonymous hyperlink references:
/// No start-string, end-string = `__`.
/// Start-string = "`", end-string = `\`__`. (Phrase references.)
///
///
/// Hyperlink references are indicated by a trailing underscore, "_", except for
/// standalone hyperlinks which are recognized independently.
///
/// Important: before this parser try `rst_text2dest()` first!
///
/// The caller must guarantee, that either:
/// * we are at the input start -or-
/// * the byte just before was a whitespace (including newline)!
///
/// For named references in reStructuredText `link_text` and `link_label`
/// are the same. By convention we return for anonymous references:
/// `link_label='_'`.
///
/// The parser checks that this _reference link_ is followed by a whitespace
/// without consuming it.
///
fn rst_parse_text2label(i: &str) -> nom::IResult<&str, (&str, &str)> {
    // Consumes and returns a word ending with `_`.
    // Strips off one the trailing `_` before returning the result.
    fn take_word_consume_first_ending_underscore(i: &str) -> nom::IResult<&str, &str> {
        let mut i = i;
        let (k, mut r) =
            nom::bytes::complete::take_till1(|c| c == ' ' || c == '\t' || c == '\n')(i)?;
        // Is `r` ending with `__`?
        if r.len() >= 2 && &r[r.len() - 2..r.len()] == "__" {
            // Consume one `_`, but keep one `_` in remaining bytes.
            i = &i[r.len() - 1..];
            // Strip two `__` from result.
            r = &r[..r.len() - 2];
        } else {
            // Make sure that at least the last byte is `_`.
            let _ = tag("_")(&r[r.len() - 1..r.len()])?;
            // Consume it.
            i = k;
            // Strip it from result.
            r = &r[..r.len() - 1]
        };

        Ok((i, r))
    }

    let (mut i, link_text) = alt((
        nom::sequence::delimited(
            tag("`"),
            nom::bytes::complete::escaped(
                nom::character::complete::none_of(r#"\`"#),
                '\\',
                nom::character::complete::one_of(ESCAPABLE),
            ),
            tag("`_"),
        ),
        take_word_consume_first_ending_underscore,
    ))(i)?;

    // For named references in reStructuredText `link_text` and `link_label`
    // are the same. By convention we define for anonymous references the
    // `link_label='_'`.
    let mut link_label = link_text;

    // Is this an anonymous reference? Consume the second `_` also.
    if let Ok((j, _)) = nom::character::complete::char::<_, nom::error::Error<_>>('_')(i) {
        link_label = "_";
        i = j;
    };

    Ok((i, (link_text, link_label)))
}

/// Wrapper around `rst_label2dest()` that packs the result in
/// `Link::Label2Dest`.
pub fn rst_label2dest_link(i: &str) -> nom::IResult<&str, Link> {
    let (i, (l, d, t)) = rst_label2dest(i)?;
    Ok((i, Link::Label2Dest(l, d, t)))
}

/// Parse a reStructuredText _link reference definition_.
///
/// This parser consumes until the end of the line. As rst does not know about link titles,
/// the parser always returns an empty `link_title` as `Cow::Borrowed("")`.
/// ```
/// use parse_hyperlinks::parser::Link;
/// use parse_hyperlinks::parser::restructured_text::rst_label2dest;
/// use std::borrow::Cow;
///
/// assert_eq!(
///   rst_label2dest("   .. _`label`: destination\nabc"),
///   Ok(("\nabc", (Cow::from("label"), Cow::from("destination"), Cow::from(""))))
/// );
/// ```
/// Here some examples for link references:
/// ```rst
/// .. _Python home page: http://www.python.org
/// .. _`Python: home page`: http://www.python.org
/// ```
/// See unit test `test_rst_label2dest()` for more examples.
pub fn rst_label2dest(i: &str) -> nom::IResult<&str, (Cow<str>, Cow<str>, Cow<str>)> {
    let my_err = |_| {
        nom::Err::Error(nom::error::Error::new(
            i,
            nom::error::ErrorKind::EscapedTransform,
        ))
    };

    let (i, c) = rst_explicit_markup_block(i)?;

    let (ln, ld) = match c {
        Cow::Borrowed(s) => {
            let (_, (ln, ld)) = rst_parse_label2dest(s)?;
            (
                rst_escaped_link_text_transform(ln)?.1,
                rst_escaped_link_destination_transform(ld)?.1,
            )
        }

        Cow::Owned(strg) => {
            let (_, (ln, ld)) = rst_parse_label2dest(&strg).map_err(my_err)?;
            let ln = Cow::Owned(
                rst_escaped_link_text_transform(ln)
                    .map_err(my_err)?
                    .1
                    .to_string(),
            );
            let ld = Cow::Owned(
                rst_escaped_link_destination_transform(ld)
                    .map_err(my_err)?
                    .1
                    .to_string(),
            );
            (ln, ld)
        }
    };

    // We do not need to consume whitespace until the end of the line,
    // because `rst_explicit_markup_block()` had stripped the whitespace
    // already.

    Ok((i, (ln, ld, Cow::Borrowed(""))))
}

/// This parser detects the position of the link name and the link destination.
/// It does not perform any transformation.
/// The caller must guarantee, that the parser starts at first character of the
/// input or at the first character of a line.
/// If the reference name contains any colons, either:
/// * the phrase must be enclosed in backquotes, or
/// * the colon must be backslash escaped.
/// [reStructuredText Markup
/// Specification](https://docutils.sourceforge.io/docs/ref/rst/restructuredtext.html#hyperlink-targets)
fn rst_parse_label2dest(i: &str) -> nom::IResult<&str, (&str, &str)> {
    let (i, _) = nom::character::complete::char('_')(i)?;
    let (link_destination, link_text) = alt((
        nom::sequence::delimited(
            tag("`"),
            nom::bytes::complete::escaped(
                nom::character::complete::none_of(r#"\`"#),
                '\\',
                nom::character::complete::one_of(r#" `:<>"#),
            ),
            tag("`: "),
        ),
        nom::sequence::terminated(
            nom::bytes::complete::escaped(
                nom::character::complete::none_of(r#"\:"#),
                '\\',
                nom::character::complete::one_of(r#" `:<>"#),
            ),
            tag(": "),
        ),
    ))(i)?;

    Ok(("", (link_text, link_destination)))
}

/// This parses an explicit markup block.
/// The parser expects to start at the beginning of the line.
/// Syntax diagram:
/// ```text
/// +-------+----------------------+
/// | ".. " | in  1                |
/// +-------+ in  2                |
///         |    in  3             |
///         +----------------------+
/// out
/// ```
/// An explicit markup block is a text block:
/// * whose first line begins with ".." followed by whitespace (the "explicit
///   markup start"),
/// * whose second and subsequent lines (if any) are indented relative to the
///   first, and
/// * which ends before an unindented line
/// As with external hyperlink targets, the link block of an indirect
/// hyperlink target may begin on the same line as the explicit markup start
/// or the next line. It may also be split over multiple lines, in which case
/// the lines are joined with whitespace before being normalized.
fn rst_explicit_markup_block(i: &str) -> nom::IResult<&str, Cow<str>> {
    fn indent<'a>(wsp1: &'a str, wsp2: &'a str) -> impl Fn(&'a str) -> IResult<&'a str, ()> {
        move |i: &str| {
            let (i, _) = nom::character::complete::line_ending(i)?;
            let (i, _) = nom::bytes::complete::tag(wsp1)(i)?;
            let (i, _) = nom::bytes::complete::tag(wsp2)(i)?;
            Ok((i, ()))
        }
    }

    let (i, (wsp1, wsp2)) = nom::sequence::pair(
        nom::character::complete::space0,
        nom::combinator::map(nom::bytes::complete::tag(".. "), |_| "   "),
    )(i)?;

    let (j, v) = nom::multi::separated_list1(
        indent(&wsp1, &wsp2),
        nom::character::complete::not_line_ending,
    )(i)?;

    // If the block consists of only one line return now.
    if v.len() == 1 {
        return Ok((j, Cow::Borrowed(v[0].clone())));
    };

    let mut s = String::new();
    let mut is_first = true;

    for subs in &v {
        if !is_first {
            s.push(' ');
        }
        s.push_str(subs);
        is_first = false;
    }

    Ok((j, Cow::from(s)))
}

/// Replace the following escaped characters:
///     \\\`\ \:\<\>
/// with:
///     \`:<>
/// Preserves usual whitespace, but removes `\ `.
fn rst_escaped_link_text_transform(i: &str) -> IResult<&str, Cow<str>> {
    nom::combinator::map(
        nom::bytes::complete::escaped_transform(
            nom::bytes::complete::is_not("\\"),
            '\\',
            alt((
                value("\\", tag("\\")),
                value("`", tag("`")),
                value(":", tag(":")),
                value("<", tag("<")),
                value(">", tag(">")),
                value("", tag(" ")),
            )),
        ),
        |s| if s == i { Cow::from(i) } else { Cow::from(s) },
    )(i)
}

/// Deletes all whitespace, but keeps one space for each `\ `.
fn remove_whitespace(i: &str) -> IResult<&str, Cow<str>> {
    let mut res = Cow::Borrowed("");
    let mut j = i;
    while j != "" {
        let (k, _) = nom::character::complete::multispace0(j)?;
        let (k, s) = nom::bytes::complete::escaped(
            nom::character::complete::none_of("\\\r\n \t"),
            '\\',
            nom::character::complete::one_of(r#" :`<>\"#),
        )(k)?;
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
        j = k;
    }

    Ok((j, res))
}

/// Replace the following escaped characters:
///     \\\`\ \:\<\>
/// with:
///     \` :<>
fn rst_escaped_link_destination_transform(i: &str) -> IResult<&str, Cow<str>> {
    let my_err = |_| {
        nom::Err::Error(nom::error::Error::new(
            i,
            nom::error::ErrorKind::EscapedTransform,
        ))
    };

    let (j, c) = remove_whitespace(i)?;

    let (_, s) =
        nom::bytes::complete::escaped_transform::<_, nom::error::Error<_>, _, _, _, _, _, _>(
            nom::bytes::complete::is_not("\\"),
            '\\',
            nom::character::complete::one_of(ESCAPABLE),
        )(&*c)
        .map_err(my_err)?;

    // When nothing was changed we can continue with `Borrowed`.
    if s == i {
        Ok((j, Cow::Borrowed(i)))
    } else {
        Ok((j, Cow::Owned(s.to_owned())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom::error::ErrorKind;

    #[test]
    fn test_rst_text2dest() {
        let expected = (
            "abc",
            (
                Cow::from("Python home page"),
                Cow::from("http://www.python.org"),
                Cow::from(""),
            ),
        );
        assert_eq!(
            rst_text2dest("`Python home page <http://www.python.org>`_abc").unwrap(),
            expected
        );
        assert_eq!(
            rst_text2dest("`Python home page <http://www.python.org>`__abc").unwrap(),
            expected
        );

        let expected = (
            "",
            (
                Cow::from(r#"Python<home> page"#),
                Cow::from("http://www.python.org"),
                Cow::from(""),
            ),
        );
        assert_eq!(
            rst_text2dest(r#"`Python\ \<home\> page <http://www.python.org>`_"#).unwrap(),
            expected
        );

        let expected = (
            "",
            (
                Cow::from(r#"my news at <http://python.org>"#),
                Cow::from("http://news.python.org"),
                Cow::from(""),
            ),
        );
        assert_eq!(
            rst_text2dest(r#"`my news at \<http://python.org\> <http://news.python.org>`_"#)
                .unwrap(),
            expected
        );

        let expected = (
            "",
            (
                Cow::from(r#"my news at <http://python.org>"#),
                Cow::from(r#"http://news. <python>.org"#),
                Cow::from(""),
            ),
        );
        assert_eq!(
            rst_text2dest(
                r#"`my news at \<http\://python.org\> <http:// news.\ \<python\>.org>`_"#
            )
            .unwrap(),
            expected
        );
    }

    #[test]
    fn test_rst_parse_text2dest() {
        let expected = ("abc", ("Python home page", "http://www.python.org"));
        assert_eq!(
            rst_parse_text2dest("`Python home page <http://www.python.org>`_abc").unwrap(),
            expected
        );

        let expected = ("", (r#"Python\ \<home\> page"#, "http://www.python.org"));
        assert_eq!(
            rst_parse_text2dest(r#"`Python\ \<home\> page <http://www.python.org>`_"#).unwrap(),
            expected
        );

        let expected = (
            "",
            (
                r#"my news at \<http://python.org\>"#,
                "http://news.python.org",
            ),
        );
        assert_eq!(
            rst_parse_text2dest(r#"`my news at \<http://python.org\> <http://news.python.org>`_"#)
                .unwrap(),
            expected
        );

        let expected = (
            "",
            (
                r#"my news at \<http\://python.org\>"#,
                r#"http:// news.\ \<python\>.org"#,
            ),
        );
        assert_eq!(
            rst_parse_text2dest(
                r#"`my news at \<http\://python.org\> <http:// news.\ \<python\>.org>`_"#
            )
            .unwrap(),
            expected
        );
    }

    #[test]
    fn test_rst_text2label() {
        assert_eq!(
            rst_text2label(r#"li\<nktext_ abc"#),
            Ok((" abc", (Cow::from("li<nktext"), Cow::from("li<nktext"))))
        );
        assert_eq!(
            rst_text2label(r#"`li\:nk text`_ abc"#),
            Ok((" abc", (Cow::from("li:nk text"), Cow::from("li:nk text"))))
        );
        assert_eq!(
            rst_text2label("`link text`__ abc"),
            Ok((" abc", (Cow::from("link text"), Cow::from("_"))))
        );
    }

    #[test]
    fn test_rst_parse_text2label() {
        assert_eq!(
            rst_parse_text2label("linktext_ abc"),
            Ok((" abc", ("linktext", "linktext")))
        );

        assert_eq!(
            rst_parse_text2label("linktext__ abc"),
            Ok((" abc", ("linktext", "_")))
        );

        assert_eq!(
            rst_parse_text2label("link_text_ abc"),
            Ok((" abc", ("link_text", "link_text")))
        );

        assert_eq!(
            rst_parse_text2label("`link text`_ abc"),
            Ok((" abc", ("link text", "link text")))
        );

        assert_eq!(
            rst_parse_text2label("`link text`_abc"),
            Ok(("abc", ("link text", "link text")))
        );

        assert_eq!(
            rst_parse_text2label("`link_text`_ abc"),
            Ok((" abc", ("link_text", "link_text")))
        );

        assert_eq!(
            rst_parse_text2label("`link text`__ abc"),
            Ok((" abc", ("link text", "_")))
        );
    }

    #[test]
    fn test_rst_label2dest() {
        let expected = (
            "\nabc",
            (
                Cow::from("Python: home page"),
                Cow::from("http://www.python.org"),
                Cow::from(""),
            ),
        );
        assert_eq!(
            rst_label2dest(".. _`Python: home page`: http://www.python.org\nabc").unwrap(),
            expected
        );
        assert_eq!(
            rst_label2dest("  .. _`Python: home page`: http://www.py\n     thon.org    \nabc")
                .unwrap(),
            expected
        );

        let expected = nom::Err::Error(nom::error::Error::new(
            "x .. _`Python: home page`: http://www.python.org\nabc",
            ErrorKind::Tag,
        ));
        assert_eq!(
            rst_label2dest("x .. _`Python: home page`: http://www.python.org\nabc").unwrap_err(),
            expected
        );

        let expected = (
            "",
            (
                Cow::from("Python: `home page`"),
                Cow::from("http://www.python .org"),
                Cow::from(""),
            ),
        );
        assert_eq!(
            rst_label2dest(r#".. _Python\: \`home page\`: http://www.python\ .org"#).unwrap(),
            expected
        );
        assert_eq!(
            rst_label2dest(r#".. _`Python: \`home page\``: http://www.python\ .org"#).unwrap(),
            expected
        );

        let expected = (
            "",
            (
                Cow::from("my news at <http://python.org>"),
                Cow::from("http://news.python.org"),
                Cow::from(""),
            ),
        );
        assert_eq!(
            rst_label2dest(r#".. _`my news at <http://python.org>`: http://news.python.org"#)
                .unwrap(),
            expected
        );
        assert_eq!(
            rst_label2dest(r#".. _`my news at \<http://python.org\>`: http://news.python.org"#)
                .unwrap(),
            expected
        );
        assert_eq!(
            rst_label2dest(r#".. _my news at \<http\://python.org\>: http://news.python.org"#)
                .unwrap(),
            expected
        );

        let expected = (
            "",
            (
                Cow::from("my news"),
                Cow::from("http://news.<python>.org"),
                Cow::from(""),
            ),
        );
        assert_eq!(
            rst_label2dest(r#".. _my news: http://news.<python>.org"#).unwrap(),
            expected
        );
        assert_eq!(
            rst_label2dest(r#".. _my news: http://news.\<python\>.org"#).unwrap(),
            expected
        );
    }

    #[test]
    fn test_rst_parse_link_label2dest() {
        let expected = ("", ("Python home page", "http://www.python.org"));
        assert_eq!(
            rst_parse_label2dest("_Python home page: http://www.python.org").unwrap(),
            expected
        );
        assert_eq!(
            rst_parse_label2dest("_`Python home page`: http://www.python.org").unwrap(),
            expected
        );

        let expected = ("", ("Python: home page", "http://www.python.org"));
        assert_eq!(
            rst_parse_label2dest("_`Python: home page`: http://www.python.org").unwrap(),
            expected
        );

        let expected = ("", (r#"Python\: home page"#, "http://www.python.org"));
        assert_eq!(
            rst_parse_label2dest(r#"_Python\: home page: http://www.python.org"#).unwrap(),
            expected
        );

        let expected = (
            "",
            ("my news at <http://python.org>", "http://news.python.org"),
        );
        assert_eq!(
            rst_parse_label2dest(r#"_`my news at <http://python.org>`: http://news.python.org"#)
                .unwrap(),
            expected
        );

        let expected = (
            "",
            (
                r#"my news at \<http://python.org\>"#,
                "http://news.python.org",
            ),
        );
        assert_eq!(
            rst_parse_label2dest(r#"_`my news at \<http://python.org\>`: http://news.python.org"#)
                .unwrap(),
            expected
        );

        let expected = (
            "",
            (
                r#"my news at \<http\://python.org\>"#,
                "http://news.python.org",
            ),
        );
        assert_eq!(
            rst_parse_label2dest(r#"_my news at \<http\://python.org\>: http://news.python.org"#)
                .unwrap(),
            expected
        );
    }

    #[test]
    fn test_rst_explicit_markup_block() {
        assert_eq!(
            rst_explicit_markup_block(".. 11111"),
            Ok(("", Cow::from("11111")))
        );
        assert_eq!(
            rst_explicit_markup_block("   .. 11111\nout"),
            Ok(("\nout", Cow::from("11111")))
        );
        assert_eq!(
            rst_explicit_markup_block("   .. 11111\n      222222\n      333333\nout"),
            Ok(("\nout", Cow::from("11111 222222 333333")))
        );
        assert_eq!(
            rst_explicit_markup_block("   .. first\n      second\n       1indent\nout"),
            Ok(("\nout", Cow::from("first second  1indent")))
        );
        assert_eq!(
            rst_explicit_markup_block("   ..first"),
            Err(nom::Err::Error(nom::error::Error::new(
                "..first",
                ErrorKind::Tag
            )))
        );
        assert_eq!(
            rst_explicit_markup_block("x  .. first"),
            Err(nom::Err::Error(nom::error::Error::new(
                "x  .. first",
                ErrorKind::Tag
            )))
        );
    }

    #[test]
    fn test_rst_escaped_link_text_transform() {
        assert_eq!(rst_escaped_link_text_transform(""), Ok(("", Cow::from(""))));
        // Different than the link destination version.
        assert_eq!(
            rst_escaped_link_text_transform("   "),
            Ok(("", Cow::from("   ")))
        );
        // Different than the link destination version.
        assert_eq!(
            rst_escaped_link_text_transform(r#"\ \ \ "#),
            Ok(("", Cow::from("")))
        );
        assert_eq!(
            rst_escaped_link_text_transform(r#"abc`:<>abc"#),
            Ok(("", Cow::from(r#"abc`:<>abc"#)))
        );
        assert_eq!(
            rst_escaped_link_text_transform(r#"\:\`\<\>\\"#),
            Ok(("", Cow::from(r#":`<>\"#)))
        );
    }

    #[test]
    fn test_rst_escaped_link_destination_transform() {
        assert_eq!(
            rst_escaped_link_destination_transform(""),
            Ok(("", Cow::Borrowed("")))
        );
        // Different than the link name version.
        assert_eq!(
            rst_escaped_link_destination_transform("  "),
            Ok(("", Cow::Borrowed("")))
        );
        assert_eq!(
            rst_escaped_link_destination_transform(" x x"),
            Ok(("", Cow::Owned("xx".to_string())))
        );
        // Different than the link name version.
        assert_eq!(
            rst_escaped_link_destination_transform(r#"\ \ \ "#),
            Ok(("", Cow::Owned("   ".to_string())))
        );
        assert_eq!(
            rst_escaped_link_destination_transform(r#"abc`:<>abc"#),
            Ok(("", Cow::Borrowed(r#"abc`:<>abc"#)))
        );
        assert_eq!(
            rst_escaped_link_destination_transform(r#"\:\`\<\>\\"#),
            Ok(("", Cow::Owned(r#":`<>\"#.to_string())))
        );
    }
    #[test]
    fn test_remove_whitespace() {
        assert_eq!(remove_whitespace(" abc "), Ok(("", Cow::Borrowed("abc"))));
        assert_eq!(
            remove_whitespace(" x x"),
            Ok(("", Cow::Owned("xx".to_string())))
        );
        assert_eq!(remove_whitespace("  \t \r \n"), Ok(("", Cow::from(""))));
        assert_eq!(
            remove_whitespace(r#"\ \ \ "#),
            Ok(("", Cow::Borrowed(r#"\ \ \ "#)))
        );
        assert_eq!(
            remove_whitespace(r#"abc`:<>abc"#),
            Ok(("", Cow::Borrowed(r#"abc`:<>abc"#)))
        );
        assert_eq!(
            remove_whitespace(r#"\:\`\<\>\\"#),
            Ok(("", Cow::Borrowed(r#"\:\`\<\>\\"#)))
        );

        assert_eq!(
            remove_whitespace("http://www.py\n     thon.org"),
            Ok(("", Cow::Owned("http://www.python.org".to_string())))
        );
    }
}
