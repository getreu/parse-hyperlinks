//! Module providing an iterator over the hyperlinks found in the input text.  Consult the
//! documentation of `parser::parse::take_link()` to see a list of supported markup languages. The
//! iterator resolves link references.

use crate::parser::parse::take_link;
use crate::parser::Link;
use std::borrow::Cow;
use std::collections::HashMap;
use std::mem::swap;

#[derive(Debug, PartialEq)]
/// A collection of `Link` objects grouped by link type.
struct HyperlinkCollection<'a> {
    /// Vector storing all `Link::Text2Dest`, `Link::Text2Label` and `Link::TextLabel2Dest` links.
    /// The tuple is defined as follows: `(link_first_byte_offset, link_len, Link)`.
    text2dest_label: Vec<(usize, usize, Link<'a>)>,
    /// Vector for `Link::Label2Label` links.
    label2label: Vec<(Cow<'a, str>, Cow<'a, str>)>,
    /// Vector for `Link::Label2Dest` and `Link::TextLabel2Dest` links.
    /// The `HashMap`'s key is the `link_label` of the link, the value its
    /// `(link_destination, link_title)`.
    label2dest: HashMap<Cow<'a, str>, (Cow<'a, str>, Cow<'a, str>)>,
}

impl<'a> HyperlinkCollection<'a> {
    fn new() -> Self {
        Self {
            text2dest_label: Vec::new(),
            label2label: Vec::new(),
            label2dest: HashMap::new(),
        }
    }

    /// Reads through the whole `Self::input` and extracts all hyperlinks and
    /// stores them in `Self::HyperlinkCollection` according to their category.
    /// One type is treated specially: `Link::TextLabel2Dest` are cloned and one
    /// copy is stored in `HyperlinkCollection::Text2Dest` and the other copy is
    /// stored in `HyperlinkCollection::Label2Dest`.
    #[inline]
    fn from(input: &'a str, render_label2dest: bool) -> Self {
        let mut i = input;
        let mut hc = HyperlinkCollection::new();
        let mut anonymous_text2label_counter = 0;
        let mut anonymous_label2x_counter = 0;
        // This index refers to `input`.
        let mut input_idx = 0;

        while let Ok((j, (skipped, res))) = take_link(i) {
            match res {
                // `Text2Dest` is stored without modification in `hc.text2dest_label`.
                l if matches!(l, Link::Text2Dest { .. }) => {
                    let link_offset = input_idx + skipped.len();
                    let link_len = i.len() - j.len() - skipped.len();
                    hc.text2dest_label.push((link_offset, link_len, l));
                }

                // `Text2label` is stored without modification in `hc.text2dest_label`.
                Link::Text2Label(text, mut label) => {
                    if label == "_" {
                        anonymous_text2label_counter += 1;
                        label = Cow::Owned(format!("_{}", anonymous_text2label_counter));
                    }
                    let link_offset = input_idx + skipped.len();
                    let link_len = i.len() - j.len() - skipped.len();
                    hc.text2dest_label
                        .push((link_offset, link_len, Link::Text2Label(text, label)))
                }
                //`TextLabel2Dest` are cloned and stored in `hc.text2dest_label` as `Text2Dest`
                // and in `hc.label2dest` (repacked in a `HashMap`).
                Link::TextLabel2Dest(tl, d, t) => {
                    let link_offset = input_idx + skipped.len();
                    let link_len = i.len() - j.len() - skipped.len();
                    hc.text2dest_label.push((
                        link_offset,
                        link_len,
                        Link::Text2Dest(tl.clone(), d.clone(), t.clone()),
                    ));

                    // Silently ignore when overwriting a key that exists already.
                    hc.label2dest.insert(tl, (d, t));
                }

                // `Label2Label` are unpacked and stored in `hc.label2label`.
                Link::Label2Label(mut from, to) => {
                    if from == "_" {
                        anonymous_label2x_counter += 1;
                        from = Cow::Owned(format!("_{}", anonymous_label2x_counter));
                    }
                    hc.label2label.push((from, to));
                }

                // `Label2Dest` are unpacked and stored as `HashMap` in `hc.label2dest`:
                Link::Label2Dest(mut l, d, t) => {
                    if l == "_" {
                        anonymous_label2x_counter += 1;
                        l = Cow::Owned(format!("_{}", anonymous_label2x_counter));
                    }
                    // Some want to have link reference definitions clickable
                    // too. Strictly speaking they are not links, this is why
                    // this is optional.
                    if render_label2dest {
                        let link_offset = input_idx + skipped.len();
                        let link_len = i.len() - j.len() - skipped.len();
                        hc.text2dest_label.push((
                            link_offset,
                            link_len,
                            Link::Text2Dest(
                                Cow::from(&input[link_offset..link_offset + link_len]),
                                d.clone(),
                                t.clone(),
                            ),
                        ));
                    };

                    // Silently ignore when overwriting a key that exists already.
                    hc.label2dest.insert(l, (d, t));
                }
                _ => unreachable!(),
            };

            // Prepare next iteration.
            input_idx += i.len() - j.len();
            i = j;
        }

        hc
    }

