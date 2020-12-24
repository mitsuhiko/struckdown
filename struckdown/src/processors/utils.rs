#[macro_export]
macro_rules! implement_processor {
    ($type:ty, $iter:tt) => {
        impl $crate::processors::Processor for $type {
            fn apply<'data>(
                self: Box<Self>,
                iter: Box<dyn Iterator<Item = $crate::event::AnnotatedEvent<'data>> + 'data>,
            ) -> Box<dyn Iterator<Item = $crate::event::AnnotatedEvent<'data>> + 'data> {
                Box::new($iter::new(iter, std::borrow::Cow::Owned(*self)))
            }

            fn apply_ref<'data, 'options: 'data>(
                &'options self,
                iter: Box<dyn Iterator<Item = $crate::event::AnnotatedEvent<'data>> + 'data>,
            ) -> Box<dyn Iterator<Item = $crate::event::AnnotatedEvent<'data>> + 'data> {
                Box::new($iter::new(iter, std::borrow::Cow::Borrowed(self)))
            }
        }
    };
}
