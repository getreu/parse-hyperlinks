//! This module implements parsers to extract hyperlinks and link reference
//! definitions from text input.

pub mod asciidoc;
pub mod html;
pub mod markdown;
pub mod parse;
pub mod restructured_text;
pub mod wikitext;

use std::borrow::Cow;

/// A link can be an _inline link_, a _reference link_, a _link reference
/// definition_, a combined _inline link / link reference definition_, a
/// _reference alias_ or an _inline image_. This is the main return type of this
/// API.
///
/// The _link title_ in Markdown is optional, when not given the string is set
/// to the empty string `""`.  The back ticks \` in reStructuredText can be
/// omitted when only one word is enclosed without spaces.
#[derive(Debug, PartialEq, Clone)]
#[non_exhaustive]
pub enum Link<'a> {
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

    /// Inline Image.
    /// The tuple is defined as follows:
    /// ```text
    /// Image(img_alt, img_src)
    /// ```
    /// Note: this crate does not contain parsers for this variant.
    Image(Cow<'a, str>, Cow<'a, str>),
}
