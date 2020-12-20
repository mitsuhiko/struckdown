---
description: Example markdown file with source code.
---

# Hello Syntect

This is a small example showing syntax highlighting:

```rust
#[test]
fn test_html() {
    insta::glob!("inputs/*.md", |file| {
        let source = fs::read_to_string(file).unwrap();
        let html = to_html(apply_processors(parse(&source)), &Default::default());
        insta::assert_snapshot!(html);
    });
}
```