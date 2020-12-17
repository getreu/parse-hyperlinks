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

# Usage examples

## Markdown

1. Create a file `input.txt`:

   ```md
   abc[text10](destination10 "title10")abc
   abc[text11][label11]abc
   abc[text12](destination2 "title12")
   [text13]: destination3 "title13"
   [label11]: destination1 "title11"
   abc[text13]abc
   ```

2. Run `parse-hyperlinks`:

   ```shell
   $ ./parse-hyperlinks <input.txt >ouput.html
   ```

3. Inspect `output.html`:

   ```html
   <a href="destination10" title="title10">text10</a><br/>
   <a href="destination1" title="title11">text11</a><br/>
   <a href="destination2" title="title12">text12</a><br/>
   <a href="destination3" title="title13">text13</a><br/>
   ```

## reStructuredText

1. Create a file `input.txt`:

   ```rst
   abc `text21 <label21_>`_abc
   abc text22_ abc
   abc text23__ abc
   abc text_label24_ abc
   abc text25__ abc
   .. _label21: destination21
   .. _text22: destination22
   .. __: destination23
   __ destination25
   ```

2. Run `parse-hyperlinks`:

   ```shell
   $ ./parse-hyperlinks <input.txt >ouput.html
   ```

3. Inspect `output.html`:

   ```html
   <a href="destination21" title="">text21</a><br/>
   <a href="destination22" title="">text22</a><br/>
   <a href="destination23" title="">text23</a><br/>
   <a href="destination25" title="">text25</a><br/>
   ```

## Asciidoc

1. Create a file `input.txt`:

   ```adoc
   abc
   abc https://destination30[text30]abc
   abc link:https://destination31[text31]abc
   abc {label32}[text32]abc
   abc {label33}abc
   :label32: https://destination32
   :label33: https://destination33
   ```

2. Run `parse-hyperlinks`:

   ```shell
   $ ./parse-hyperlinks <input.txt >ouput.html
   ```

3. Inspect `output.html`:

   ```html
   <a href="https://destination30" title="">text30</a><br/>
   <a href="https://destination31" title="">text31</a><br/>
   <a href="https://destination32" title="">text32</a><br/>
   <a href="https://destination33" title="">https://destination33</a><br/>
   ```
