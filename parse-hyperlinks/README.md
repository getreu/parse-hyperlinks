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


## The Parse-Hyperlinks input contract

1. All input is UTF-8 encoded.

2. The input text is formatted according to one of the markup language
   specification above. As Parse-Hyperlinks ignores most of the markup, it
   relies solely on the hyperlink specification of the respective markup
   language.



## General HTML requirements

1. The characters `&<>"` in absolute URLs in HTML documents must be _HTML-
   escape-encoded_: these characters are replaced with their entity names, e.g.
   `&amp;`, `&lt;`, `&gt;` and `&quote`.

2. Relative URLs (local links) in UTF-8 encoded HTML document, do not need to
   be HTML-escape encoded. I recommend not to do so.

3. Relative URLs (local links) must not be preceded by a scheme, e.g. `html:`.

4. In addition to HTML-escape-encoding, URLs can be _percent encoded_, e.g.
   `%20` or `%26`. When both encoding appear in an HTML document, the HTML
   escape decoding is applied first, then the percent encoding. 
   For example, the encoded string `Ü ber%26amp;Über &amp` is decoded to 
   `Ü  ber&amp;Über &`. In general, URLs in UTF-8 HTML documents can be 
   expressed without percent encoding, which is recommended.


      
## Parse-Hyperlinks output guaranties

The following section explains how Parse-Hyperlinks meets the above _General
HTML requirements_. It refers to the items in the list above.

1. Only functions in the `renderer` module, _HTML-escape_ encode absolute URLs
   in HTML documents: The characters `&<>"` are replaced with their HTML escape
   entity names, e.g.: `&amp;`, `&lt;`, `&gt;` and `&quote`. All other parsers
   and iterators do not apply HTML-escape-encoding to absolute URLs.

2. No function, parser or iterator in Parse-Hyperlinks applies 
   escape-encoding to relative URLs.

3. This property is not enforced by Parse-Hyperlinks. Compliance depend on
   the parser's input.

4. Percent-encoding in Parse-Hyperlinks:
  
   * No _percent encoding_ at all is performed in Parse-Hyperlinks.
   
   * _Percent decoding_: In some cases, when the markup language specification
     requires the input URL to be percent encoded, the concerned consuming
     parser decodes the percent encoding automatically.
     Percent decoding is URL's is performed implicitly when consuming:
     * Markdown autolinks when parsed by: `md_text2dest()`,
     * Asciidoc URLs when parsed by: `adoc_label2dest()`, `adoc_text2dest`
     * WikiText URLs when parsed by: `wikitext_text2dest()`

   * Rendered autolink markup:
   
     1. The same Markdown input may result in different HTML according to
        the renderer. For example: `pulldown-cmark` renders 
        the Markdown autolink  `<http://getreu.net/Ü%20&>` into 
        `<a href="http://getreu.net/%C3%9C%20&amp;">http://getreu.net/Ü%20&amp;</a>`.
        * Observation 1: the rendition contains percent and HTML escape codes.
        * Obesrvation 2: the link destination 
          (`http://getreu.net/%C3%9C%20&amp;`) and the link text
          (`http://getreu.net/Ü%20&amp;`) are slightly different, which has
          to be taken into account when detecting autolinks based on the HTML
          rendition.
  
     2. The Parse-Hyperlinks Markdown renderer gives for the same 
        input `<http://getreu.net/Ü%20&>` a slightly different result: 
        `<a href="http://getreu.net/Ü%20&amp;">http://getreu.net/Ü &amp;</a>`.
        Explanation: first the parser `md_text2dest()` (percent) decodes the
        URL to `http://getreu.net/Ü &` and the renderer function in the module
        `renderer` (HTML-escape) encodes the result into 
        `http://getreu.net/Ü%20&amp;`
