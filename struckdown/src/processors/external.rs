use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::runtime::Runtime;

use tokio::io::BufReader;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::process::{ChildStdin, ChildStdout, Command};

use crate::event::AnnotatedEvent;
use crate::processors::Processor;

/// Passes a JSON serialized stream through an external program.
///
/// When applied this wraps the stream in a [`ExternalIter`].
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct External {
    /// The executable to run.
    pub cmd: PathBuf,
    /// The arguments to pass to the external command.
    #[serde(default)]
    pub args: Vec<String>,
}

impl Processor for External {
    fn apply<'data>(
        self: Box<Self>,
        iter: Box<dyn Iterator<Item = AnnotatedEvent<'data>> + 'data>,
    ) -> Box<dyn Iterator<Item = AnnotatedEvent<'data>> + 'data> {
        Box::new(ExternalIter::new(iter, Cow::Owned(*self)))
    }

    fn apply_ref<'data, 'options: 'data>(
        &'options self,
        iter: Box<dyn Iterator<Item = AnnotatedEvent<'data>> + 'data>,
    ) -> Box<dyn Iterator<Item = AnnotatedEvent<'data>> + 'data> {
        Box::new(ExternalIter::new(iter, Cow::Borrowed(self)))
    }
}

/// The iterator implementing [`External`].
pub struct ExternalIter<'data, 'options, I: Iterator<Item = AnnotatedEvent<'data>>> {
    source: I,
    spawned: bool,
    stdin: Option<ChildStdin>,
    stdout: Option<BufReader<ChildStdout>>,
    buffered_event: Option<Vec<u8>>,
    options: Cow<'options, External>,
    rt: Option<Runtime>,
}

impl<'data, 'options, I: Iterator<Item = AnnotatedEvent<'data>>> ExternalIter<'data, 'options, I> {
    pub fn new(iterator: I, options: Cow<'options, External>) -> Self {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        Self {
            source: iterator,
            spawned: false,
            stdin: None,
            stdout: None,
            buffered_event: None,
            options,
            rt: Some(rt),
        }
    }
}

impl<'data, 'options, I: Iterator<Item = AnnotatedEvent<'data>>> Iterator
    for ExternalIter<'data, 'options, I>
{
    type Item = AnnotatedEvent<'data>;

    fn next(&mut self) -> Option<Self::Item> {
        // TODO: error handling
        // This entire code is a massive hack.  There must be a better way to do this but
        // i'm too lazy to figure this out right now for a small experiment.

        let rt = self.rt.take().unwrap();
        if !self.spawned {
            rt.block_on(async {
                let mut process = Command::new(&self.options.cmd)
                    .args(&self.options.args)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn()
                    .unwrap();
                self.stdin = process.stdin.take();
                self.stdout = process.stdout.take().map(BufReader::new);
                self.spawned = true;
            });
        }

        let mut done = false;
        while !done {
            if self.buffered_event.is_none() {
                if let Some(event) = self.source.next() {
                    let mut serialized_event = serde_json::to_vec(&event).unwrap();
                    serialized_event.push(b'\n');
                    self.buffered_event = Some(serialized_event);
                } else {
                    // close stdin if we're done writing.
                    self.stdin.take();
                }
            }

            let mut stdin = self.stdin.take();
            let mut stdout = self.stdout.take().unwrap();
            let mut rv = None;

            rt.block_on(async {
                let mut line = String::new();

                let should_write = self.buffered_event.is_some();
                let write = async {
                    if let (Some(ref mut stdin), Some(ref buffered_event)) = (&mut stdin, &self.buffered_event) {
                        stdin.write(buffered_event).await.unwrap();
                    }
                };

                tokio::select! {
                    read = stdout.read_line(&mut line) => {
                        match read {
                            Ok(0) | Err(_) => done = true,
                            Ok(_) => {
                                let event: AnnotatedEvent<'_> = serde_json::from_str(&line).unwrap();
                                rv = Some(event);
                            }
                        }
                    }
                    _ = write, if should_write => {
                        self.buffered_event.take();
                    }
                }
            });

            self.stdin = stdin;
            self.stdout = Some(stdout);

            if let Some(rv) = rv {
                self.rt = Some(rt);
                return Some(rv);
            } else if done {
                // close stuff
                self.stdin.take();
                self.stdout.take();
                self.rt = Some(rt);
                return None;
            }
        }

        None
    }
}
