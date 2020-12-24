//! Abstracts event stream modifications.
use crate::event::AnnotatedEvent;
use crate::parser::{Parser, ParserOptions};
use crate::processors::Processor;

/// Helper for applying preconfigured processors to an event stream.
pub struct Pipeline {
    parser: Parser,
    processors: Vec<Box<dyn Processor>>,
}

impl Default for Pipeline {
    fn default() -> Pipeline {
        Pipeline::new()
    }
}

impl Pipeline {
    /// Creates a new pipeline.
    pub fn new() -> Pipeline {
        Pipeline {
            parser: Parser::default(),
            processors: Vec::new(),
        }
    }

    /// Changes the parsing options.
    pub fn set_parser_options(&mut self, parser_options: &ParserOptions) {
        self.parser = Parser::new(parser_options);
    }

    /// Adds a processor to the pipeline
    pub fn add_processor<P: Processor + 'static>(&mut self, processor: P) {
        self.processors.push(Box::new(processor));
    }

    /// Applies the pipeline to a stream consuming the processor.
    pub fn apply<'data, I: Iterator<Item = AnnotatedEvent<'data>> + 'data>(
        self,
        iter: I,
    ) -> Box<dyn Iterator<Item = AnnotatedEvent<'data>> + 'data> {
        let mut iter = Box::new(iter) as Box<dyn Iterator<Item = AnnotatedEvent<'data>>>;
        for processor in self.processors {
            iter = processor.apply(iter);
        }
        iter
    }

    /// Applies the pipeline to a stream.
    pub fn apply_ref<'data, 'options: 'data, I: Iterator<Item = AnnotatedEvent<'data>> + 'data>(
        &'options self,
        iter: I,
    ) -> Box<dyn Iterator<Item = AnnotatedEvent<'data>> + 'data> {
        let mut iter = Box::new(iter) as Box<dyn Iterator<Item = AnnotatedEvent<'data>>>;
        for processor in &self.processors {
            iter = processor.apply_ref(iter);
        }
        iter
    }

    /// Parses and processes a document returning an event stream.
    pub fn process<'data, 'options: 'data>(
        &'options self,
        source: &'data str,
    ) -> Box<dyn Iterator<Item = AnnotatedEvent<'data>> + 'data> {
        self.apply_ref(Box::new(self.parser.parse(source)))
    }
}

#[test]
fn test_basic_pipeline() {
    use crate::html::to_html;
    use crate::processors::BuiltinProcessor;

    let mut pipeline = Pipeline::new();
    pipeline.add_processor(BuiltinProcessor::AutoAnchors(Default::default()));
    insta::assert_snapshot!(to_html(
        pipeline.process("# Hello World!\nAha"),
        &Default::default()
    ));
}
