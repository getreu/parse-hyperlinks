# Parse hyperlinks

A parser library written with [Nom](https://crates.io/crates/nom) to
recognize hyperlinks and link reference definitions in Markdown,
reStructuredText, Asciidoc and HTML formatted text input.

[![Cargo](https://img.shields.io/crates/v/parse-hyperlinks.svg)](
https://crates.io/crates/parse-hyperlinks)
[![Documentation](https://docs.rs/parse-hyperlinks/badge.svg)](
https://docs.rs/parse-hyperlinks)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](
https://github.com/getreu/parse-hyperlinks)

The library implements the
[CommonMark Specification 0.29](https://spec.commonmark.org/0.29/),
[reStructuredText Markup Specification](https://docutils.sourceforge.io/docs/ref/rst/restructuredtext.html)
(revision 8571, date 2020-10-28), the specifications in
[Asciidoctor User Manual, chapter 26](https://asciidoctor.org/docs/user-manual/#url) (date 2020-12-03)
and [HTML 5.2: section 4.5](https://www.w3.org/TR/html52/textlevel-semantics.html#the-a-element).

To illustrate the usage and the API of the library, [parse-hyperlinks](https://crates.io/crates/parse-hyperlinks)
comes also with a simple command line application.

Installation:

```bash
cargo install parse-hyperlinks
```

Usage example:

1. Create a file `input.txt`:

   ```text
   abc [my blog](https://getreu.net "blog title")abc
      [my blog]: https://getreu.net "blog title"
   abc`my blog <https://getreu.net>`_abc
     .. _my blog: https://get
        reu.net
   abc<a href="https://getreu.net" title="blog title">my blog</a>abc
   abc https://getreu.net[my blog]abc
   ```

2. Run `parse-hyperlinks`:

   ```shell
   $ ./parse-hyperlinks <input.txt >ouput.html
   ```

3. Inspect `output.html`:

   ```html
   <a href="https://getreu.net" title="blog title">my blog</a><br/>
   <a href="https://getreu.net" title="blog title">my blog</a><br/>
   <a href="https://getreu.net" title="">my blog</a><br/>
   <a href="https://getreu.net" title="">my blog</a><br/>
   <a href="https://getreu.net" title="blog title">my blog</a><br/>
   <a href="https://getreu.net" title="">my blog</a><br/>
   ```
