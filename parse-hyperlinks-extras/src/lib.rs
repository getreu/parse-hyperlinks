//! Library for parsing hyperlinks and image elements in HTML format.  The library implements
//! [HTML 5.2: 4.7. Embedded content](https://www.w3.org/TR/html52/semantics-embedded-content.html#the-img-element)
//! and extends the parser collection
//! [Parse-hyperlinks](https://crates.io/crates/parse-hyperlinks).

#![allow(dead_code)]

pub mod iterator_html;
pub mod parser;
