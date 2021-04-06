//! Module providing an iterator over the hyperlinks found in the input text.
//! This iterator parses only HTML no other markup languages.

use crate::parser::parse_html::take_img_link;
use crate::parser::Link;
use std::borrow::Cow;

#[derive(Debug, PartialEq)]
/// Iterator over the inline images in the HTML formatted `input` text.
/// This struct holds the iterator's state and an advancing pointer into the `input` text.
/// The iterator's `next()` method returns a tuple with 2 tuples inside:
/// `Some(((input_split)(html_image_element)))`.
///
/// Each tuple has the following parts:
/// * `input_split = (skipped_characters, consumed_characters, remaining_characters)`
/// * `html_image_elemet = (img_src, img_alt)`
///
/// # Input split
///
/// ```
/// use parse_hyperlinks::iterator_html::InlineImage;
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
/// # HTML
///
/// ```
/// use parse_hyperlinks::iterator_html::InlineImage;
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
    /// Index where the last output started.
    last_output_offset: usize,
    /// Length of the last output.
    last_output_len: usize,
}

/// Constructor for the `Hyperlink` struct.
impl<'a> InlineImage<'a> {
    /// Constructor for the iterator. `input` is the text with inline images to be
    /// extracted.
    #[inline]
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            last_output_offset: 0,
            last_output_len: 0,
        }
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
impl<'a> Iterator for InlineImage<'a> {
    type Item = ((&'a str, &'a str, &'a str), (Cow<'a, str>, Cow<'a, str>));
    fn next(&mut self) -> Option<Self::Item> {
        let mut output = None;

        if let Ok((remaining_input, (skipped, Link::Image(src, alt)))) = take_img_link(self.input) {
            let consumed = &self.input[skipped.len()..self.input.len() - remaining_input.len()];
            // Assing output.
            output = Some(((skipped, consumed, remaining_input), (src, alt)));
            debug_assert_eq!(self.input, {
                let mut s = "".to_string();
                s.push_str(skipped);
                s.push_str(consumed);
                s.push_str(remaining_input);
                s
            });
            self.input = remaining_input; // Same state, we leave the loop.
        };
        output
    }
}