    /// Takes one by one, one item from `HyperlinkCollection::label2label` and
    /// searches the corresponding label in `HyperlinkCollection::label2dest`.
    /// When found, add a new item to `HyperlinkCollection::label2dest`. Continue
    /// until `HyperlinkCollection::label2label` is empty or no more corresponding
    /// items can be associated.
    #[inline]
    fn resolve_label2label_references(&mut self) {
        let mut nb_no_match = 0;
        let mut idx = 0;
        while !self.label2label.is_empty() && nb_no_match < self.label2label.len() {
            let (key_alias, key) = &self.label2label[idx];
            // This makes sure, that we advance in the loop.
            if let Some(value) = self.label2dest.get(key) {
                let found_new_key = key_alias.clone();
                let found_value = value.clone();
                // We advance in the loop, because we remove the element `idx` points to.
                self.label2label.remove(idx);
                self.label2dest.insert(found_new_key, found_value);
                // We give up only, after a complete round without match.
                nb_no_match = 0;
            } else {
                // We advance in the loop because we increment `idx`.
                idx += 1;
                nb_no_match += 1;
            };
            // Make sure, that `idx` always points to some valid index.
            if idx >= self.label2label.len() {
                idx = 0;
            }
        }
    }

    /// Takes one by one, one item of type `Link::Text2Label` from
    /// `HyperlinkCollection::text2text_label` and searches the corresponding
    /// label in `HyperlinkCollection::label2dest`. The associated
    /// `Link::Text2Label` and `Link::Label2Dest` are resolved into a new
    /// `Link::Text2Dest` object. Then the item form the fist list is replaced by
    /// this new object. After this operation the
    /// `HyperlinkCollection::text2text_label` list contains only
    /// `Link::Text2Dest` objects (and some unresolvable `Link::Text2Label`
    /// objects).
    #[inline]
    fn resolve_text2label_references(&mut self) {
        let mut idx = 0;
        while idx < self.text2dest_label.len() {
            // If we can not resolve the label, we just skip it.
            if let (input_offset, len, Link::Text2Label(text, label)) = &self.text2dest_label[idx] {
                if let Some((dest, title)) = &self.label2dest.get(&*label) {
                    let new_link = if text == "" {
                        (
                            *input_offset,
                            *len,
                            Link::Text2Dest(dest.clone(), dest.clone(), title.clone()),
                        )
                    } else {
                        (
                            *input_offset,
                            *len,
                            Link::Text2Dest(text.clone(), dest.clone(), title.clone()),
                        )
                    };
                    self.text2dest_label[idx] = new_link;
                };
            };
            // We advance in the loop because we increment `idx`.
            idx += 1;
        }
    }
}

