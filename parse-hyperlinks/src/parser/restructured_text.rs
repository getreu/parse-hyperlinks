//! This module implements parsers for RestructuredText hyperlinks.
#![allow(dead_code)]
#![allow(clippy::type_complexity)]

use crate::parser::parse::LABEL_LEN_MAX;
use crate::parser::Link;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::*;
use nom::IResult;
use std::borrow::Cow;

/// Character that can be escaped with `\`.
///
/// Note: If ever you change this, change also
/// `rst_escaped_link_text_transform()`.
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
/// the parser always returns an empty `link_title` as `Cow::Borrowed("")`
/// ```
/// use parse_hyperlinks::parser::Link;
/// use parse_hyperlinks::parser::restructured_text::rst_text2dest;
/// use std::borrow::Cow;
///
/// assert_eq!(
///   rst_text2dest("`name <destination>`__abc"),
///   Ok(("abc", (Cow::from("name"), Cow::from("destination"), Cow::from(""))))
/// );
/// ```
/// A hyperlink reference may directly embed a destination URI or (since Docutils
/// 0.11) a hyperlink reference within angle brackets `<>` as shown in the
/// following example:
/// ```rst
/// abc `Python home page <http://www.python.org>`__ abc
/// ```
/// The bracketed URI must be preceded by whitespace and be the last text
/// before the end string.
pub fn rst_text2dest(i: &str) -> nom::IResult<&str, (Cow<str>, Cow<str>, Cow<str>)> {
    let (i, (ln, ld)) = rst_parse_text2target(true, false)(i)?;
    let ln = rst_escaped_link_text_transform(ln)?.1;
    let ld = rst_escaped_link_destination_transform(ld)?.1;

    Ok((i, (ln, ld, Cow::Borrowed(""))))
}

/// Wrapper around `rst_textlabel2dest()` that packs the result in
/// `Link::TextLabel2Dest`.
pub fn rst_text_label2dest_link(i: &str) -> nom::IResult<&str, Link> {
    let (i, (te, de, ti)) = rst_text_label2dest(i)?;
    Ok((i, Link::TextLabel2Dest(te, de, ti)))
}

/// Parse a RestructuredText combined _inline hyperlink_ with _link reference definition_.
///
/// The parser expects to start at the link start (\`) to succeed.
/// As rst does not know about link titles,
/// the parser always returns an empty `link_title` as `Cow::Borrowed("")`.
/// ```
/// use parse_hyperlinks::parser::Link;
/// use parse_hyperlinks::parser::restructured_text::rst_text_label2dest;
/// use std::borrow::Cow;
///
/// assert_eq!(
///   rst_text_label2dest("`name <destination>`_abc"),
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
/// before the end string.
pub fn rst_text_label2dest(i: &str) -> nom::IResult<&str, (Cow<str>, Cow<str>, Cow<str>)> {
    let (i, (ln, ld)) = rst_parse_text2target(false, false)(i)?;
    let ln = rst_escaped_link_text_transform(ln)?.1;
    let ld = rst_escaped_link_destination_transform(ld)?.1;

    Ok((i, (ln, ld, Cow::Borrowed(""))))
}

