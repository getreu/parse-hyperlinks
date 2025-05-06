//! This module implements parsers to extract hyperlinks and link reference
//! definitions from text input.

pub mod asciidoc;
pub mod html;
pub mod html_img;
pub mod markdown;
pub mod markdown_img;
pub mod parse;
pub mod restructured_text;
pub mod wikitext;
use nom::error::ErrorKind;
use percent_encoding::percent_decode_str;
use std::borrow::Cow;

/// A [hyperlink] with the following variants:
/// * an [inline link] `Text2Dev`,
/// * a [reference link] `Text2Label`,
/// * a [link reference definition] `Label2Dest`,
/// * a [combined inline link / link reference definition] `TextLabel2Dest`,
/// * a [reference alias] `Label2Label`,
/// * an [inline image] `Image` or
/// * an [inline link with embedded inline image] `Image2Dest`
///
/// This is the main return type of this API.
///
/// The _link title_ in Markdown is optional, when not given the string is set
/// to the empty string `""`.  The back ticks \` in reStructuredText can be
/// omitted when only one word is enclosed without spaces.
///
/// [markup hyperlink]: https://spec.commonmark.org/0.30/#links)
/// [reference link]: https://spec.commonmark.org/0.30/#reference-link
/// [link reference definition]: https://spec.commonmark.org/0.30/#link-reference-definition
/// [combined inline link / link reference definition]: https://docutils.sourceforge.io/docs/ref/rst/restructuredtext.html#hyperlink-references
/// [reference alias]: https://docutils.sourceforge.io/docs/ref/rst/restructuredtext.html#hyperlink-references
/// [inline image]: https://spec.commonmark.org/0.30/#images
/// [inline link with embedded inline image]: https://spec.commonmark.org/0.30/#example-519
#[derive(Debug, PartialEq, Clone)]
#[non_exhaustive]
pub enum Link<'a> {
    /// An _inline link_ with the following tuple values:
    /// ```text
    /// Text2Dest(link_text, link_destination, link_title)
    /// ```
    /// In (stand alone) **inline links** the destination and title are given
    /// immediately after the link text. When an _inline link_ is rendered, only
    /// the `link_text` is visible in the continuous text.
    /// * Markdown example:
    ///   ```md
    ///       [link_text](link_dest "link title")
    ///   ```
    /// * reStructuredText example:
    ///   ```rst
    ///       `link_text <link_dest>`__
    ///   ```
    /// *  Asciidoc example:
    ///    ```adoc
    ///    http://link_dest[link_text]
    ///    ```
    /// *  Wikitext example:
    ///    ```wm
    ///    [http://link_dest link_text]
    ///    ```
    Text2Dest(Cow<'a, str>, Cow<'a, str>, Cow<'a, str>),

    /// A _reference link_ with the following tuple values:
    /// ```text
    /// Text2Label(link_text, link_label)
    /// ```
    /// In **reference links** the destination and title are defined elsewhere
    /// in the document in some _link reference definition_. When a _reference
    /// link_ is rendered only `link_text` is visible.
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
    Text2Label(Cow<'a, str>, Cow<'a, str>),

    /// A _link reference definition_ with the following tuple values:
    /// ```text
    /// Label2Dest(link_label, link_destination, link_title)
    /// ```
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
    Label2Dest(Cow<'a, str>, Cow<'a, str>, Cow<'a, str>),

    /// An _inline link/link reference definition'_ with tuple values:
    /// ```text
    /// Label2Dest(link_text_label, link_destination, link_title)
    /// ```
    /// This type represents a combined **inline link** and **link reference
    /// definition**. Semantically `TextLabel2Dest` is a shorthand for two links
    /// `Text2Dest` and `Label2Dest` in one object, where _link text_ and _link
    /// label_ are the same string. When rendered, _link text_ is visible.
    ///
    /// * Consider the following reStructuredText link:
    ///   ```rst
    ///   `link_text_label <link_dest>`_
    ///
    ///   `a <b>`_
    ///   ```
    ///   In this link is `b` the _link destination_ and `a` has a double role:
    ///   it defines _link text_ of the first link `Text2Dest("a", "b", "")` and
    ///   _link label_ of the second link `Label2Dest("a", "b", "")`.
    ///
    TextLabel2Dest(Cow<'a, str>, Cow<'a, str>, Cow<'a, str>),

    /// A _reference alias_ with the following tuple values:
    /// ```text
    /// Label2Label(alt_link_label, link_label)
    /// ```
    /// The **reference alias** defines an alternative link label
    /// `alt_link_label` for an existing `link_label` defined elsewhere in the
    /// document. At some point, the `link_label` must be resolved to a
    /// `link_destination` by a _link_reference_definition_. A _reference
    /// alias_ is not visible when the document is rendered.
    /// This link type is only available in reStructuredText, e.g.
    /// ```rst
    /// .. _`alt_link_label`: `link_label`_
    /// ```
    Label2Label(Cow<'a, str>, Cow<'a, str>),

    /// An _inline image_ with the following tuple values:
    /// ```text
    /// Image(img_alt, img_src)
    /// ```
    /// Note: this crate does not contain parsers for this variant.
    Image(Cow<'a, str>, Cow<'a, str>),

    /// An _inline link_ with embedded _inline image_ and the following
    /// tuple values.
    /// ```text
    /// Image2Text(text1, img_alt, img_src, text2, dest, title)
    /// ```
    Image2Dest(
        Cow<'a, str>,
        Cow<'a, str>,
        Cow<'a, str>,
        Cow<'a, str>,
        Cow<'a, str>,
        Cow<'a, str>,
    ),
}

/// A parser that decodes percent encoded URLS.
/// This parser consumes all input. It returns `Err` when the percent-decoded
/// bytes are not well-formed in UTF-8.
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

#[test]
fn test_percent_decode() {
    let res = percent_decode("percent%20encoded string").unwrap();
    assert!(matches!(res.1, Cow::Owned(..)));
    assert_eq!(res.1, Cow::from("percent encoded string"));

    let res = percent_decode("nothing").unwrap();
    assert!(matches!(res.1, Cow::Borrowed(..)));
    assert_eq!(res.1, Cow::from("nothing"));
}
