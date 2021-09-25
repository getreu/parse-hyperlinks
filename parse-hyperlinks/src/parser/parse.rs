//! This module implements parsers to extract hyperlinks and link reference
//! definitions from text input. The parsers search for Markdown,
//! ReStructuredText, Asciidoc, Wikitext and HTML formatted links.
#![allow(dead_code)]
#![allow(clippy::type_complexity)]

use crate::parser::asciidoc::adoc_label2dest_link;
use crate::parser::asciidoc::adoc_text2dest_link;
use crate::parser::asciidoc::adoc_text2label_link;
use crate::parser::html::html_text2dest_link;
use crate::parser::markdown::md_label2dest_link;
use crate::parser::markdown::md_text2dest_link;
use crate::parser::markdown::md_text2label_link;
use crate::parser::restructured_text::rst_label2dest_link;
use crate::parser::restructured_text::rst_label2label_link;
use crate::parser::restructured_text::rst_text2dest_link;
use crate::parser::restructured_text::rst_text2label_link;
use crate::parser::restructured_text::rst_text_label2dest_link;
use crate::parser::wikitext::wikitext_text2dest_link;
use crate::parser::Link;
use nom::branch::alt;
use nom::bytes::complete::take_till;
use nom::character::complete::anychar;
use std::borrow::Cow;

/// Link max label. This limits the damage of a forgotten closing brackets.
/// [CommonMark Spec](https://spec.commonmark.org/0.30/#link-label)
pub const LABEL_LEN_MAX: usize = 999;

