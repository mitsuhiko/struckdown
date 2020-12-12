use std::fs;

use structured_cmark::parser::parse;

#[test]
fn test_basics() {
    insta::glob!("inputs/*.md", |file| {
        let source = fs::read_to_string(file).unwrap();
        let events: Vec<_> = parse(&source).collect();
        insta::assert_yaml_snapshot!(events);
    });
}