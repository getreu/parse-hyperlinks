//! Module providing an iterator over the hyperlinks found in the input text.
//! Consult the documentation of `parser::take_link()` to see a list of
//! supported markup languages. The iterator resolves link references.

use crate::parser::take_link;
use crate::parser::Link;
use std::borrow::Cow;
use std::collections::HashMap;
use std::mem::swap;

#[derive(Debug, PartialEq)]
/// A collection of `Link` objects grouped by link type.
struct HyperlinkCollection<'a> {
    /// Vector storing all `Link::Text2Dest`, `Link::Text2Label` and `Link::TextLabel2Dest` links.
    text2dest_label: Vec<Link<'a>>,
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
    fn from(input: &'a str) -> Self {
        let mut input = input;
        let mut hc = HyperlinkCollection::new();
        let mut anonymous_text2label_counter = 0;
        let mut anonymous_label2x_counter = 0;

        while let Ok((i, (_, res))) = take_link(input) {
            match res {
                // `Text2Dest` is stored without modification in `hc.text2dest_label`.
                l if matches!(l, Link::Text2Dest{..}) => hc.text2dest_label.push(l),

                // `Text2label` is stored without modification in `hc.text2dest_label`.
                Link::Text2Label(t, mut l) => {
                    if l == "_" {
                        anonymous_text2label_counter += 1;
                        l = Cow::Owned(format!("_{}", anonymous_text2label_counter));
                    }
                    hc.text2dest_label.push(Link::Text2Label(t, l))
                }
                //`TextLabel2Dest` are cloned and stored in `hc.text2dest_label` as `Text2Dest`
                // and in `hc.label2dest` (repacked in a `HashMap`).
                Link::TextLabel2Dest(tl, d, t) => {
                    hc.text2dest_label
                        .push(Link::Text2Dest(tl.clone(), d.clone(), t.clone()));

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
                    // Silently ignore when overwriting a key that exists already.
                    hc.label2dest.insert(l, (d, t));
                }
                _ => unreachable!(),
            };

            // Prepare next iteration.
            input = i;
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
        while self.label2label.len() > 0 && nb_no_match < self.label2label.len() {
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
            if let Link::Text2Label(text, label) = &self.text2dest_label[idx] {
                if let Some((dest, title)) = &self.label2dest.get(&*label) {
                    let new_link = if text == "" {
                        Link::Text2Dest(dest.clone(), dest.clone(), title.clone())
                    } else {
                        Link::Text2Dest(text.clone(), dest.clone(), title.clone())
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
    /// all links are resolved. The integer index points to the link, that
    /// the iterator's `next()` will return.
    ResolvedLinks(Vec<Link<'a>>),
    /// All links have been returned. From now on only `None` are returned.
    End,
}

#[derive(Debug, PartialEq)]
/// A struct to hold the iterator's state and a pointer to the `input` text, most
/// output refers to with some `&str` slice.
/// # Markdown
/// ```
/// use parse_hyperlinks::iterator::Hyperlink;
/// use std::borrow::Cow;
///
/// let i = r#"abc[text0](destination0 "title0")abc
/// abc[text1][label1]abc
/// abc[text2](destination2 "title2")
///   [text3]: destination3 "title3"
///   [label1]: destination1 "title1"
/// abc[text3]abc
/// "#;
///
/// let mut iter = Hyperlink::new(i);
/// assert_eq!(iter.next(), Some((Cow::from("text0"), Cow::from("destination0"), Cow::from("title0"))));
/// assert_eq!(iter.next(), Some((Cow::from("text1"), Cow::from("destination1"), Cow::from("title1"))));
/// assert_eq!(iter.next(), Some((Cow::from("text2"), Cow::from("destination2"), Cow::from("title2"))));
/// assert_eq!(iter.next(), Some((Cow::from("text3"), Cow::from("destination3"), Cow::from("title3"))));
/// assert_eq!(iter.next(), None);
/// ```
///
/// # reStructuredText
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
///   .. _label1: destination1
///   .. _text2: destination2
///   .. __: destination3
///   __ destination5
/// "#;
///
/// let mut iter = Hyperlink::new(i);
/// assert_eq!(iter.next(), Some((Cow::from("text1"), Cow::from("destination1"), Cow::from(""))));
/// assert_eq!(iter.next(), Some((Cow::from("text2"), Cow::from("destination2"), Cow::from(""))));
/// assert_eq!(iter.next(), Some((Cow::from("text3"), Cow::from("destination3"), Cow::from(""))));
/// assert_eq!(iter.next(), Some((Cow::from("text5"), Cow::from("destination5"), Cow::from(""))));
/// assert_eq!(iter.next(), None);
///
/// ```
/// # Asciidoc
///
/// ```
/// use parse_hyperlinks::iterator::Hyperlink;
/// use std::borrow::Cow;
///
/// let i = r#"abc
/// abc https://destination0[text0]abc
/// abc link:https://destination1[text1]abc
/// abc {label2}[text2]abc
/// abc {label3}abc
/// :label2: https://destination2
/// :label3: https://destination3
/// "#;
///
/// let mut iter = Hyperlink::new(i);
/// assert_eq!(iter.next(), Some((Cow::from("text0"), Cow::from("https://destination0"), Cow::from(""))));
/// assert_eq!(iter.next(), Some((Cow::from("text1"), Cow::from("https://destination1"), Cow::from(""))));
/// assert_eq!(iter.next(), Some((Cow::from("text2"), Cow::from("https://destination2"), Cow::from(""))));
/// assert_eq!(iter.next(), Some((Cow::from("https://destination3"), Cow::from("https://destination3"), Cow::from(""))));
/// assert_eq!(iter.next(), None);
/// ```
pub struct Hyperlink<'a> {
    /// The complete text input.
    input: &'a str,
    /// Status of the `Hyperlink` state machine.
    status: Status<'a>,
}

/// Constructor for the `Hyperlink` struct.
impl<'a> Hyperlink<'a> {
    /// Constructor for the iterator. `input` is the text with hyperlinks to be
    /// extracted.
    #[inline]
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            status: Status::Init,
        }
    }
}

/// Iterator over the hyperlinks (with markup) in the `input`-text.
/// The iterator resolves _link references_.
impl<'a> Iterator for Hyperlink<'a> {
    type Item = (Cow<'a, str>, Cow<'a, str>, Cow<'a, str>);
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
                // Cloning a pointer is cheap.
                // Advance state machine and go again.
                Status::Init => Status::DirectSearch(self.input.clone()),

                Status::DirectSearch(input) => {
                    // We stay in direct mode.
                    if let Ok((remaining_input, (_, Link::Text2Dest(te, de, ti)))) =
                        take_link(input)
                    {
                        output = Some((te, de, ti));
                        // Same state, we leave the loop.
                        again = false;
                        Status::DirectSearch(remaining_input)
                    } else {
                        // We switch to resolving mode.
                        let mut hc = HyperlinkCollection::from(input);
                        hc.resolve_label2label_references();
                        hc.resolve_text2label_references();
                        let mut resolved_links = Vec::new();
                        swap(&mut hc.text2dest_label, &mut resolved_links);

                        // Advance state machine and go again.
                        Status::ResolvedLinks(resolved_links)
                    }
                }

                Status::ResolvedLinks(mut resolved_links) => {
                    while resolved_links.len() > 0 {
                        if let Link::Text2Dest(te, de, ti) = resolved_links.remove(0) {
                            output = Some((te, de, ti));
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
    Hyperlink::new(i).next()
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

        let hc = HyperlinkCollection::from(i);

        let expected = r#"[
    Text2Dest(
        "md text2",
        "md_destination2",
        "md title2",
    ),
    Text2Label(
        "md text3",
        "md text3",
    ),
    Text2Label(
        "md text4",
        "md text4",
    ),
    Text2Dest(
        "rst text1",
        "rst_destination1",
        "",
    ),
    Text2Label(
        "rst text2",
        "rst_label2",
    ),
    Text2Label(
        "rst text5",
        "_1",
    ),
    Text2Label(
        "rst text6",
        "_2",
    ),
    Text2Dest(
        "rst text_label7",
        "rst_destination7",
        "",
    ),
]"#;
        let res = format!("{:#?}", hc.text2dest_label);
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

        let mut hc = HyperlinkCollection::from(i);
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

        let mut hc = HyperlinkCollection::from(i);
        //eprintln!("{:#?}", hc);
        hc.resolve_label2label_references();
        //eprintln!("{:#?}", hc);
        hc.resolve_text2label_references();
        //eprintln!("{:#?}", hc);

        let expected = vec![
            Link::Text2Dest(
                Cow::from("text1"),
                Cow::from("destination1"),
                Cow::from("title1"),
            ),
            Link::Text2Dest(
                Cow::from("text2"),
                Cow::from("destination2"),
                Cow::from("title2"),
            ),
            Link::Text2Dest(
                Cow::from("label3"),
                Cow::from("destination3"),
                Cow::from("title3"),
            ),
            Link::Text2Label(Cow::from("label5"), Cow::from("label5")),
            Link::Text2Dest(
                Cow::from("label4"),
                Cow::from("destination3"),
                Cow::from("title3"),
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

        let mut hc = HyperlinkCollection::from(i);
        //eprintln!("{:#?}", hc);
        hc.resolve_label2label_references();
        //eprintln!("{:#?}", hc);
        hc.resolve_text2label_references();
        //eprintln!("{:#?}", hc);

        let expected = vec![
            Link::Text2Dest(Cow::from("text1"), Cow::from("destination1"), Cow::from("")),
            Link::Text2Dest(
                Cow::from("text_label2"),
                Cow::from("destination2"),
                Cow::from(""),
            ),
            Link::Text2Dest(Cow::from("text3"), Cow::from("destination3"), Cow::from("")),
            Link::Text2Label(Cow::from("text_label4"), Cow::from("text_label4")),
            Link::Text2Dest(Cow::from("text5"), Cow::from("destination5"), Cow::from("")),
        ];
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

        let mut iter = Hyperlink::new(i);

        let expected = Some((Cow::from("text0"), Cow::from("destination0"), Cow::from("")));
        let item = iter.next();
        //eprintln!("item: {:#?}", item);
        assert_eq!(item, expected);

        let expected = Some((
            Cow::from("text1"),
            Cow::from("destination1"),
            Cow::from("title1"),
        ));
        let item = iter.next();
        //eprintln!("item: {:#?}", item);
        assert_eq!(item, expected);

        let expected = Some((
            Cow::from("text2"),
            Cow::from("destination2"),
            Cow::from("title2"),
        ));
        let item = iter.next();
        //eprintln!("item: {:#?}", item);
        assert_eq!(item, expected);

        let expected = Some((
            Cow::from("label3"),
            Cow::from("destination3"),
            Cow::from("title3"),
        ));
        let item = iter.next();
        //eprintln!("item: {:#?}", item);
        assert_eq!(item, expected);

        let expected = Some((
            Cow::from("label4"),
            Cow::from("destination3"),
            Cow::from("title3"),
        ));
        let item = iter.next();
        //eprintln!("item: {:#?}", item);
        assert_eq!(item, expected);

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
