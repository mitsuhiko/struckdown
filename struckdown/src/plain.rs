use crate::event::{AnnotatedEvent, Str};

/// Renders an event stream to plain text.
pub fn to_plain_text<'data: 'event, 'event, I>(iter: I) -> Str<'data>
where
    I: Iterator<Item = &'event AnnotatedEvent<'data>>,
{
    let mut last_event = None;
    let mut buf = None::<String>;

    for annotated_event in iter {
        last_event = match (last_event, annotated_event.event.raw_text(), &mut buf) {
            (last_event, None, _) => last_event,
            (_, Some(new), Some(ref mut buf)) => {
                buf.push_str(new.as_str());
                None
            }
            (None, Some(raw), _) => Some(raw.clone()),
            (Some(last), Some(new), buf) => {
                let mut new_buf = String::new();
                new_buf.push_str(last.as_str());
                new_buf.push_str(new.as_str());
                *buf = Some(new_buf);
                None
            }
        };
    }

    match (buf, last_event) {
        (Some(buf), _) => buf.into(),
        (None, Some(last_event)) => last_event,
        _ => "".into(),
    }
}
