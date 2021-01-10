# Atext2html

[Atext2html](https://crates.io/crates/atext2html) is a command line utility
written with [Nom](https://crates.io/crates/nom) to recognize hyperlinks and
link reference definitions in Markdown, reStructuredText, Asciidoc and HTML
formatted text input. [Atext2html](https://crates.io/crates/atext2html) renders
the source text verbatim to HTML, but makes hyperlinks clickable. By default
the hyperlink's text appears the same as in the source text. When the flag
`--render-links` is given, hyperlinks are represented only by their link text,
which makes inline links more readable.

[![Cargo](https://img.shields.io/crates/v/atext2html.svg)](
https://crates.io/crates/atext2html)
[![Documentation](https://docs.rs/atext2html/badge.svg)](
https://docs.rs/atext2html)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](
https://gitlab.com/getreu/atext2html)

[Atext2html](https://crates.io/crates/atext2html)
illustrates the usage of the underlaying library
[Parse-hyperlinks](https://crates.io/crates/parse-hyperlinks). The
[API of Parse-hyperlinks](https://docs.rs/parse-hyperlinks/0.19.5/parse_hyperlinks/index.html)
provides insights about the operating principle of this utility.

### Installation:

```bash
cargo install atext2html
```

# Usage

```
Render source text with markup hyperlinks.

USAGE:
    atext2html [FLAGS] [OPTIONS] [FILE]...

FLAGS:
    -h, --help            Prints help information
    -l, --only-links      print only links (one per line)
    -r, --render-links    render hyperlinks
    -V, --version         print version and exit

OPTIONS:
    -o, --output <output>    print not to stdout but in file

ARGS:
    <FILE>...    paths to files to render (or `-` for stdin)
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

2. Run `atext2html`:

   ```shell
   $ ./atext2html -o output.html input.txt
   ```

3. Inspect `output.html`:

   ```html
   <pre>abc<a href="destination10" title="title10">[text10](destination10 "title10")</a>abc
   abc<a href="destination1" title="title11">[text11][label11]</a>abc
   abc<a href="destination2" title="title12">[text12](destination2 "title12")</a>
   <a href="destination3" title="title13">[text13]: destination3 "title13"</a>
   <a href="destination1" title="title11">[label11]: destination1 "title11"</a>
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
   <a href="destination3" title="title13">[text13]: destination3 "title13"</a>
   <a href="destination1" title="title11">[label11]: destination1 "title11"</a>
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

2. Run `atext2html`:

   ```shell
   $ ./atext2html -o output.html input.txt
   ```

3. Inspect `output.html`:

   ```html
   <pre>abc <a href="destination21" title="">`text21 &lt;label21_&gt;`_</a>abc
   abc <a href="destination22" title="">text22_</a> abc
   abc <a href="destination23" title="">text23__</a> abc
   abc text_label24_ abc
   abc <a href="destination25" title="">text25__</a> abc
   <a href="destination21" title="">   .. _label21: destination21</a>
   <a href="destination22" title="">   .. _text22: destination22</a>
   <a href="destination23" title="">   .. __: destination23</a>
   <a href="destination25" title="">   __ destination25</a></pre>
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
   <a href="destination21" title="">   .. _label21: destination21</a>
   <a href="destination22" title="">   .. _text22: destination22</a>
   <a href="destination23" title="">   .. __: destination23</a>
   <a href="destination25" title="">   __ destination25</a>
   </pre>


## Asciidoc

1. Create a file `input.txt` with text and hyperlinks:

   ```adoc
   abc
   abc https://destination30[text30]abc
   abc link:https://destination31[text31]abc
   abc{label32}[text32]abc
   abc{label33}abc
   :label32: https://destination32
   :label33: https://destination33
   ```

2. Run `atext2html`:

   ```shell
   $ ./atext2html -o output.html input.txt
   ```

3. Inspect `output.html`:

   ```html
   <pre>abc
   abc <a href="https://destination30" title="">https://destination30[text30]</a>abc
   abc <a href="https://destination31" title="">link:https://destination31[text31]</a>abc
   abc<a href="https://destination32" title="">{label32}[text32]</a>abc
   abc<a href="https://destination33" title="">{label33}</a>abc
   <a href="https://destination32" title="">:label32: https://destination32</a>
   <a href="https://destination33" title="">:label33: https://destination33</a></pre>
   ```

   This is how it looks like in the web-browser:

   ```shell
   $ firefox output.html
   ```

   <pre>
   abc
   abc <a href="https://destination30" title="">https://destination30[text30]</a>abc
   abc <a href="https://destination31" title="">link:https://destination31[text31]</a>abc
   abc<a href="https://destination32" title="">{label32}[text32]</a>abc
   abc<a href="https://destination33" title="">{label33}</a>abc
   <a href="https://destination32" title="">:label32: https://destination32</a>
   <a href="https://destination33" title="">:label33: https://destination33</a>
   </pre>


## HTML

1. Create a file `input.txt` with text and hyperlinks:

   ```adoc
   $ ./atext2html -o output.html input.txt
   ```

2. Run `atext2html`:

   ```shell
   $ ./atext2html <input.txt >output.html
   $ ./atext2html <input.txt >output.html
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
