use std::fs;

use struckdown::event::{AnnotatedEvent, DocumentStartEvent, Event};
use struckdown::html::to_html;
use struckdown::parser::parse;
use struckdown::processors;

use itertools::Either;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(tag = "processor")]
pub enum Processor {
    AutoAnchors(processors::AutoAnchorsOptions),
}

fn apply_configured_processors<'data, I: 'data + Iterator<Item = AnnotatedEvent<'data>>>(
    processors: Vec<serde_yaml::Value>,
    iter: I,
) -> impl Iterator<Item = AnnotatedEvent<'data>> {
    let mut iter = Box::new(iter) as Box<dyn Iterator<Item = AnnotatedEvent<'data>>>;

    for processor in processors {
        let processor: Processor = serde_yaml::from_value(processor).unwrap();
        match processor {
            Processor::AutoAnchors(options) => {
                iter = Box::new(processors::AutoAnchors::new_with_options(iter, options));
            }
        }
    }

    return iter;
}

fn apply_processors<'data, I: 'data + Iterator<Item = AnnotatedEvent<'data>>>(
    iter: I,
) -> impl Iterator<Item = AnnotatedEvent<'data>> {
    let mut iter = iter.peekable();

    if let Some(document_start) = iter.peek() {
        match document_start.event {
            Event::DocumentStart(DocumentStartEvent {
                front_matter: Some(ref front_matter),
            }) => {
                if let Some(processors) =
                    front_matter.get("processors").and_then(|x| x.as_sequence())
                {
                    return Either::Left(apply_configured_processors(
                        processors.iter().map(|x| x.clone()).collect(),
                        iter,
                    ));
                }
            }
            _ => {}
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
        let html = to_html(apply_processors(parse(&source)));
        insta::assert_snapshot!(html);
    });
}
