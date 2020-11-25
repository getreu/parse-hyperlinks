# Parse hyperlinks

A parser library written with [Nom](https://crates.io/crates/nom) to recognize
hyperlinks in Markdown or RestructuredText formatted text input.

[![Cargo](https://img.shields.io/crates/v/parse-hyperlinks.svg)](
https://crates.io/crates/parse-hyperlinks)
[![Documentation](https://docs.rs/parse-hyperlinks/badge.svg)](
https://docs.rs/parse-hyperlinks)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](
https://github.com/getreu/parse-hyperlinks)


This library comes with a simple command-line application that illustrates
how to use the library. Installation:

```bash
cargo install parse-hyperlinks
```

Usage example:

```bash
$ cat input.txt
abc [my blog](https://getreu.net "blog title")abc
   [my blog]: https://getreu.net "blog title"
abc`my blog <https://getreu.net>`_abc
  .. _my blog: https://get
     reu.net
$
$ ./parse-hyperlinks <input.txt >ouput.html
$
$ cat ouput.html
<a href="https://getreu.net" title="blog title">my blog</a><br/>
<a href="https://getreu.net" title="blog title">my blog</a><br/>
<a href="https://getreu.net" title="">my blog</a><br/>
<a href="https://getreu.net" title="">my blog</a><br/>
$
```
