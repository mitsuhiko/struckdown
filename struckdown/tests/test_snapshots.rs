use std::fs;

use struckdown::event::{AnnotatedEvent, DocumentStartEvent, Event};
use struckdown::html::to_html;
use struckdown::parser::parse;
use struckdown::pipeline::Pipeline;
use struckdown::processors::BuiltinProcessor;
use struckdown::value::Value;

use itertools::Either;

fn apply_configured_processors<'data, I: 'data + Iterator<Item = AnnotatedEvent<'data>>>(
    processors: Vec<Value>,
    iter: I,
) -> impl Iterator<Item = AnnotatedEvent<'data>> {
    let mut pipeline = Pipeline::new();

    for processor in processors {
        pipeline.add_processor(serde_json::from_value::<BuiltinProcessor>(processor).unwrap());
    }

    pipeline.apply(Box::new(iter))
}

fn apply_processors<'data, I: 'data + Iterator<Item = AnnotatedEvent<'data>>>(
    iter: I,
) -> impl Iterator<Item = AnnotatedEvent<'data>> {
    let mut iter = iter.peekable();

    if let Some(document_start) = iter.peek() {
        if let Event::DocumentStart(DocumentStartEvent {
            front_matter: Some(ref front_matter),
        }) = document_start.event
        {
            if let Some(processors) = front_matter.get("processors").and_then(|x| x.as_array()) {
                return Either::Left(apply_configured_processors(processors.clone(), iter));
            }
        }
    }

    Either::Right(iter)
}

#[test]
fn test_parser() {
    insta::glob!("inputs/*.md", |file| {
        let source = fs::read_to_string(file).unwrap();
        let events: Vec<_> = apply_processors(parse(&source)).collect();
        insta::assert_yaml_snapshot!(events);
    });
}

#[test]
fn test_html() {
    insta::glob!("inputs/*.md", |file| {
        let source = fs::read_to_string(file).unwrap();
        let html = to_html(apply_processors(parse(&source)), &Default::default());
        insta::assert_snapshot!(html);
    });
}
