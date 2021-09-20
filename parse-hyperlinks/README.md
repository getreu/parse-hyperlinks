# Parse hyperlinks

[Parse-hyperlinks](https://crates.io/crates/parse-hyperlinks),
a parser library written with [Nom](https://crates.io/crates/nom) to
recognize hyperlinks and link reference definitions in Markdown,
reStructuredText, Asciidoc and HTML formatted text input.

[![Cargo](https://img.shields.io/crates/v/parse-hyperlinks.svg)](
https://crates.io/crates/parse-hyperlinks)
[![Documentation](https://docs.rs/parse-hyperlinks/badge.svg)](
https://docs.rs/parse-hyperlinks)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](
https://gitlab.com/getreu/parse-hyperlinks)

The library implements the
[CommonMark Specification 0.30](https://spec.commonmark.org/0.30/),
[reStructuredText Markup Specification](https://docutils.sourceforge.io/docs/ref/rst/restructuredtext.html)
(revision 8571, date 2020-10-28), the specifications in
[Asciidoctor User Manual, chapter 26](https://asciidoctor.org/docs/user-manual/#url) (date 2020-12-03)
and [HTML 5.2: section 4.5](https://www.w3.org/TR/html52/textlevel-semantics.html#the-a-element).

To illustrate the usage and the
[API of the library](https://docs.rs/parse-hyperlinks/0.19.6/parse_hyperlinks/index.html),
[Parse-hyperlinks](https://crates.io/crates/parse-hyperlinks) comes with a
simple command line application:
[Atext2html](https://crates.io/crates/atext2html)

