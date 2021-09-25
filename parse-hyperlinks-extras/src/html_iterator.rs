//! Module providing iterators over the hyperlinks found in the input text.
//! Only HTML no other markup languages are parsed here.
#![allow(clippy::type_complexity)]

use crate::parser::parse_html::take_img;
use crate::parser::parse_html::take_img_link;
use crate::parser::parse_html::take_link;
use parse_hyperlinks::parser::Link;
use std::borrow::Cow;

#[derive(Debug, PartialEq)]
/// Iterator over the inline hyperlinks in the HTML formatted `input` text.
/// This struct holds the iterator's state as an advancing pointer into the `input` text.
/// The iterator's `next()` method returns a tuple with 2 tuples inside:
/// `Some(((input_split)(html_hyperlink_element)))`.
///
/// Each tuple has the following parts:
/// * `input_split = (skipped_characters, consumed_characters, remaining_characters)`
/// * `html_hyperlink_element = (text_text, link_destination, link_title)`
///
/// # Input split
///
/// ```
/// use parse_hyperlinks_extras::html_iterator::Hyperlink;
/// use std::borrow::Cow;
///
/// let i = "abc<a href=\"dest1\" title=\"title1\">text1</a>abc\n\
///          abc<a href=\"dest2\" title=\"title2\">text2</a>xyz";
///
/// let mut iter = Hyperlink::new(i);
/// assert_eq!(iter.next().unwrap().0,
///            ("abc",
///             "<a href=\"dest1\" title=\"title1\">text1</a>",
///             "abc\nabc<a href=\"dest2\" title=\"title2\">text2</a>xyz")
///           );
/// assert_eq!(iter.next().unwrap().0,
///            ("abc\nabc",
///             "<a href=\"dest2\" title=\"title2\">text2</a>",
///             "xyz")
///           );
/// assert_eq!(iter.next(), None);
/// ```
/// # Link content
/// ## HTML
///
/// ```
/// use parse_hyperlinks_extras::html_iterator::Hyperlink;
/// use std::borrow::Cow;
///
/// let i = "abc<a href=\"dest1\" title=\"title1\">text1</a>abc\
///          abc<a href=\"dest2\" title=\"title2\">text2</a>abc";
///
///
/// let mut iter = Hyperlink::new(i);
/// assert_eq!(iter.next().unwrap().1, (Cow::from("text1"), Cow::from("dest1"), Cow::from("title1")));
/// assert_eq!(iter.next().unwrap().1, (Cow::from("text2"), Cow::from("dest2"), Cow::from("title2")));
/// assert_eq!(iter.next(), None);
/// ```
pub struct Hyperlink<'a> {
    /// The remaining text input.
    input: &'a str,
}

/// Constructor for the `Hyperlink` struct.
impl<'a> Hyperlink<'a> {
    /// Constructor for the iterator. `input` is the text with inline images to be
    /// extracted.
    #[inline]
    pub fn new(input: &'a str) -> Self {
        Self { input }
    }
}