/// Consumes the input until it finds a Markdown, RestructuredText, Asciidoc or
/// HTML formatted _inline link_ (`Text2Dest`) or _link reference definition_
/// (`Label2Dest`).
///
/// Returns `Ok((remaining_input, (link_text_or_label, link_destination,
/// link_title)))`. The parser recognizes only stand alone _inline links_ and
/// _link reference definitions_, but no _reference links_.
///
/// # Limitations:
/// Link reference labels are never resolved into link text. This limitation only
/// concerns this parser. Others are not affected.
///
/// Very often this limitation has no effect at all. This is the case, when the
/// _link text_ and the _link label_ are identical:
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
/// use parse_hyperlinks::parser::parse::take_text2dest_label2dest;
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
/// link type information and ignores all _reference links_. In case the input
/// text contains _link reference definitions_, this function is be faster than
/// the `parse_hyperlinks::iterator::Hyperlink` iterator.
///
/// Note: This function is depreciated and will be removed in some later release.
/// Use `take_link()` instead.
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
///
/// # Markdown
///
/// ```
/// use parse_hyperlinks::parser::Link;
/// use parse_hyperlinks::parser::parse::take_link;
/// use std::borrow::Cow;
///
/// let i = r#"abc[text1][label1]abc
/// abc[text2](destination2 "title2")
/// [label1]: destination1 'title1'
/// "#;
///
/// let (i, r) = take_link(i).unwrap();
/// assert_eq!(r.0, "abc");
/// assert_eq!(r.1, Link::Text2Label(Cow::from("text1"), Cow::from("label1")));
/// let (i, r) = take_link(i).unwrap();
/// assert_eq!(r.0, "abc\nabc");
/// assert_eq!(r.1, Link::Text2Dest(Cow::from("text2"), Cow::from("destination2"), Cow::from("title2")));
/// let (i, r) = take_link(i).unwrap();
/// assert_eq!(r.0, "\n");
/// assert_eq!(r.1, Link::Label2Dest(Cow::from("label1"), Cow::from("destination1"), Cow::from("title1")));
/// ```
/// # reStructuredText
///
/// ```
/// use parse_hyperlinks::parser::Link;
/// use parse_hyperlinks::parser::parse::take_link;
/// use std::borrow::Cow;
///
/// let i = r#"abc
/// abc `text0 <destination0>`_abc
/// abc `text1 <destination1>`__abc
/// abc `text2 <label2_>`_abc
/// abc text3__ abc
/// .. _label1: destination1
/// .. __: destination3
/// __ destination4
/// "#;
///
/// let (i, r) = take_link(i).unwrap();
/// assert_eq!(r.0, "abc\nabc ");
/// assert_eq!(r.1, Link::TextLabel2Dest(Cow::from("text0"), Cow::from("destination0"), Cow::from("")));
/// let (i, r) = take_link(i).unwrap();
/// assert_eq!(r.1, Link::Text2Dest(Cow::from("text1"), Cow::from("destination1"), Cow::from("")));
/// let (i, r) = take_link(i).unwrap();
/// assert_eq!(r.1, Link::Text2Label(Cow::from("text2"), Cow::from("label2")));
/// let (i, r) = take_link(i).unwrap();
/// assert_eq!(r.1, Link::Text2Label(Cow::from("text3"), Cow::from("_")));
/// let (i, r) = take_link(i).unwrap();
/// assert_eq!(r.1, Link::Label2Dest(Cow::from("label1"), Cow::from("destination1"), Cow::from("")));
/// let (i, r) = take_link(i).unwrap();
/// assert_eq!(r.1, Link::Label2Dest(Cow::from("_"), Cow::from("destination3"), Cow::from("")));
/// let (i, r) = take_link(i).unwrap();
/// assert_eq!(r.1, Link::Label2Dest(Cow::from("_"), Cow::from("destination4"), Cow::from("")));
/// ```
/// # Asciidoc
///
/// ```
/// use parse_hyperlinks::parser::Link;
/// use parse_hyperlinks::parser::parse::take_link;
/// use std::borrow::Cow;
///
/// let i = r#"abc
/// abc https://destination0[text0]abc
/// abc link:https://destination1[text1]abc
/// abc{label2}[text2]abc
/// abc{label3}abc
/// :label4: https://destination4
/// "#;
///
/// let (i, r) = take_link(i).unwrap();
/// assert_eq!(r.0, "abc\nabc ");
/// assert_eq!(r.1, Link::Text2Dest(Cow::from("text0"), Cow::from("https://destination0"), Cow::from("")));
/// let (i, r) = take_link(i).unwrap();
/// assert_eq!(r.1, Link::Text2Dest(Cow::from("text1"), Cow::from("https://destination1"), Cow::from("")));
/// let (i, r) = take_link(i).unwrap();
/// assert_eq!(r.1, Link::Text2Label(Cow::from("text2"), Cow::from("label2")));
/// let (i, r) = take_link(i).unwrap();
/// assert_eq!(r.1, Link::Text2Label(Cow::from(""), Cow::from("label3")));
/// let (i, r) = take_link(i).unwrap();
/// assert_eq!(r.1, Link::Label2Dest(Cow::from("label4"), Cow::from("https://destination4"), Cow::from("")));
/// ```
///
/// # HTML
///
/// ```
/// use parse_hyperlinks::parser::Link;
/// use parse_hyperlinks::parser::parse::take_link;
/// use std::borrow::Cow;
///
/// let i = r#"abc<a href="destination1" title="title1">text1</a>abc
/// abc<a href="destination2" title="title2">text2</a>abc
/// "#;
///
/// let (i, r) = take_link(i).unwrap();
/// assert_eq!(r.0, "abc");
/// assert_eq!(r.1, Link::Text2Dest(Cow::from("text1"), Cow::from("destination1"), Cow::from("title1")));
/// let (i, r) = take_link(i).unwrap();
/// assert_eq!(r.0, "abc\nabc");
/// assert_eq!(r.1, Link::Text2Dest(Cow::from("text2"), Cow::from("destination2"), Cow::from("title2")));
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
        // Does never fail.
        let (k, count) = nom::multi::many0_count(nom::character::complete::newline)(j)?;
        debug_assert_eq!(j.len() - k.len(), count);
        if count > 0 {
            skip_count += j.len() - k.len();
            j = k;
            line_start = true;
        };

        // Are we at the beginning of a line?
        if line_start || input_start {
            if let Ok((k, r)) = alt((
                // Now we search for `label2*`.
                // For both parser is the indent meaningful. We mustn't consume them.
                rst_label2label_link,
                rst_label2dest_link,
            ))(j)
            {
                break (k, r);
            };
        };

        // Are we on a whitespace? Now consume them.
        whitespace = false;
        if let (k, Some(_)) = nom::combinator::opt(nom::character::complete::space1)(j)? {
            skip_count += j.len() - k.len();
            j = k;
            whitespace = true;
        }

        // Are we at the beginning of a line?
        if line_start || input_start {
            if let Ok((k, r)) = alt((
                // Now we search for `label2*`.
                // These parsers do not care about the indent, as long it is
                // only whitespace.
                wikitext_text2dest_link,
                md_label2dest_link,
                adoc_label2dest_link,
            ))(j)
            {
                break (k, r);
            };
        };
        // Start searching for links.

        // Regular `text` links can start everywhere.
        if let Ok((k, r)) = alt((
            // This should be first, because it is very specific.
            wikitext_text2dest_link,
            // Start with `text2dest`.
            md_text2dest_link,
            // `rst_text2dest` must be always placed before `rst_text2label`.
            rst_text2dest_link,
            rst_text_label2dest_link,
            adoc_text2label_link,
            html_text2dest_link,
        ))(j)
        {
            break (k, r);
        };

        if whitespace || line_start || input_start {
            // There must be at least one more byte. If it is one of `([<'"`, skip it.
            let k = if let (k, Some(_)) =
                nom::combinator::opt(nom::character::complete::one_of("([<'\""))(j)?
            {
                // Skip that char.
                k
            } else {
                // Change nothing.
                j
            };

            // `rst_text2label` must be always placed after `rst_text2dest`.
            // `md_text2label` must be always placed after `adoc_text2label` and `adoc_text2dest`,
            // because the former consumes `[*]`.
            if let Ok((l, r)) = alt((rst_text2label_link, adoc_text2dest_link))(k) {
                // If ever we have skipped a char, remember it now.
                skip_count += j.len() - k.len();
                break (l, r);
            };
        };

        // This parser is so unspecific, that it must be the last.
        if let Ok((k, r)) = md_text2label_link(j) {
            break (k, r);
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
            // These could be the start of all `md_*` link types.
            || c == '['
            // These could be the start of the `adoc_text2label` link type.
            || c == '{'
            // And this could be an HTML hyperlink:
            || c == '<')(j)?;

        skip_count += j.len() - k.len();
        j = k;
        input_start = false;
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
    match link {
        Link::Label2Dest(_, _, _) | Link::Label2Label(_, _) => {}
        _ => {
            // Just consume, the result does not matter.
            let (m, _) = nom::combinator::opt(alt((rst_label2dest_link, md_label2dest_link)))(l)?;
            l = m;
        }
    };

    let skipped_input = &i[0..skip_count];

    Ok((l, (skipped_input, link)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_take_link() {
        let expected = nom::Err::Error(nom::error::Error::new("", nom::error::ErrorKind::Eof));
        let err = take_link("").unwrap_err();
        assert_eq!(err, expected);

        let i = r#"[md label1]: md_destination1 "md title1"
abc [md text2](md_destination2 "md title2")[md text3]: abc[md text4]: abc
   [md label5]: md_destination5 "md title5"
abc `rst text1 <rst_destination1>`__abc
abc `rst text2 <rst_label2_>`_ .. _norst: no .. _norst: no
.. _rst label3: rst_destination3
  .. _rst label4: rst_d
     estination4
__ rst_label5_
abc `rst text_label6 <rst_destination6>`_abc
<a href="html_destination1"
   title="html title1">html text1</a>
abc https://adoc_destination1[adoc text1] abc
abc {adoc-label2}abc {adoc-label3}[adoc text3]abc
 :adoc-label4: https://adoc_destination4
abc{adoc-label5}abc https://adoc_destination6 abc
abc[https://wikitext.link Wikitext Testlink]abc
"#;

        let expected = Link::Label2Dest(
            Cow::from("md label1"),
            Cow::from("md_destination1"),
            Cow::from("md title1"),
        );
        let (i, (_, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);

        let expected = Link::Text2Dest(
            Cow::from("md text2"),
            Cow::from("md_destination2"),
            Cow::from("md title2"),
        );
        let (i, (skipped, res)) = take_link(i).unwrap();
        assert_eq!(skipped, "\nabc ");
        assert_eq!(res, expected);

        let expected = Link::Text2Label(Cow::from("md text3"), Cow::from("md text3"));
        let (i, (_, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);

        let expected = Link::Text2Label(Cow::from("md text4"), Cow::from("md text4"));
        let (i, (_, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);

        let expected = Link::Label2Dest(
            Cow::from("md label5"),
            Cow::from("md_destination5"),
            Cow::from("md title5"),
        );
        let (i, (_, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);

        let expected = Link::Text2Dest(
            Cow::from("rst text1"),
            Cow::from("rst_destination1"),
            Cow::from(""),
        );
        let (i, (_, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);

        let expected = Link::Text2Label(Cow::from("rst text2"), Cow::from("rst_label2"));
        let (i, (_, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);

        let expected = Link::Label2Dest(
            Cow::from("rst label3"),
            Cow::from("rst_destination3"),
            Cow::from(""),
        );
        let (i, (_, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);

        let expected = Link::Label2Dest(
            Cow::from("rst label4"),
            Cow::from("rst_destination4"),
            Cow::from(""),
        );
        let (i, (_, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);

        let expected = Link::Label2Label(Cow::from("_"), Cow::from("rst_label5"));
        let (i, (_, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);

        let expected = Link::TextLabel2Dest(
            Cow::from("rst text_label6"),
            Cow::from("rst_destination6"),
            Cow::from(""),
        );
        let (i, (_, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);

        let expected = Link::Text2Dest(
            Cow::from("html text1"),
            Cow::from("html_destination1"),
            Cow::from("html title1"),
        );
        let (i, (_, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);

        let expected = Link::Text2Dest(
            Cow::from("adoc text1"),
            Cow::from("https://adoc_destination1"),
            Cow::from(""),
        );
        let (i, (_, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);

        let expected = Link::Text2Label(Cow::from(""), Cow::from("adoc-label2"));
        let (i, (_, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);

        let expected = Link::Text2Label(Cow::from("adoc text3"), Cow::from("adoc-label3"));
        let (i, (_, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);

        let expected = Link::Label2Dest(
            Cow::from("adoc-label4"),
            Cow::from("https://adoc_destination4"),
            Cow::from(""),
        );
        let (i, (_, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);

        let expected = Link::Text2Label(Cow::from(""), Cow::from("adoc-label5"));
        let (i, (skipped, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);
        assert_eq!(skipped, "\nabc");

        let expected = Link::Text2Dest(
            Cow::from("https://adoc_destination6"),
            Cow::from("https://adoc_destination6"),
            Cow::from(""),
        );
        let (i, (skipped, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);
        assert_eq!(skipped, "abc ");

        let expected = Link::Text2Dest(
            Cow::from("Wikitext Testlink"),
            Cow::from("https://wikitext.link"),
            Cow::from(""),
        );
        let (_i, (skipped, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);
        assert_eq!(skipped, " abc\nabc");
    }

    #[test]
    fn test_take_link2() {
        // New input:
        // Do we find the same at the input start also?
        let i = ".. _`My: home page`: http://getreu.net\nabc";
        let expected = Link::Label2Dest(
            Cow::from("My: home page"),
            Cow::from("http://getreu.net"),
            Cow::from(""),
        );
        let (i, (_, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);
        assert_eq!(i, "\nabc");

        let i = "https://adoc_link_destination[adoc link text]abc";
        let expected = Link::Text2Dest(
            Cow::from("adoc link text"),
            Cow::from("https://adoc_link_destination"),
            Cow::from(""),
        );
        let (i, (_, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);
        assert_eq!(i, "abc");
    }

    #[test]
    fn test_take_link3() {
        let i = r#"   [md label3]: md_destination3 "md title3"
        [md label1]: md_destination1 "md title1"
        [md label2]: md_destination2 "md title2"
        abc[md text_label3]abc[md text_label4]
        "#;

        let expected = Link::Label2Dest(
            Cow::from("md label3"),
            Cow::from("md_destination3"),
            Cow::from("md title3"),
        );
        let (i, (_, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);

        let expected = Link::Label2Dest(
            Cow::from("md label1"),
            Cow::from("md_destination1"),
            Cow::from("md title1"),
        );
        let (i, (_, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);

        let expected = Link::Label2Dest(
            Cow::from("md label2"),
            Cow::from("md_destination2"),
            Cow::from("md title2"),
        );
        let (i, (_, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);

        let expected = Link::Text2Label(Cow::from("md text_label3"), Cow::from("md text_label3"));
        let (i, (_, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);

        let expected = Link::Text2Label(Cow::from("md text_label4"), Cow::from("md text_label4"));
        let (_i, (_, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);
    }

    #[test]
    fn test_take_link4() {
        let i = r#"
.. _label4: label3_
label2__
__ label5
"#;

        let expected = Link::Label2Label(Cow::from("label4"), Cow::from("label3"));
        let (i, (_, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);

        let expected = Link::Text2Label(Cow::from("label2"), Cow::from("_"));
        let (i, (_, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);

        let expected = Link::Label2Dest(Cow::from("_"), Cow::from("label5"), Cow::from(""));
        let (_i, (_, res)) = take_link(i).unwrap();
        assert_eq!(res, expected);
    }
}
