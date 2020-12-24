# struckdown

struckdown is an experimental markdown/commonmark processing pipeline
library inspired by Python's docutils and Sphinx.  The idea is to
support custom roles and directives for markdown similar to how
how reStructuredText works and to provide an abstraction that can be
processed by standardized tools without having to be aware of how
MarkDown works.

## Contents

Here is what exists today:

- `struckdown`: a Rust library that implements a structured markdown
  processing library based on `pulldown-cmark`.
- `struck`: an experimental command line executable to play around
  with the library.