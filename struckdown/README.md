# struckdown

struckdown is an experimental markdown/commonmark processing pipeline
library inspired by Python's docutils and Sphinx.  The idea is to
support custom roles and directives for markdown similar to how
how reStructuredText works and to provide an abstraction that can be
processed by standardized tools without having to be aware of how
MarkDown works.

## Ideas

A markdown document is parsed into an event stream which can then be
modified by event processors including external tools.  Because the
event stream can be serialized to JSON it's easy to manipulate.

## Example

```rust
use struckdown::pipeline::Pipeline;
use struckdown::processors::{AutoAnchors, Syntect};
use struckdown::html::to_html;

// create a default pipeline
let mut pipeline = Pipeline::default();

// add a processor for anchors and syntax highlighting.
pipeline.add_processor(AutoAnchors::default());
pipeline.add_processor(Syntect::default());

// parse and process into a final event stream.
let stream = pipeline.process(r#"
# Hello World
```python
print("Hello World!");
```"#);

// render to html
let html = to_html(stream, &Default::default());
```