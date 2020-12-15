use std::fs;

use struckdown::event::{AnnotatedEvent, DocumentStartEvent, Event};
use struckdown::parser::parse;
use struckdown::processors;

use itertools::Either;

fn apply_configured_extensions<'data, I: 'data + Iterator<Item = AnnotatedEvent<'data>>>(
    extensions: Vec<String>,
    iter: I,
) -> impl Iterator<Item = AnnotatedEvent<'data>> {
    let mut iter = Box::new(iter) as Box<dyn Iterator<Item = AnnotatedEvent<'data>>>;

    for extension in extensions {
        if extension == "AutoAnchors" {
            iter = Box::new(processors::AutoAnchors::new(iter));
        } else {
            panic!("unknown extension {}", extension);
        }
    }

    return iter;
}

fn apply_extensions<'data, I: 'data + Iterator<Item = AnnotatedEvent<'data>>>(
    iter: I,
) -> impl Iterator<Item = AnnotatedEvent<'data>> {
    let mut iter = iter.peekable();

    if let Some(document_start) = iter.peek() {
        match document_start.event {
            Event::DocumentStart(DocumentStartEvent {
                front_matter: Some(ref front_matter),
            }) => {
                if let Some(extensions) =
                    front_matter.get("extensions").and_then(|x| x.as_sequence())
                {
                    return Either::Left(apply_configured_extensions(
                        extensions
                            .iter()
                            .map(|x| x.as_str().unwrap().to_owned())
                            .collect(),
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
fn test_basics() {
    insta::glob!("inputs/*.md", |file| {
        let source = fs::read_to_string(file).unwrap();
        let events: Vec<_> = apply_extensions(parse(&source)).collect();
        insta::assert_yaml_snapshot!(events);
    });
}
