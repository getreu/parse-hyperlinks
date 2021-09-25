---
title: "Extra parsers for hyperlinks in lightweight markup"
filename_sync: false
---

# Extra parsers for hyperlinks in lightweight markup

This crate is based on and extends the
[parse-hyperlinks](https://crates.io/crates/parse-hyperlinks) library.
It contains some extra parsers needed for the [Tp-Note](https://crates.io/crates/tp-note)
application.

[Parse-hyperlinks-extras](https://crates.io/crates/parse-hyperlinks-extras),
a parser library written with [Nom](https://crates.io/crates/nom) to
recognize images and hyperlinks in HTML formatted input. For now, only HTML
is implemented.

[![Cargo](https://img.shields.io/crates/v/parse-hyperlinks-extras.svg)](
https://crates.io/crates/parse-hyperlinks-extras)
[![Documentation](https://docs.rs/parse-hyperlinks-extras/badge.svg)](
https://docs.rs/parse-hyperlinks-extras)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](
https://gitlab.com/getreu/parse-hyperlinks-extras)

The library follows the
[HTML 5.2: 4.7. embedded content](https://www.w3.org/TR/html52/semantics-embedded-content.html#the-img-element)
specification. For further details, please consult the
[API documentation](https://docs.rs/parse-hyperlinks-extras/),
and
[parse-hyperlinks-extras on crates.io](https://crates.io/crates/parse-hyperlinks-extras)
