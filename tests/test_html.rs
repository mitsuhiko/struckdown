use std::fs;

use struckdown::html::to_html;
use struckdown::parser::parse;

#[test]
fn test_basics() {
    insta::glob!("inputs/*.md", |file| {
        let source = fs::read_to_string(file).unwrap();
        let html = to_html(parse(&source));
        insta::assert_snapshot!(html);
    });
}