/// This parser finds rst links of type:
///     `*<*>`__
/// or:
///     `*<*>`_
///
/// Escape sequences are recognized and skipped, but not replaced here.
/// If `anonym==true`: it recognizes:
///     `*<*>`__
/// otherwise:
///     `*<*>`_
///
/// If `label==true` (`target==label`): it recognizes
///     `*<*_>`_?
/// otherwise (`target==dest`):
///     `*<*>`_?
fn rst_parse_text2target(
    anonym: bool,
    label: bool,
) -> impl Fn(&str) -> IResult<&str, (&str, &str)> {
    move |i: &str| {
        let (mut i, inner) = nom::sequence::delimited(
            tag("`"),
            nom::bytes::complete::escaped(
                nom::character::complete::none_of(r#"\`"#),
                '\\',
                nom::character::complete::one_of(ESCAPABLE),
            ),
            tag("`_"),
        )(i)?;

        if anonym {
            let (j, _) = nom::character::complete::char('_')(i)?;
            i = j;
        };

        // Assure that the next char is not`_`.
        if !i.is_empty() {
            let _ = nom::combinator::not(nom::character::complete::char('_'))(i)?;
        };

        // From here on, we only deal with the inner result of the above.
        // Take everything until the first unescaped `<`
        let (inner_rest, link_text): (&str, &str) = nom::bytes::complete::escaped(
            nom::character::complete::none_of(r#"\<"#),
            '\\',
            nom::character::complete::one_of(ESCAPABLE),
        )(inner)?;
        // Trim trailing whitespace.
        let link_text = link_text.trim_end();

        let (j, mut link_dest_label) = nom::sequence::delimited(
            tag("<"),
            nom::bytes::complete::escaped(
                nom::character::complete::none_of(r#"\<>"#),
                '\\',
                nom::character::complete::one_of(ESCAPABLE),
            ),
            tag(">"),
        )(inner_rest)?;

        // Fail if there are bytes left between `>` and `\``.
        let (_, _) = nom::combinator::eof(j)?;

        // Now check if `link_dest_label` is what we are expecting (which depends
        // on `label`).

        // Fail if `link_dest_label` is empty.
        let (_, _) = nom::combinator::not(nom::combinator::eof)(link_dest_label)?;

        // Get last char.
        let last_char_is_ = link_dest_label.is_char_boundary(link_dest_label.len() - 1)
            && &link_dest_label[link_dest_label.len() - 1..] == "_";
        // If (`label==true`), we expect trailing `_`, fail otherwise.
        // If (`label==false`), we fail when there is a trailing `_`.
        if (label && !last_char_is_) || (!label && last_char_is_) {
            return Err(nom::Err::Error(nom::error::Error::new(
                i,
                nom::error::ErrorKind::Tag,
            )));
        };
        // When label, strip trailing `_`.
        if label {
            link_dest_label = &link_dest_label[..link_dest_label.len() - 1];
        };

        Ok((i, (link_text, link_dest_label)))
    }
}

/// Wrapper around `rst_text2dest()` that packs the result in
/// `Link::Text2Dest`.
pub fn rst_text2label_link(i: &str) -> nom::IResult<&str, Link> {
    let (i, (te, la)) = rst_text2label(i)?;
    Ok((i, Link::Text2Label(te, la)))
}

/// Parse a RestructuredText _reference link_.
///
/// The caller must guarantee, that
/// * the parser is at the input start (no bytes exist before).
/// * the preceding bytes are whitespaces or newline, _or_
/// * the preceding bytes are whitespaces or newline, followed by one of: `([<'"`
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
///   rst_text2label("`link text<link label_>`_ abc"),
///   Ok((" abc", (Cow::from("link text"), Cow::from("link label"))))
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
    let (mut i, (link_text, mut link_label)) = alt((
        rst_parse_text2target(false, true),
        nom::combinator::map(rst_parse_simple_label, |s| (s, s)),
    ))(i)?;

    // Is this an anonymous reference? Consume the second `_` also.
    if let (j, Some(_)) = nom::combinator::opt(nom::character::complete::char('_'))(i)? {
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
/// assert_eq!(
///   rst_label2dest("   .. __: destination\nabc"),
///   Ok(("\nabc", (Cow::from("_"), Cow::from("destination"), Cow::from(""))))
/// );
/// assert_eq!(
///   rst_label2dest("   __ destination\nabc"),
///   Ok(("\nabc", (Cow::from("_"), Cow::from("destination"), Cow::from(""))))
/// );
/// ```
/// Here some examples for link references:
/// ```rst
/// .. _Python home page: http://www.python.org
/// .. _`Python: home page`: http://www.python.org
/// ```
/// See unit test `test_rst_label2dest()` for more examples.
pub fn rst_label2dest(i: &str) -> nom::IResult<&str, (Cow<str>, Cow<str>, Cow<str>)> {
    let (i, (l, d)) = rst_label2target(false, i)?;
    Ok((i, (l, d, Cow::from(""))))
}

/// Wrapper around `rst_label2label()` that packs the result in
/// `Link::Label2Label`.
pub fn rst_label2label_link(i: &str) -> nom::IResult<&str, Link> {
    let (i, (l1, l2)) = rst_label2label(i)?;
    Ok((i, Link::Label2Label(l1, l2)))
}

/// Parse a reStructuredText _link reference to link reference definition_.
/// This type defines an alias (alternative name) for a link reference:
/// ```
/// use parse_hyperlinks::parser::Link;
/// use parse_hyperlinks::parser::restructured_text::rst_label2label;
/// use std::borrow::Cow;
///
/// assert_eq!(
///   rst_label2label("   .. _`alt label`: `label`_\nabc"),
///   Ok(("\nabc", (Cow::from("alt label"), Cow::from("label"))))
/// );
/// assert_eq!(
///   rst_label2label("   .. __: label_\nabc"),
///   Ok(("\nabc", (Cow::from("_"), Cow::from("label"))))
/// );
/// assert_eq!(
///   rst_label2label("   __ label_\nabc"),
///   Ok(("\nabc", (Cow::from("_"), Cow::from("label"))))
/// );
/// ```
pub fn rst_label2label(i: &str) -> nom::IResult<&str, (Cow<str>, Cow<str>)> {
    rst_label2target(true, i)
}

/// Parser for _link_reference_definitions_:
/// * `label==false`:  the link is of type `Label2Dest`
/// * `label==true`: the link is of type `Label2Label`
fn rst_label2target(label: bool, i: &str) -> nom::IResult<&str, (Cow<str>, Cow<str>)> {
    let my_err = |_| {
        nom::Err::Error(nom::error::Error::new(
            i,
            nom::error::ErrorKind::EscapedTransform,
        ))
    };

    // If there is a block start? What kind of?
    let (i, c, block_header_is__) =
        if let (i, Some(c)) = nom::combinator::opt(rst_explicit_markup_block(".. "))(i)? {
            (i, c, false)
        } else {
            let (i, c) = rst_explicit_markup_block("__ ")(i)?;
            (i, c, true)
        };

    let (source, target) = match c {
        Cow::Borrowed(s) => {
            let (_, (ls, lt)) = if !block_header_is__ {
                rst_parse_label2target(label)(s)?
            } else if label {
                // This is supposed to be a label.
                ("", ("_", rst_parse_simple_label(s)?.1))
            } else {
                // This is supposed to be a destination (url).
                ("", ("_", s))
            };
            // If the target is a destination (not a label), the last char must not be `_`.
            if !label {
                let _ = nom::combinator::not(rst_parse_simple_label)(lt).map_err(my_err)?;
            };
            (
                rst_escaped_link_text_transform(ls)?.1,
                rst_escaped_link_destination_transform(lt)?.1,
            )
        }

        Cow::Owned(strg) => {
            let (_, (ls, lt)) = if !block_header_is__ {
                rst_parse_label2target(label)(&strg).map_err(my_err)?
            } else if label {
                // This is supposed to be a label.
                let s = rst_parse_simple_label(&strg).map_err(my_err)?.1;
                ("", ("_", s))
            } else {
                // This is supposed to be a destination (url).
                ("", ("_", strg.as_str()))
            };
            // If the target is a destination (not a label), the last char must not be `_`.
            if !label {
                let _ = nom::combinator::not(rst_parse_simple_label)(lt).map_err(my_err)?;
            };
            let ls = Cow::Owned(
                rst_escaped_link_text_transform(ls)
                    .map_err(my_err)?
                    .1
                    .to_string(),
            );
            let lt = Cow::Owned(
                rst_escaped_link_destination_transform(lt)
                    .map_err(my_err)?
                    .1
                    .to_string(),
            );
            (ls, lt)
        }
    };

    // We do not need to consume whitespace until the end of the line,
    // because `rst_explicit_markup_block()` had stripped the whitespace
    // already.

    Ok((i, (source, target)))
}

/// The parser recognizes `Label2Dest` links (`label==false`):
///     _label: dest
/// or `Label2Label` links (`label==true):
///     _alt_label: label_
/// It does not perform any escape character transformation.
fn rst_parse_label2target(label: bool) -> impl Fn(&str) -> IResult<&str, (&str, &str)> {
    move |i: &str| {
        let (i, link_text) = alt((
            nom::sequence::delimited(
                tag("_`"),
                nom::bytes::complete::escaped(
                    nom::character::complete::none_of(r#"\`"#),
                    '\\',
                    nom::character::complete::one_of(ESCAPABLE),
                ),
                tag("`: "),
            ),
            nom::sequence::delimited(
                tag("_"),
                nom::bytes::complete::escaped(
                    nom::character::complete::none_of(r#"\:"#),
                    '\\',
                    nom::character::complete::one_of(ESCAPABLE),
                ),
                tag(": "),
            ),
            nom::combinator::value("_", tag("__: ")),
        ))(i)?;

        let link_target = if label {
            // The target is another label.
            rst_parse_simple_label(i)?.1
        } else {
            // The target is a destination.
            i
        };

        Ok(("", (link_text, link_target)))
    }
}

/// This parser consumes a simple label:
///     one_word_label_
/// or
///     `more words label`_
fn rst_parse_simple_label(i: &str) -> nom::IResult<&str, &str> {
    // Consumes and returns a word ending with `_`.
    // Strips off one the trailing `_` before returning the result.
    fn take_word_consume_first_ending_underscore(i: &str) -> nom::IResult<&str, &str> {
        let mut i = i;
        let (k, mut r) = nom::bytes::complete::take_till1(|c: char| {
            !(c.is_alphanumeric() || c == '-' || c == '_')
        })(i)?;
        // Is `r` ending with `__`? There should be at least 2 bytes: `"__".len()`
        if r.len() >= 3 && r.is_char_boundary(r.len() - 2) && &r[r.len() - 2..] == "__" {
            // Consume one `_`, but keep one `_` in remaining bytes.
            i = &i[r.len() - 1..];
            // Strip two `__` from result.
            r = &r[..r.len() - 2];
        // Is `r` ending with `_`? There should be at least 1 byte: `"_".len()`.
        } else if !r.is_empty() && r.is_char_boundary(r.len() - 1) && &r[r.len() - 1..] == "_" {
            // Remaining bytes.
            i = k;
            // Strip `_` from result.
            r = &r[..r.len() - 1]
        } else {
            return Err(nom::Err::Error(nom::error::Error::new(
                k,
                nom::error::ErrorKind::Tag,
            )));
        };

        Ok((i, r))
    }

    let (i, r) = nom::combinator::verify(
        alt((
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
        )),
        |s: &str| s.len() <= LABEL_LEN_MAX,
    )(i)?;

    // Return error if label is empty.
    let _ = nom::combinator::not(alt((nom::combinator::eof, tag("``"))))(r)?;

    Ok((i, r))
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
fn rst_explicit_markup_block<'a>(
    block_header: &'a str,
) -> impl Fn(&'a str) -> IResult<&'a str, Cow<'a, str>> {
    move |i: &'a str| {
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
            nom::combinator::map(nom::bytes::complete::tag(block_header), |_| "   "),
        )(i)?;

        let (j, v) = nom::multi::separated_list1(
            indent(wsp1, wsp2),
            nom::character::complete::not_line_ending,
        )(i)?;

        // If the block consists of only one line return now.
        if v.len() == 1 {
            return Ok((j, Cow::Borrowed(v[0])));
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
            // This list is the same as `ESCAPABLE`.
            alt((
                tag("\\"),
                tag("`"),
                tag(":"),
                tag("<"),
                tag(">"),
                tag("_"),
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
    while !j.is_empty() {
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

    let c = &*remove_whitespace(i)?.1;

    let s = nom::bytes::complete::escaped_transform::<_, nom::error::Error<_>, _, _, _, _, _, _>(
        nom::bytes::complete::is_not("\\"),
        '\\',
        nom::character::complete::one_of(ESCAPABLE),
    )(c)
    .map_err(my_err)?
    .1;

    // When nothing was changed we can continue with `Borrowed`.
    if s == i {
        Ok(("", Cow::Borrowed(i)))
    } else {
        Ok(("", Cow::Owned(s)))
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
            rst_text2dest("`Python home page <http://www.python.org>`__abc").unwrap(),
            expected
        );

        let expected = (
            "abc",
            (
                Cow::from(r#"Python<home> page"#),
                Cow::from("http://www.python.org"),
                Cow::from(""),
            ),
        );
        assert_eq!(
            rst_text2dest(r#"`Python\ \<home\> page <http://www.python.org>`__abc"#).unwrap(),
            expected
        );

        let expected = (
            "abc",
            (
                Cow::from(r#"my news at <http://python.org>"#),
                Cow::from("http://news.python.org"),
                Cow::from(""),
            ),
        );
        assert_eq!(
            rst_text2dest(r#"`my news at \<http://python.org\> <http://news.python.org>`__abc"#)
                .unwrap(),
            expected
        );

        let expected = (
            "abc",
            (
                Cow::from(r#"my news at <http://python.org>"#),
                Cow::from(r#"http://news. <python>.org"#),
                Cow::from(""),
            ),
        );
        assert_eq!(
            rst_text2dest(
                r#"`my news at \<http\://python.org\> <http:// news.\ \<python\>.org>`__abc"#
            )
            .unwrap(),
            expected
        );
    }

    #[test]
    fn test_rst_parse_text2dest_label() {
        let expected = ("abc", ("Python home page", "http://www.python.org"));
        assert_eq!(
            rst_parse_text2target(false, false)("`Python home page <http://www.python.org>`_abc")
                .unwrap(),
            expected
        );

        let expected = nom::Err::Error(nom::error::Error::new("abc", ErrorKind::Tag));
        assert_eq!(
            rst_parse_text2target(false, false)("`Python home page <http://www.python.org_>`_abc")
                .unwrap_err(),
            expected
        );

        let expected = nom::Err::Error(nom::error::Error::new("", ErrorKind::Tag));
        assert_eq!(
            rst_parse_text2target(false, false)("`_abc").unwrap_err(),
            expected
        );

        let expected = ("abc", ("Python home page", "http://www.python.org"));
        assert_eq!(
            rst_parse_text2target(true, false)("`Python home page <http://www.python.org>`__abc")
                .unwrap(),
            expected
        );

        let expected = ("abc", (r#"Python\ \<home\> page"#, "http://www.python.org"));
        assert_eq!(
            rst_parse_text2target(false, false)(
                r#"`Python\ \<home\> page <http://www.python.org>`_abc"#
            )
            .unwrap(),
            expected
        );

        let expected = (
            "abc",
            (
                r#"my news at \<http://python.org\>"#,
                "http://news.python.org",
            ),
        );
        assert_eq!(
            rst_parse_text2target(false, false)(
                r#"`my news at \<http://python.org\> <http://news.python.org>`_abc"#
            )
            .unwrap(),
            expected
        );

        let expected = (
            "abc",
            (
                r#"my news at \<http\://python.org\>"#,
                r#"http:// news.\ \<python\>.org"#,
            ),
        );
        assert_eq!(
            rst_parse_text2target(false, false)(
                r#"`my news at \<http\://python.org\> <http:// news.\ \<python\>.org>`_abc"#
            )
            .unwrap(),
            expected
        );

        let expected = (
            "abc",
            (
                r#"my news at \<http\://python.org\>"#,
                r#"http:// news.\ \<python\>.org"#,
            ),
        );
        assert_eq!(
            rst_parse_text2target(false, false)(
                r#"`my news at \<http\://python.org\> <http:// news.\ \<python\>.org>`_abc"#
            )
            .unwrap(),
            expected
        );
        let expected = ("abc", (r#"rst link text"#, "rst_link_label"));
        assert_eq!(
            rst_parse_text2target(false, true)(r#"`rst link text <rst_link_label_>`_abc"#).unwrap(),
            expected
        );

        let expected = nom::Err::Error(nom::error::Error::new("abc", ErrorKind::Tag));
        assert_eq!(
            rst_parse_text2target(false, true)(r#"`my news <python webpage>`_abc"#).unwrap_err(),
            expected
        );
    }

    #[test]
    fn test_rst_text2label() {
        assert_eq!(
            rst_text2label(r#"link_text_ abc"#),
            Ok((" abc", (Cow::from("link_text"), Cow::from("link_text"))))
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

        assert_eq!(
            rst_parse_text2label("`link text<link label_>`_ abc"),
            Ok((" abc", ("link text", "link label")))
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

        let expected = (
            "",
            (
                Cow::from("_"),
                Cow::from("http://news.python.org"),
                Cow::from(""),
            ),
        );
        assert_eq!(
            rst_label2dest(r#".. __: http://news.python.org"#).unwrap(),
            expected
        );
        assert_eq!(
            rst_label2dest(r#"__ http://news.python.org"#).unwrap(),
            expected
        );
        assert_eq!(
            rst_label2dest(".. _label: `link destination`_").unwrap_err(),
            nom::Err::Error(nom::error::Error::new(
                ".. _label: `link destination`_",
                ErrorKind::EscapedTransform
            )),
        );
        assert_eq!(
            rst_label2dest("__ link_destination_").unwrap_err(),
            nom::Err::Error(nom::error::Error::new(
                "__ link_destination_",
                ErrorKind::EscapedTransform
            )),
        );
    }

    #[test]
    fn test_rst_label2label() {
        assert_eq!(
            rst_label2label("   .. _`alt label`: `label`_\nabc"),
            Ok(("\nabc", (Cow::from("alt label"), Cow::from("label"))))
        );
        assert_eq!(
            rst_label2label("   .. __: label_\nabc"),
            Ok(("\nabc", (Cow::from("_"), Cow::from("label"))))
        );
        assert_eq!(
            rst_label2label("   __ label_\nabc"),
            Ok(("\nabc", (Cow::from("_"), Cow::from("label"))))
        );
        assert_eq!(
            rst_label2label("_label: label").unwrap_err(),
            nom::Err::Error(nom::error::Error::new("_label: label", ErrorKind::Tag)),
        );
        assert_eq!(
            rst_label2label("__ destination").unwrap_err(),
            nom::Err::Error(nom::error::Error::new("", ErrorKind::Tag)),
        );
    }

    #[test]
    fn test_rst_parse_label2target() {
        let expected = ("", ("Python home page", "http://www.python.org"));
        assert_eq!(
            rst_parse_label2target(false)("_Python home page: http://www.python.org").unwrap(),
            expected
        );
        assert_eq!(
            rst_parse_label2target(false)("_`Python home page`: http://www.python.org").unwrap(),
            expected
        );

        let expected = ("", ("Python: home page", "http://www.python.org"));
        assert_eq!(
            rst_parse_label2target(false)("_`Python: home page`: http://www.python.org").unwrap(),
            expected
        );

        let expected = ("", (r#"Python\: home page"#, "http://www.python.org"));
        assert_eq!(
            rst_parse_label2target(false)(r#"_Python\: home page: http://www.python.org"#).unwrap(),
            expected
        );

        let expected = (
            "",
            ("my news at <http://python.org>", "http://news.python.org"),
        );
        assert_eq!(
            rst_parse_label2target(false)(
                r#"_`my news at <http://python.org>`: http://news.python.org"#
            )
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
            rst_parse_label2target(false)(
                r#"_`my news at \<http://python.org\>`: http://news.python.org"#
            )
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
            rst_parse_label2target(false)(
                r#"_my news at \<http\://python.org\>: http://news.python.org"#
            )
            .unwrap(),
            expected
        );

        let expected = ("", ("_", "http://news.python.org"));
        assert_eq!(
            rst_parse_label2target(false)(r#"__: http://news.python.org"#).unwrap(),
            expected
        );

        let expected = ("", ("alt_label", "one_word_label"));
        assert_eq!(
            rst_parse_label2target(true)("_alt_label: one_word_label_").unwrap(),
            expected
        );

        let expected = ("", ("alt label", "more words label"));
        assert_eq!(
            rst_parse_label2target(true)("_`alt label`: `more words label`_").unwrap(),
            expected
        );
    }

    #[test]
    fn test_parse_simple_label() {
        let expected = ("", "one_word_label");
        assert_eq!(rst_parse_simple_label("one_word_label_").unwrap(), expected);

        let expected = (" abc", "one_word_label");
        assert_eq!(
            rst_parse_simple_label("one_word_label_ abc").unwrap(),
            expected
        );
        assert_eq!(
            rst_parse_simple_label("`one_word_label`_ abc").unwrap(),
            expected
        );

        let expected = ("", "more words label");
        assert_eq!(
            rst_parse_simple_label("`more words label`_").unwrap(),
            expected
        );

        let expected = (". abc", "more words label");
        assert_eq!(
            rst_parse_simple_label("`more words label`_. abc").unwrap(),
            expected
        );

        let expected = ("? abc", "more words label");
        assert_eq!(
            rst_parse_simple_label("`more words label`_? abc").unwrap(),
            expected
        );

        let expected = (" abc", "more words label");
        assert_eq!(
            rst_parse_simple_label("`more words label`_ abc").unwrap(),
            expected
        );

        assert_eq!(
            rst_parse_simple_label("_").unwrap_err(),
            nom::Err::Error(nom::error::Error::new("", ErrorKind::Not)),
        );

        assert_eq!(
            rst_parse_simple_label("``_").unwrap_err(),
            nom::Err::Error(nom::error::Error::new("``_", ErrorKind::TakeTill1)),
        );
    }

    #[test]
    fn test_rst_explicit_markup_block() {
        assert_eq!(
            rst_explicit_markup_block(".. ")(".. 11111"),
            Ok(("", Cow::from("11111")))
        );
        assert_eq!(
            rst_explicit_markup_block(".. ")("   .. 11111\nout"),
            Ok(("\nout", Cow::from("11111")))
        );
        assert_eq!(
            rst_explicit_markup_block(".. ")("   .. 11111\n      222222\n      333333\nout"),
            Ok(("\nout", Cow::from("11111 222222 333333")))
        );
        assert_eq!(
            rst_explicit_markup_block(".. ")("   .. first\n      second\n       1indent\nout"),
            Ok(("\nout", Cow::from("first second  1indent")))
        );
        assert_eq!(
            rst_explicit_markup_block(".. ")("   ..first"),
            Err(nom::Err::Error(nom::error::Error::new(
                "..first",
                ErrorKind::Tag
            )))
        );
        assert_eq!(
            rst_explicit_markup_block(".. ")("x  .. first"),
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