/// Iterator over the HTML inline images in the `input`-text.
/// The iterator's `next()` method returns a tuple with 2 tuples inside:
/// * `Some(((input_split)(link_content)))`
///
/// Each tuple has the following parts:
/// * `input_split = (skipped_characters, consumed_characters, remaining_characters)`
/// * `link_content = (link_text, link_destination, link_title)`
///
impl<'a> Iterator for Hyperlink<'a> {
    type Item = (
        (&'a str, &'a str, &'a str),
        (Cow<'a, str>, Cow<'a, str>, Cow<'a, str>),
    );
    fn next(&mut self) -> Option<Self::Item> {
        let mut output = None;

        if let Ok((remaining_input, (skipped, Link::Text2Dest(link_text, link_dest, link_title)))) =
            take_link(self.input)
        {
            let consumed = &self.input[skipped.len()..self.input.len() - remaining_input.len()];
            // Assigning output.
            output = Some((
                (skipped, consumed, remaining_input),
                (link_text, link_dest, link_title),
            ));
            debug_assert_eq!(self.input, {
                let mut s = "".to_string();
                s.push_str(skipped);
                s.push_str(consumed);
                s.push_str(remaining_input);
                s
            });
            self.input = remaining_input;
        };
        output
    }
}

#[derive(Debug, PartialEq)]
/// Iterator over the inline images in the HTML formatted `input` text.
/// This struct holds the iterator's state, as an advancing pointer into the `input` text.  The
/// iterator's `next()` method returns a tuple with 2 tuples inside:
/// `Some(((input_split)(html_image_element)))`.
///
/// Each tuple has the following parts:
/// * `input_split = (skipped_characters, consumed_characters, remaining_characters)`
/// * `html_image_element = (img_src, img_alt)`
///
/// # Input split
///
/// ```
/// use parse_hyperlinks_extras::html_iterator::InlineImage;
/// use std::borrow::Cow;
///
/// let i = r#"abc<img src="dest1" alt="text1">efg<img src="dest2" alt="text2">hij"#;
///
/// let mut iter = InlineImage::new(i);
/// assert_eq!(iter.next().unwrap().0, ("abc", r#"<img src="dest1" alt="text1">"#,
///       r#"efg<img src="dest2" alt="text2">hij"#));
/// assert_eq!(iter.next().unwrap().0, ("efg", r#"<img src="dest2" alt="text2">"#,
///       "hij"));
/// assert_eq!(iter.next(), None);
/// ```
/// # Link content
/// ## HTML
///
/// ```
/// use parse_hyperlinks_extras::html_iterator::InlineImage;
/// use std::borrow::Cow;
///
/// let i = r#"abc<img src="dest1" alt="text1">abc
/// abc<img src="dest2" alt="text2">abc
/// "#;
///
/// let mut iter = InlineImage::new(i);
/// assert_eq!(iter.next().unwrap().1, (Cow::from("text1"), Cow::from("dest1")));
/// assert_eq!(iter.next().unwrap().1, (Cow::from("text2"), Cow::from("dest2")));
/// assert_eq!(iter.next(), None);
/// ```
pub struct InlineImage<'a> {
    /// The remaining text input.
    input: &'a str,
}

/// Constructor for the `Hyperlink` struct.
impl<'a> InlineImage<'a> {
    /// Constructor for the iterator. `input` is the text with inline images to be
    /// extracted.
    #[inline]
    pub fn new(input: &'a str) -> Self {
        Self { input }
    }
}

/// Iterator over the HTML inline images in the `input`-text.
/// The iterator's `next()` method returns a tuple with 2 tuples inside:
/// * `Some(((input_split)(link_content)))`
///
/// Each tuple has the following parts:
/// * `input_split = (skipped_characters, consumed_characters, remaining_characters)`
/// * `link_content = (image_alt, image_src)`
///
impl<'a> Iterator for InlineImage<'a> {
    type Item = ((&'a str, &'a str, &'a str), (Cow<'a, str>, Cow<'a, str>));
    fn next(&mut self) -> Option<Self::Item> {
        let mut output = None;

        if let Ok((remaining_input, (skipped, Link::Image(alt, src)))) = take_img(self.input) {
            let consumed = &self.input[skipped.len()..self.input.len() - remaining_input.len()];
            // Assigning output.
            output = Some(((skipped, consumed, remaining_input), (alt, src)));
            debug_assert_eq!(self.input, {
                let mut s = "".to_string();
                s.push_str(skipped);
                s.push_str(consumed);
                s.push_str(remaining_input);
                s
            });
            self.input = remaining_input;
        };
        output
    }
}

#[derive(Debug, PartialEq)]
/// Iterator over the hyperlinks and inline images in the HTML formatted `input` text.
/// This struct holds the iterator's state, as an advancing pointer into the `input` text.  The
/// iterator's `next()` method returns a tuple with another tuples inside:
/// `Some(((input_split), destination))`.
///
/// The tuple has the following parts:
/// * `input_split = (skipped_characters, consumed_characters, remaining_characters)`
///
/// # Input split
///
/// ```
/// use parse_hyperlinks_extras::html_iterator::HyperlinkOrInlineImage;
/// use std::borrow::Cow;
///
/// let i = r#"abc<img src="dest1" alt="text1">abc
/// abc<a href="dest2" title="title2">text2</a>abc"#;
///
/// let mut iter = HyperlinkOrInlineImage::new(i);
/// assert_eq!(iter.next().unwrap().0, ("abc",
///     r#"<img src="dest1" alt="text1">"#,
///     "abc\nabc<a href=\"dest2\" title=\"title2\">text2</a>abc"
///     ));
/// assert_eq!(iter.next().unwrap().0, ("abc\nabc",
///     "<a href=\"dest2\" title=\"title2\">text2</a>",
///     "abc"
///     ));
/// assert_eq!(iter.next(), None);
/// ```
/// # Link content
/// ## HTML
///
/// ```
/// use parse_hyperlinks_extras::html_iterator::HyperlinkOrInlineImage;
/// use std::borrow::Cow;
///
/// let i = r#"abc<img src="dest1" alt="text1">abc
/// abc<a href="dest2" title="title2">text2</a>abc"#;
///
///
/// let mut iter = HyperlinkOrInlineImage::new(i);
/// assert_eq!(iter.next().unwrap().1, (Cow::from("dest1")));
/// assert_eq!(iter.next().unwrap().1, (Cow::from("dest2")));
/// assert_eq!(iter.next(), None);
/// ```
pub struct HyperlinkOrInlineImage<'a> {
    /// The remaining text input.
    input: &'a str,
}

/// Constructor for the `HyperlinkOrInlineImage` struct.
impl<'a> HyperlinkOrInlineImage<'a> {
    /// Constructor for the iterator. `input` is the text with hyperlinks and
    /// inline images to be extracted.
    #[inline]
    pub fn new(input: &'a str) -> Self {
        Self { input }
    }
}

/// Iterator over the HTML hyperlinks and inline images in the `input`-text.
/// The iterator's `next()` method returns a tuple with 2 tuples inside:
/// * `Some(((input_split)(link_content)))`
///
/// Each tuple has the following parts:
/// * `input_split = (skipped_characters, consumed_characters, remaining_characters)`
/// * `link_content = image_src` for inline images or `link_content = destination`
///   for hyperlinks.
///
impl<'a> Iterator for HyperlinkOrInlineImage<'a> {
    type Item = ((&'a str, &'a str, &'a str), Cow<'a, str>);
    fn next(&mut self) -> Option<Self::Item> {
        let mut output = None;

        if let Ok((remaining_input, (skipped, img_link))) = take_img_link(self.input) {
            let dest = match img_link {
                Link::Text2Dest(_, d, _) => d,
                Link::Image(_, s) => s,
                _ => unimplemented!("take_img_link() should not return this variant"),
            };

            let consumed = &self.input[skipped.len()..self.input.len() - remaining_input.len()];
            // Assigning output.
            output = Some(((skipped, consumed, remaining_input), dest));
            debug_assert_eq!(self.input, {
                let mut s = "".to_string();
                s.push_str(skipped);
                s.push_str(consumed);
                s.push_str(remaining_input);
                s
            });
            self.input = remaining_input;
        };
        output
    }
}
