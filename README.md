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

1. Create a file `input.txt` with text and hyperlinks:

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
   $ ./parse-hyperlinks <input.txt >output.html
   ```

3. Inspect `output.html`:

   ```html
   <pre>abc<a href="destination10" title="title10">[text10](destination10 "title10")</a>abc
   abc<a href="destination1" title="title11">[text11][label11]</a>abc
   abc<a href="destination2" title="title12">[text12](destination2 "title12")</a>
   [text13]: destination3 "title13"
   [label11]: destination1 "title11"
   abc<a href="destination3" title="title13">[text13]</a>abc</pre>
   ```

   This is how it looks like in the web browser:

   ```shell
   $ firefox output.html
   ```

   <pre>
   abc<a href="destination10" title="title10">[text10](destination10 "title10")</a>abc
   abc<a href="destination1" title="title11">[text11][label11]</a>abc
   abc<a href="destination2" title="title12">[text12](destination2 "title12")</a>
   [text13]: destination3 "title13"
   [label11]: destination1 "title11"
   abc<a href="destination3" title="title13">[text13]</a>abc
   </pre>

## reStructuredText

1. Create a file `input.txt` with text and hyperlinks:

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
   $ ./parse-hyperlinks <input.txt >output.html
   ```

3. Inspect `output.html`:

   ```html
   <pre>abc <a href="destination21" title="">`text21 &lt;label21_&gt;`_</a>abc
   abc <a href="destination22" title="">text22_</a> abc
   abc <a href="destination23" title="">text23__</a> abc
   abc text_label24_ abc
   abc <a href="destination25" title="">text25__</a> abc
   .. _label21: destination21
   .. _text22: destination22
   .. __: destination23
   __ destination25</pre>
   ```

   This is how it looks like in the web browser:

   ```shell
   $ firefox output.html
   ```

   <pre>
   abc <a href="destination21" title="">`text21 &lt;label21_&gt;`_</a>abc
   abc <a href="destination22" title="">text22_</a> abc
   abc <a href="destination23" title="">text23__</a> abc
   abc text_label24_ abc
   abc <a href="destination25" title="">text25__</a> abc
   .. _label21: destination21
   .. _text22: destination22
   .. __: destination23
   __ destination25
   </pre>


## Asciidoc

1. Create a file `input.txt` with text and hyperlinks:

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
   $ ./parse-hyperlinks <input.txt >output.html
   ```

3. Inspect `output.html`:

   ```html
   <pre>abc
   abc <a href="https://destination30" title="">https://destination30[text30]</a>abc
   abc <a href="https://destination31" title="">link:https://destination31[text31]</a>abc
   abc <a href="https://destination32" title="">{label32}[text32]</a>abc
   abc <a href="https://destination33" title="">{label33}</a>abc
   :label32: https://destination32
   :label33: https://destination33</pre>
   ```

   This is how it looks like in the web-browser:

   ```shell
   $ firefox output.html
   ```

   <pre>
   abc
   abc <a href="https://destination30" title="">https://destination30[text30]</a>abc
   abc <a href="https://destination31" title="">link:https://destination31[text31]</a>abc
   abc <a href="https://destination32" title="">{label32}[text32]</a>abc
   abc <a href="https://destination33" title="">{label33}</a>abc
   :label32: https://destination32
   :label33: https://destination33
   </pre>


## HTML

1. Create a file `input.txt` with text and hyperlinks:

   ```adoc
   abc<a href="dest1" title="title1">text1</a>abc
   abc<a href="dest2" title="title2">text2</a>abc
   ```

2. Run `parse-hyperlinks`:

   ```shell
   $ ./parse-hyperlinks <input.txt >output.html
   ```

3. Inspect `output.html`:

   ```html
   <pre>abc<a href="dest1" title="title1">&lt;a href="dest1" title="title1"&gt;text1&lt;/a&gt;</a>abc
   abc<a href="dest2" title="title2">&lt;a href="dest2" title="title2"&gt;text2&lt;/a&gt;</a>abc</pre>
   ```

   This is how it looks like in the web-browser:

   ```shell
   $ firefox output.html
   ```

   <pre>
   abc<a href="dest1" title="title1">&lt;a href="dest1" title="title1"&gt;text1&lt;/a&gt;</a>abc
   abc<a href="dest2" title="title2">&lt;a href="dest2" title="title2"&gt;text2&lt;/a&gt;</a>abc
   </pre>