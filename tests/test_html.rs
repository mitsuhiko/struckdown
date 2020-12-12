use std::fs;

use structured_cmark::parser::parse;
use structured_cmark::renderer::to_html;

#[test]
fn test_basics() {
    insta::glob!("inputs/*.md", |file| {
        let source = fs::read_to_string(file).unwrap();
        let html = to_html(parse(&source));
        insta::assert_snapshot!(html);
    });
}