#[derive(Debug, PartialEq)]
/// The interator's state.
enum Status<'a> {
    /// Initial state. Iterator is not started.
    Init,
    /// So far only `Text2Dest` links are coming, no links need to be resolved.
    DirectSearch(&'a str),
    /// As soon as the first reference appears, the remaining text is read and
    /// all links are resolved. The tuple describes a resolved link. The first
    /// integer index points to the first byte of the link in `self.input`, the
    /// second interger is the lenght of the link in `input` bytes. Then follows
    /// the `Link`.
    ResolvedLinks(Vec<(usize, usize, Link<'a>)>),
    /// All links have been returned. From now on only `None` are returned.
    End,
}

#[derive(Debug, PartialEq)]
/// Iterator over all the hyperlinks in the `input` text.
/// This struct holds the iterator's state and an advancing pointer into the `input` text.
/// The iterator's `next()` method returns a tuple with 2 tuples inside:
/// `Some(((input_split)(link_content)))`.
///
/// Each tuple has the following parts:
/// * `input_split = (skipped_characters, consumed_characters, remaining_characters)`
/// * `link_content = (link_text, link_destination, link_title)`
///
/// # Input split
///
/// ```
/// use parse_hyperlinks::iterator::Hyperlink;
/// use std::borrow::Cow;
///
/// let i = "abc[text0](dest0)efg[text1](dest1)hij";
///
/// let mut iter = Hyperlink::new(i, false);
/// assert_eq!(iter.next().unwrap().0, ("abc", "[text0](dest0)", "efg[text1](dest1)hij"));
/// assert_eq!(iter.next().unwrap().0, ("efg", "[text1](dest1)", "hij"));
/// assert_eq!(iter.next(), None);
/// ```
/// # Link content
/// ## Markdown
/// ```
/// use parse_hyperlinks::iterator::Hyperlink;
/// use std::borrow::Cow;
///
/// let i = r#"abc[text0](dest0 "title0")abc
/// abc[text1][label1]abc
/// abc[text2](dest2 "title2")
/// [text3]: dest3 "title3"
/// [label1]: dest1 "title1"
/// abc[text3]abc
/// "#;
///
/// let mut iter = Hyperlink::new(i, false);
/// assert_eq!(iter.next().unwrap().1, (Cow::from("text0"), Cow::from("dest0"), Cow::from("title0")));
/// assert_eq!(iter.next().unwrap().1, (Cow::from("text1"), Cow::from("dest1"), Cow::from("title1")));
/// assert_eq!(iter.next().unwrap().1, (Cow::from("text2"), Cow::from("dest2"), Cow::from("title2")));
/// assert_eq!(iter.next().unwrap().1, (Cow::from("text3"), Cow::from("dest3"), Cow::from("title3")));
/// assert_eq!(iter.next(), None);
/// ```
///
/// ## reStructuredText
///
/// ```
/// use parse_hyperlinks::iterator::Hyperlink;
/// use std::borrow::Cow;
///
/// let i = r#"
/// abc `text1 <label1_>`_abc
/// abc text2_ abc
/// abc text3__ abc
/// abc text_label4_ abc
/// abc text5__ abc
/// .. _label1: dest1
/// .. _text2: dest2
/// .. __: dest3
/// __ dest5
/// "#;
///
/// let mut iter = Hyperlink::new(i, false);
/// assert_eq!(iter.next().unwrap().1, (Cow::from("text1"), Cow::from("dest1"), Cow::from("")));
/// assert_eq!(iter.next().unwrap().1, (Cow::from("text2"), Cow::from("dest2"), Cow::from("")));
/// assert_eq!(iter.next().unwrap().1, (Cow::from("text3"), Cow::from("dest3"), Cow::from("")));
/// assert_eq!(iter.next().unwrap().1, (Cow::from("text5"), Cow::from("dest5"), Cow::from("")));
/// assert_eq!(iter.next(), None);
///
/// ```
/// ## Asciidoc
///
/// ```
/// use parse_hyperlinks::iterator::Hyperlink;
/// use std::borrow::Cow;
///
/// let i = r#"abc
/// abc https://dest0[text0]abc
/// abc link:https://dest1[text1]abc
/// abc {label2}[text2]abc
/// abc {label3}abc
/// :label2: https://dest2
/// :label3: https://dest3
/// "#;
///
/// let mut iter = Hyperlink::new(i, false);
/// assert_eq!(iter.next().unwrap().1, (Cow::from("text0"), Cow::from("https://dest0"), Cow::from("")));
/// assert_eq!(iter.next().unwrap().1, (Cow::from("text1"), Cow::from("https://dest1"), Cow::from("")));
/// assert_eq!(iter.next().unwrap().1, (Cow::from("text2"), Cow::from("https://dest2"), Cow::from("")));
/// assert_eq!(iter.next().unwrap().1, (Cow::from("https://dest3"), Cow::from("https://dest3"), Cow::from("")));
/// assert_eq!(iter.next(), None);
/// ```
///
/// # HTML
///
/// ```
/// use parse_hyperlinks::iterator::Hyperlink;
/// use std::borrow::Cow;
///
/// let i = r#"abc<a href="dest1" title="title1">text1</a>abc
/// abc<a href="dest2" title="title2">text2</a>abc
/// "#;
///
/// let mut iter = Hyperlink::new(i, false);
/// assert_eq!(iter.next().unwrap().1, (Cow::from("text1"), Cow::from("dest1"), Cow::from("title1")));
/// assert_eq!(iter.next().unwrap().1, (Cow::from("text2"), Cow::from("dest2"), Cow::from("title2")));
/// assert_eq!(iter.next(), None);
/// ```
pub struct Hyperlink<'a> {
    /// The remaining text input.
    input: &'a str,
    /// Status of the `Hyperlink` state machine.
    status: Status<'a>,
    /// Index where the last output started.
    last_output_offset: usize,
    /// Length of the last output.
    last_output_len: usize,
    /// By default, `Label2Dest` link reference definitions are not rendered. If
    /// `render_label` is true, then `Label2Dest` is rendered like an inline
    /// link: with the full link reference definition's source as _link text_ and
    /// the definition's destination as _link destination_.
    render_label: bool,
}

/// Constructor for the `Hyperlink` struct.
impl<'a> Hyperlink<'a> {
    /// Constructor for the iterator. `input` is the text with hyperlinks to be
    /// extracted.
    ///
    /// # Optional rendering of Label2Dest link reference definitions
    ///
    /// By default `Label2Dest` link reference definitions are not rendered:
    ///
    /// ```
    /// use parse_hyperlinks::iterator::Hyperlink;
    /// use std::borrow::Cow;
    ///
    /// let i = r#"abc[text1][label1]abc
    /// [label1]: dest1 "title1"
    /// "#;
    ///
    /// let mut iter = Hyperlink::new(i, false);
    /// assert_eq!(iter.next().unwrap().1, (Cow::from("text1"), Cow::from("dest1"), Cow::from("title1")));
    /// assert_eq!(iter.next(), None);
    /// ```
    ///
    /// If the internal variable `render_label` is true, then `Label2Dest` link
    /// reference definitions are rendered like inline links: the full
    /// `Label2Dest` link reference definition's source will appear as _link
    /// text_ and its destination as _link destination_. To enable this feature,
    /// construct `Hyperlink` with the second positional parameter set to `true`,
    /// e.g. `Hyperlink::new(i, true)`.
    ///
    /// ```
    /// use parse_hyperlinks::iterator::Hyperlink;
    /// use std::borrow::Cow;
    ///
    /// let i = r#"abc[text1][label1]abc
    /// [label1]: dest1 "title1"
    /// "#;
    ///
    /// let mut iter = Hyperlink::new(i, true);
    /// assert_eq!(iter.next().unwrap().1, (Cow::from("text1"), Cow::from("dest1"), Cow::from("title1")));
    /// assert_eq!(iter.next().unwrap().1, (Cow::from("[label1]: dest1 \"title1\""), Cow::from("dest1"), Cow::from("title1")));
    /// assert_eq!(iter.next(), None);
    /// ```
    ///
    #[inline]
    pub fn new(input: &'a str, render_label: bool) -> Self {
        Self {
            input,
            status: Status::Init,
            last_output_offset: 0,
            last_output_len: 0,
            render_label,
        }
    }
}

/// Iterator over the hyperlinks (with markup) in the `input`-text.
/// The iterator's `next()` method returns a tuple with 2 tuples inside:
/// * `Some(((input_split)(link_content)))`
///
/// Each tuple has the following parts:
/// * `input_split = (skipped_characters, consumed_characters, remaining_characters)`
/// * `link_content = (link_text, link_destination, link_title)`
///
impl<'a> Iterator for Hyperlink<'a> {
    #[allow(clippy::type_complexity)]
    type Item = (
        (&'a str, &'a str, &'a str),
        (Cow<'a, str>, Cow<'a, str>, Cow<'a, str>),
    );
    /// The iterator operates in 2 modes:
    /// 1. `Status::DirectSearch`: This is the starting state. So far
    ///    the iterator has only encountered inline links so far.
    ///    Nothing needs to be resolved and the next method can
    ///    output the link immediately.
    ///    The `next()` method outputs directly the result from the parser
    ///    `parser::take_link()`.
    /// 2. `Status::ResolvedLinks`: as soon as the iterator encounters
    ///    some reference link, e.g. `Text2label`, `Label2Dest` or
    ///    `Label2Label` link, it switches into `Status::ResolvedLinks` mode.
    ///    The transition happens as follows:
    ///    1. The `next()` method consumes all the remaining `input` and
    ///       calls the `populate_collection()`,
    ///       `resolve_label2label_references()` and
    ///       `resolve_text2label_references()` methods.
    ///       From now on,
    ///    2. the `next()` method outputs and deletes
    ///       `HyperlinkCollection::Dest2Text_label[0]`.
    ///       Not resolved `Text2Label` are ignored.
    fn next(&mut self) -> Option<Self::Item> {
        let mut output = None;
        let mut status = Status::Init;
        swap(&mut status, &mut self.status);

        // Advance state machine.
        let mut again = true;
        while again {
            status = match status {
                // Advance state machine and match one more time.
                Status::Init => Status::DirectSearch(self.input),

                Status::DirectSearch(input) => {
                    // We stay in direct mode.
                    if let Ok((remaining_input, (skipped, Link::Text2Dest(te, de, ti)))) =
                        take_link(input)
                    {
                        let consumed = &input[skipped.len()..input.len() - remaining_input.len()];
                        // Assing output.
                        output = Some(((skipped, consumed, remaining_input), (te, de, ti)));
                        debug_assert_eq!(input, {
                            let mut s = "".to_string();
                            s.push_str(skipped);
                            s.push_str(consumed);
                            s.push_str(remaining_input);
                            s
                        });
                        // Same state, we leave the loop.
                        again = false;
                        Status::DirectSearch(remaining_input)
                    } else {
                        // We switch to resolving mode.
                        self.input = input;
                        let mut hc = HyperlinkCollection::from(input, self.render_label);
                        hc.resolve_label2label_references();
                        hc.resolve_text2label_references();
                        let mut resolved_links = Vec::new();
                        swap(&mut hc.text2dest_label, &mut resolved_links);

                        // Advance state machine and match one more time.
                        Status::ResolvedLinks(resolved_links)
                    }
                }

                Status::ResolvedLinks(mut resolved_links) => {
                    while !resolved_links.is_empty() {
                        if let (input_offset, len, Link::Text2Dest(te, de, ti)) =
                            resolved_links.remove(0)
                        {
                            let skipped = &self.input
                                [(self.last_output_offset + self.last_output_len)..input_offset];
                            let consumed = &self.input[input_offset..input_offset + len];
                            let remaining_input = &self.input[input_offset + len..];
                            // Assign output.
                            output = Some(((skipped, consumed, remaining_input), (te, de, ti)));
                            debug_assert_eq!(self.input, {
                                let mut s = (&self.input
                                    [..self.last_output_offset + self.last_output_len])
                                    .to_string();
                                s.push_str(skipped);
                                s.push_str(consumed);
                                s.push_str(remaining_input);
                                s
                            });
                            self.last_output_offset = input_offset;
                            self.last_output_len = len;
                            break;
                        };
                    }
                    again = false;
                    if output.is_some() {
                        Status::ResolvedLinks(resolved_links)
                    } else {
                        Status::End
                    }
                }

                Status::End => {
                    again = false;
                    output = None;
                    Status::End
                }
            }
        }
        swap(&mut status, &mut self.status);
        output
    }
}

/// Recognizes hyperlinks in all supported markup languages
/// and returns the first hyperlink found as tuple:
/// `Some((link_text, link_destination, link_title))`.
///
/// Returns `None` if no hyperlink is found.
/// This function resolves _link references_.
/// ```
/// use parse_hyperlinks::iterator::first_hyperlink;
/// use std::borrow::Cow;
///
/// let i = r#"abc[t][u]abc
///            [u]: v "w"
///            abc"#;
///
/// let r = first_hyperlink(i);
/// assert_eq!(r, Some((Cow::from("t"), Cow::from("v"), Cow::from("w"))));
/// ```
pub fn first_hyperlink(i: &str) -> Option<(Cow<str>, Cow<str>, Cow<str>)> {
    if let Some((_, (text, dest, title))) = Hyperlink::new(i, false).next() {
        Some((text, dest, title))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_populate_collection() {
        let i = r#"[md label1]: md_destination1 "md title1"
abc [md text2](md_destination2 "md title2")[md text3]: abc[md text4]: abc
   [md label5]: md_destination5 "md title5"
abc `rst text1 <rst_destination1>`__abc
abc `rst text2 <rst_label2_>`_ .. _norst: no .. _norst: no
.. _rst label3: rst_destination3
  .. _rst label4: rst_d
     estination4
__ rst_label5_
__ rst_label6_
abc `rst text5`__abc
abc `rst text6`__abc
abc `rst text_label7 <rst_destination7>`_abc
"#;

        let hc = HyperlinkCollection::from(i, false);

        let expected = r#"[
    (
        45,
        39,
        Text2Dest(
            "md text2",
            "md_destination2",
            "md title2",
        ),
    ),
    (
        84,
        10,
        Text2Label(
            "md text3",
            "md text3",
        ),
    ),
    (
        99,
        10,
        Text2Label(
            "md text4",
            "md text4",
        ),
    ),
    (
        163,
        32,
        Text2Dest(
            "rst text1",
            "rst_destination1",
            "",
        ),
    ),
    (
        203,
        54,
        Text2Label(
            "rst text2",
            "rst_label2",
        ),
    ),
    (
        366,
        13,
        Text2Label(
            "rst text5",
            "_1",
        ),
    ),
    (
        387,
        13,
        Text2Label(
            "rst text6",
            "_2",
        ),
    ),
    (
        408,
        37,
        Text2Dest(
            "rst text_label7",
            "rst_destination7",
            "",
        ),
    ),
]"#;
        let res = format!("{:#?}", hc.text2dest_label);
        //eprintln!("{}", res);
        assert_eq!(hc.text2dest_label.len(), 8);
        assert_eq!(res, expected);

        let expected = r#"[
    (
        "_1",
        "rst_label5",
    ),
    (
        "_2",
        "rst_label6",
    ),
]"#;

        let res = format!("{:#?}", hc.label2label);
        assert_eq!(hc.label2label.len(), 2);
        assert_eq!(res, expected);

        //eprintln!("{:#?}", c.label2dest);
        assert_eq!(hc.label2dest.len(), 5);
        assert_eq!(
            *hc.label2dest.get("md label1").unwrap(),
            (Cow::from("md_destination1"), Cow::from("md title1"))
        );
        assert_eq!(
            *hc.label2dest.get("md label5").unwrap(),
            (Cow::from("md_destination5"), Cow::from("md title5"))
        );
        assert_eq!(
            *hc.label2dest.get("rst label3").unwrap(),
            (Cow::from("rst_destination3"), Cow::from(""))
        );
        assert_eq!(
            *hc.label2dest.get("rst label4").unwrap(),
            (Cow::from("rst_destination4"), Cow::from(""))
        );
        assert_eq!(
            *hc.label2dest.get("rst text_label7").unwrap(),
            (Cow::from("rst_destination7"), Cow::from(""))
        );
    }

    #[test]
    fn test_resolve_label2label_references() {
        let i = r#"label2_
.. _label2: rst_destination2
  .. _label5: label4_
  .. _label1: nolabel_
  .. _label4: label3_
  .. _label3: label2_
"#;

        let mut hc = HyperlinkCollection::from(i, false);
        hc.resolve_label2label_references();
        //eprintln!("{:#?}", hc);
        assert_eq!(hc.label2label.len(), 1);
        assert_eq!(
            hc.label2label[0],
            (Cow::from("label1"), Cow::from("nolabel"))
        );

        assert_eq!(hc.label2dest.len(), 4);
        assert_eq!(
            *hc.label2dest.get("label2").unwrap(),
            (Cow::from("rst_destination2"), Cow::from(""))
        );
        assert_eq!(
            *hc.label2dest.get("label3").unwrap(),
            (Cow::from("rst_destination2"), Cow::from(""))
        );
        assert_eq!(
            *hc.label2dest.get("label4").unwrap(),
            (Cow::from("rst_destination2"), Cow::from(""))
        );
        assert_eq!(
            *hc.label2dest.get("label5").unwrap(),
            (Cow::from("rst_destination2"), Cow::from(""))
        );
    }

    #[test]
    fn test_resolve_text2label_references() {
        let i = r#"abc[text1][label1]abc
        abc [text2](destination2 "title2")
          [label3]: destination3 "title3"
          [label1]: destination1 "title1"
           .. _label4: label3_
        abc[label3]abc[label5]abc
        label4_
        "#;

        let mut hc = HyperlinkCollection::from(i, false);
        //eprintln!("{:#?}", hc);
        hc.resolve_label2label_references();
        //eprintln!("{:#?}", hc);
        hc.resolve_text2label_references();
        //eprintln!("{:#?}", hc);

        let expected = vec![
            (
                3,
                15,
                Link::Text2Dest(
                    Cow::from("text1"),
                    Cow::from("destination1"),
                    Cow::from("title1"),
                ),
            ),
            (
                34,
                30,
                Link::Text2Dest(
                    Cow::from("text2"),
                    Cow::from("destination2"),
                    Cow::from("title2"),
                ),
            ),
            (
                191,
                8,
                Link::Text2Dest(
                    Cow::from("label3"),
                    Cow::from("destination3"),
                    Cow::from("title3"),
                ),
            ),
            (
                202,
                8,
                Link::Text2Label(Cow::from("label5"), Cow::from("label5")),
            ),
            (
                222,
                7,
                Link::Text2Dest(
                    Cow::from("label4"),
                    Cow::from("destination3"),
                    Cow::from("title3"),
                ),
            ),
        ];
        assert_eq!(hc.text2dest_label, expected);
    }

    #[test]
    fn test_resolve_text2label_references2() {
        let i = r#"
abc `text1 <label1_>`_abc
abc text_label2_ abc
abc text3__ abc
abc text_label4_ abc
abc text5__ abc
  .. _label1: destination1
  .. _text_label2: destination2
  .. __: destination3
  __ destination5
        "#;

        let mut hc = HyperlinkCollection::from(i, false);
        //eprintln!("{:#?}", hc);
        hc.resolve_label2label_references();
        //eprintln!("{:#?}", hc);
        hc.resolve_text2label_references();
        //eprintln!("{:#?}", hc);

        let expected = vec![
            (
                5,
                18,
                Link::Text2Dest(Cow::from("text1"), Cow::from("destination1"), Cow::from("")),
            ),
            (
                31,
                12,
                Link::Text2Dest(
                    Cow::from("text_label2"),
                    Cow::from("destination2"),
                    Cow::from(""),
                ),
            ),
            (
                52,
                7,
                Link::Text2Dest(Cow::from("text3"), Cow::from("destination3"), Cow::from("")),
            ),
            (
                68,
                12,
                Link::Text2Label(Cow::from("text_label4"), Cow::from("text_label4")),
            ),
            (
                89,
                7,
                Link::Text2Dest(Cow::from("text5"), Cow::from("destination5"), Cow::from("")),
            ),
        ];
        assert_eq!(hc.text2dest_label, expected);
    }

    #[test]
    fn test_resolve_text2label_references3() {
        let i = r#"
abc[my homepage]abc
abc

[my homepage]: https://getreu.net
abc"#;

        let mut hc = HyperlinkCollection::from(i, false);
        eprintln!("{:#?}", hc);
        hc.resolve_label2label_references();
        //eprintln!("{:#?}", hc);
        hc.resolve_text2label_references();
        //eprintln!("{:#?}", hc);

        let expected = vec![(
            4,
            13,
            Link::Text2Dest(
                Cow::from("my homepage"),
                Cow::from("https://getreu.net"),
                Cow::from(""),
            ),
        )];
        assert_eq!(hc.text2dest_label, expected);
    }

    #[test]
    fn test_next() {
        let i = r#"abc[text0](destination0)abc
abc[text1][label1]abc
abc [text2](destination2 "title2")
  [label3]: destination3 "title3"
  [label1]: destination1 "title1"
   .. _label4: label3_
abc[label3]abc[label5]abc
label4_
        "#;

        let mut iter = Hyperlink::new(i, false);

        let expected = (Cow::from("text0"), Cow::from("destination0"), Cow::from(""));
        let item = iter.next().unwrap();
        //eprintln!("item: {:#?}", item);
        assert_eq!(item.1, expected);

        let expected = (
            Cow::from("text1"),
            Cow::from("destination1"),
            Cow::from("title1"),
        );
        let item = iter.next().unwrap();
        //eprintln!("item: {:#?}", item);
        assert_eq!(item.1, expected);

        let expected = (
            Cow::from("text2"),
            Cow::from("destination2"),
            Cow::from("title2"),
        );
        let item = iter.next().unwrap();
        //eprintln!("item: {:#?}", item);
        assert_eq!(item.1, expected);

        let expected = (
            Cow::from("label3"),
            Cow::from("destination3"),
            Cow::from("title3"),
        );
        let item = iter.next().unwrap();
        //eprintln!("item: {:#?}", item);
        assert_eq!(item.1, expected);

        let expected = (
            Cow::from("label4"),
            Cow::from("destination3"),
            Cow::from("title3"),
        );
        let item = iter.next().unwrap();
        //eprintln!("item: {:#?}", item);
        assert_eq!(item.1, expected);

        let expected = None;
        let item = iter.next();
        //eprintln!("item: {:#?}", item);
        assert_eq!(item, expected);

        let expected = None;
        let item = iter.next();
        //eprintln!("item: {:#?}", item);
        assert_eq!(item, expected);
    }
}
