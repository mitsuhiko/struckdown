use std::fs;

use struckdown::parser::parse;
use struckdown::renderer::to_html;

#[test]
fn test_basics() {
    insta::glob!("inputs/*.md", |file| {
        let source = fs::read_to_string(file).unwrap();
        let html = to_html(parse(&source));
        insta::assert_snapshot!(html);
    });
}
