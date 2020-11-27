//! This module implements parsers for RestructuredText hyperlinks.
#![allow(dead_code)]

use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::*;
use nom::error::Error;
use nom::error::ErrorKind;
use nom::IResult;

/// Parse a RestructuredText hyperlink.
/// The parser expects to start at the link start (\`) to succeed.
/// ```
/// use parse_hyperlinks::parser::restructured_text::rst_link;
/// assert_eq!(
///   rst_link("`name <target>`_abc"),
///   Ok(("abc", ("name".to_string(), "target".to_string())))
/// );
/// ```
/// A hyperlink reference may directly embed a target URI or (since Docutils
/// 0.11) a hyperlink reference within angle brackets ("<...>") as in
/// ```rst
/// abc `Python home page <http://www.python.org>`_ abc
/// ```
/// The bracketed URI must be preceded by whitespace and be the last text
/// before the end string. For more details see the
/// [reStructuredText Markup
/// Specification](https://docutils.sourceforge.io/docs/ref/rst/restructuredtext.html#embedded-uris-and-aliases)
/// It returns either `Ok((i, (link_text, link_destination)))` or some error.
pub fn rst_link(i: &str) -> nom::IResult<&str, (String, String)> {
    match rst_parse_link(i) {
        Ok((i, (ln, lt))) => {
            let ln = if let Ok((_, ln_trans)) = rst_escaped_link_name_transform(ln) {
                ln_trans
            } else {
                ln.to_string()
            };
            let lt = if let Ok((_, lt_trans)) = rst_escaped_link_target_transform(lt) {
                lt_trans
            } else {
                lt.to_string()
            };
            Ok((i, (ln, lt)))
        }
        std::result::Result::Err(nom::Err::Error(nom::error::Error { input: _, code })) => {
            Err(nom::Err::Error(Error::new(i, code)))
        }
        Err(_) => Err(nom::Err::Error(Error::new(i, ErrorKind::EscapedTransform))),
    }
}

/// Parse a RestructuredText link references.
/// The parser expects to start at the beginning of the line.
/// ```
/// use parse_hyperlinks::parser::restructured_text::rst_link_ref;
/// assert_eq!(
///   rst_link_ref("   .. _`name`: target\nabc"),
///   Ok(("\nabc", ("name".to_string(), "target".to_string())))
/// );
/// ```
/// Here some examples for link references:
/// ```rst
/// .. _Python home page: http://www.python.org
/// .. _`Python: home page`: http://www.python.org
/// ```
/// See unit test `test_rst_link_ref()` for more examples.
/// It returns either `Ok((i, (link_text, link_destination)))` or some error.
pub fn rst_link_ref(i: &str) -> nom::IResult<&str, (String, String)> {
    let (i, block) = rst_explicit_markup_block(i)?;
    match rst_parse_link_ref(block.as_str()) {
        Ok((_, (ln, lt))) => {
            let ln = if let Ok((_, ln_trans)) = rst_escaped_link_name_transform(ln) {
                ln_trans
            } else {
                ln.to_string()
            };
            let lt = if let Ok((_, lt_trans)) = rst_escaped_link_target_transform(lt) {
                lt_trans
            } else {
                lt.to_string()
            };
            Ok((i, (ln, lt)))
        }
        std::result::Result::Err(nom::Err::Error(nom::error::Error { input: _, code })) => {
            Err(nom::Err::Error(Error::new(i, code)))
        }
        Err(_) => Err(nom::Err::Error(nom::error::Error::new(
            i,
            ErrorKind::EscapedTransform,
        ))),
    }
}

/// This parser used by `rst_link()`, does all the work that can be
/// done without allocating new strings.
/// Removing of escaped characters is not performed here.
fn rst_parse_link(i: &str) -> nom::IResult<&str, (&str, &str)> {
    let (i, j) = nom::sequence::delimited(
        tag("`"),
        nom::bytes::complete::escaped(
            nom::character::complete::none_of(r#"\`"#),
            '\\',
            nom::character::complete::one_of(r#" `:<>"#),
        ),
        tag("`_"),
    )(i)?;
    // Consume another optional pending `_`, if there is one. This can not fail.
    let (i, _) = nom::combinator::opt(nom::bytes::complete::tag("_"))(i)?;

    // From here on, we only deal with the inner result of the above.
    // Take everything until the first unescaped `<`
    let (j, link_name): (&str, &str) = nom::bytes::complete::escaped(
        nom::character::complete::none_of(r#"\<"#),
        '\\',
        nom::character::complete::one_of(r#" `:<>"#),
    )(j)?;
    // Trim trailing whitespace.
    let link_name = link_name.trim_end();
    let (j, link_target) = nom::sequence::delimited(
        tag("<"),
        nom::bytes::complete::escaped(
            nom::character::complete::none_of(r#"\<>"#),
            '\\',
            nom::character::complete::one_of(r#" `:<>"#),
        ),
        tag(">"),
    )(j)?;
    // Fail if there are bytes left between `>` and `\``.
    let (_, _) = nom::combinator::eof(j)?;

    Ok((i, (link_name, link_target)))
}

/// This parser detects the position of the link name and the link target.
/// It does not perform any transformation.
/// This parser expects to start at the beginning of the line.
/// If the reference name contains any colons, either:
/// * the phrase must be enclosed in backquotes, or
/// * the colon must be backslash escaped.
/// [reStructuredText Markup
/// Specification](https://docutils.sourceforge.io/docs/ref/rst/restructuredtext.html#hyperlink-targets)
fn rst_parse_link_ref(i: &str) -> nom::IResult<&str, (&str, &str)> {
    let (i, _) = nom::character::complete::char('_')(i)?;
    let (link_target, link_name) = alt((
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

    Ok(("", (link_name, link_target)))
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
fn rst_explicit_markup_block(i: &str) -> nom::IResult<&str, String> {
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

    let (i, v) = nom::multi::separated_list1(
        indent(&wsp1, &wsp2),
        nom::character::complete::not_line_ending,
    )(i)?;

    let mut s = String::new();
    let mut is_first = true;

    for subs in &v {
        if !is_first {
            s.push(' ');
        }
        s.push_str(subs);
        is_first = false;
    }

    Ok((i, s))
}

/// Replace the following escaped characters:
///     \\\`\ \:\<\>
/// with:
///     \`:<>
/// Preserves usual whitespace, but removes `\ `.
fn rst_escaped_link_name_transform(i: &str) -> IResult<&str, String> {
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
    )(i)
}

/// Replace the following escaped characters:
///     \\\`\ \:\<\>
/// with:
///     \` :<>
/// Deletes all whitespace, but keeps one space for each `\ `.
fn rst_escaped_link_target_transform(mut i: &str) -> IResult<&str, String> {
    let mut res = String::new();

    while i != "" {
        let (j, _) = nom::character::complete::space0(i)?;
        let (j, s) = nom::bytes::complete::escaped_transform(
            nom::bytes::complete::is_not("\\ \t"),
            '\\',
            alt((
                value("\\", tag("\\")),
                value("`", tag("`")),
                value(":", tag(":")),
                value("<", tag("<")),
                value(">", tag(">")),
                value(" ", tag(" ")),
            )),
        )(j)?;
        res.push_str(&s);
        i = j;
    }
    Ok(("", res))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom::error::ErrorKind;

    #[test]
    fn test_rst_link() {
        let expected = (
            "abc",
            (
                "Python home page".to_string(),
                "http://www.python.org".to_string(),
            ),
        );
        assert_eq!(
            rst_link("`Python home page <http://www.python.org>`_abc").unwrap(),
            expected
        );
        assert_eq!(
            rst_link("`Python home page <http://www.python.org>`__abc").unwrap(),
            expected
        );

        let expected = (
            "",
            (
                r#"Python<home> page"#.to_string(),
                "http://www.python.org".to_string(),
            ),
        );
        assert_eq!(
            rst_link(r#"`Python\ \<home\> page <http://www.python.org>`_"#).unwrap(),
            expected
        );

        let expected = (
            "",
            (
                r#"my news at <http://python.org>"#.to_string(),
                "http://news.python.org".to_string(),
            ),
        );
        assert_eq!(
            rst_link(r#"`my news at \<http://python.org\> <http://news.python.org>`_"#).unwrap(),
            expected
        );

        let expected = (
            "",
            (
                r#"my news at <http://python.org>"#.to_string(),
                r#"http://news. <python>.org"#.to_string(),
            ),
        );
        assert_eq!(
            rst_link(r#"`my news at \<http\://python.org\> <http:// news.\ \<python\>.org>`_"#)
                .unwrap(),
            expected
        );
    }

    #[test]
    fn test_rst_link_ref() {
        let expected = (
            "\nabc",
            (
                "Python: home page".to_string(),
                "http://www.python.org".to_string(),
            ),
        );
        assert_eq!(
            rst_link_ref(".. _`Python: home page`: http://www.python.org\nabc").unwrap(),
            expected
        );
        assert_eq!(
            rst_link_ref("  .. _`Python: home page`: http://www.py\n     thon.org    \nabc")
                .unwrap(),
            expected
        );

        let expected = nom::Err::Error(nom::error::Error::new(
            "x .. _`Python: home page`: http://www.python.org\nabc",
            ErrorKind::Tag,
        ));
        assert_eq!(
            rst_link_ref("x .. _`Python: home page`: http://www.python.org\nabc").unwrap_err(),
            expected
        );

        let expected = (
            "",
            (
                "Python: `home page`".to_string(),
                "http://www.python .org".to_string(),
            ),
        );
        assert_eq!(
            rst_link_ref(r#".. _Python\: \`home page\`: http://www.python\ .org"#).unwrap(),
            expected
        );
        assert_eq!(
            rst_link_ref(r#".. _`Python: \`home page\``: http://www.python\ .org"#).unwrap(),
            expected
        );

        let expected = (
            "",
            (
                "my news at <http://python.org>".to_string(),
                "http://news.python.org".to_string(),
            ),
        );
        assert_eq!(
            rst_link_ref(r#".. _`my news at <http://python.org>`: http://news.python.org"#)
                .unwrap(),
            expected
        );
        assert_eq!(
            rst_link_ref(r#".. _`my news at \<http://python.org\>`: http://news.python.org"#)
                .unwrap(),
            expected
        );
        assert_eq!(
            rst_link_ref(r#".. _my news at \<http\://python.org\>: http://news.python.org"#)
                .unwrap(),
            expected
        );

        let expected = (
            "",
            (
                "my news".to_string(),
                "http://news.<python>.org".to_string(),
            ),
        );
        assert_eq!(
            rst_link_ref(r#".. _my news: http://news.<python>.org"#).unwrap(),
            expected
        );
        assert_eq!(
            rst_link_ref(r#".. _my news: http://news.\<python\>.org"#).unwrap(),
            expected
        );
    }

    #[test]
    fn test_rst_parse_link() {
        let expected = ("abc", ("Python home page", "http://www.python.org"));
        assert_eq!(
            rst_parse_link("`Python home page <http://www.python.org>`_abc").unwrap(),
            expected
        );

        let expected = ("", (r#"Python\ \<home\> page"#, "http://www.python.org"));
        assert_eq!(
            rst_parse_link(r#"`Python\ \<home\> page <http://www.python.org>`_"#).unwrap(),
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
            rst_parse_link(r#"`my news at \<http://python.org\> <http://news.python.org>`_"#)
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
            rst_parse_link(
                r#"`my news at \<http\://python.org\> <http:// news.\ \<python\>.org>`_"#
            )
            .unwrap(),
            expected
        );
    }

    #[test]
    fn test_rst_parse_link_ref() {
        let expected = ("", ("Python home page", "http://www.python.org"));
        assert_eq!(
            rst_parse_link_ref("_Python home page: http://www.python.org").unwrap(),
            expected
        );
        assert_eq!(
            rst_parse_link_ref("_`Python home page`: http://www.python.org").unwrap(),
            expected
        );

        let expected = ("", ("Python: home page", "http://www.python.org"));
        assert_eq!(
            rst_parse_link_ref("_`Python: home page`: http://www.python.org").unwrap(),
            expected
        );

        let expected = ("", (r#"Python\: home page"#, "http://www.python.org"));
        assert_eq!(
            rst_parse_link_ref(r#"_Python\: home page: http://www.python.org"#).unwrap(),
            expected
        );

        let expected = (
            "",
            ("my news at <http://python.org>", "http://news.python.org"),
        );
        assert_eq!(
            rst_parse_link_ref(r#"_`my news at <http://python.org>`: http://news.python.org"#)
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
            rst_parse_link_ref(r#"_`my news at \<http://python.org\>`: http://news.python.org"#)
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
            rst_parse_link_ref(r#"_my news at \<http\://python.org\>: http://news.python.org"#)
                .unwrap(),
            expected
        );
    }

    #[test]
    fn test_rst_explicit_markup_block() {
        assert_eq!(
            rst_explicit_markup_block(".. 11111"),
            Ok(("", "11111".to_string()))
        );
        assert_eq!(
            rst_explicit_markup_block("   .. 11111\nout"),
            Ok(("\nout", "11111".to_string()))
        );
        assert_eq!(
            rst_explicit_markup_block("   .. 11111\n      222222\n      333333\nout"),
            Ok(("\nout", "11111 222222 333333".to_string()))
        );
        assert_eq!(
            rst_explicit_markup_block("   .. first\n      second\n       1indent\nout"),
            Ok(("\nout", "first second  1indent".to_string()))
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
    fn test_rst_escaped_link_name_transform() {
        assert_eq!(
            rst_escaped_link_name_transform(""),
            Ok(("", "".to_string()))
        );
        // Different than the link target version.
        assert_eq!(
            rst_escaped_link_name_transform("   "),
            Ok(("", "   ".to_string()))
        );
        // Different than the link target version.
        assert_eq!(
            rst_escaped_link_name_transform(r#"\ \ \ "#),
            Ok(("", "".to_string()))
        );
        assert_eq!(
            rst_escaped_link_name_transform(r#"abc`:<>abc"#),
            Ok(("", r#"abc`:<>abc"#.to_string()))
        );
        assert_eq!(
            rst_escaped_link_name_transform(r#"\:\`\<\>\\"#),
            Ok(("", r#":`<>\"#.to_string()))
        );
    }

    #[test]
    fn test_rst_escaped_link_target_transform() {
        assert_eq!(
            rst_escaped_link_target_transform(""),
            Ok(("", "".to_string()))
        );
        // Different than the link name version.
        assert_eq!(
            rst_escaped_link_target_transform("  "),
            Ok(("", "".to_string()))
        );
        // Different than the link name version.
        assert_eq!(
            rst_escaped_link_target_transform(r#"\ \ \ "#),
            Ok(("", "   ".to_string()))
        );
        assert_eq!(
            rst_escaped_link_target_transform(r#"abc`:<>abc"#),
            Ok(("", r#"abc`:<>abc"#.to_string()))
        );
        assert_eq!(
            rst_escaped_link_target_transform(r#"\:\`\<\>\\"#),
            Ok(("", r#":`<>\"#.to_string()))
        );
    }
}
