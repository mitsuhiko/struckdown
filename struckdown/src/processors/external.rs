use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt::Display;
use std::path::PathBuf;
use std::process::Stdio;

use serde::{Deserialize, Serialize};
use tokio::io::BufReader;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::process::{ChildStdin, ChildStdout, Command};
use tokio::runtime::Runtime;

use crate::event::{AnnotatedEvent, ErrorEvent};
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
    /// Optional environment variables to pass.
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    /// An optional working directory.
    pub cwd: Option<PathBuf>,
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

#[derive(PartialEq)]
enum State {
    Initial,
    Communicating,
    Done,
}

/// The iterator implementing [`External`].
pub struct ExternalIter<'data, 'options, I: Iterator<Item = AnnotatedEvent<'data>>> {
    source: I,
    state: State,
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
            state: State::Initial,
            stdin: None,
            stdout: None,
            buffered_event: None,
            options,
            rt: Some(rt),
        }
    }
}

fn error_event<D: Display>(err: &D, options: &External) -> ErrorEvent<'static> {
    ErrorEvent {
        title: format!(
            "Failed to execute external processor '{}')",
            options.cmd.display()
        )
        .into(),
        description: Some(err.to_string().into()),
    }
}

impl<'data, 'options, I: Iterator<Item = AnnotatedEvent<'data>>> Iterator
    for ExternalIter<'data, 'options, I>
{
    type Item = AnnotatedEvent<'data>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.state {
                State::Done => return None,
                State::Initial => {
                    let rt = self.rt.take().unwrap();
                    let mut error = None;
                    rt.block_on(async {
                        let mut cmd = Command::new(&self.options.cmd);
                        cmd.args(&self.options.args)
                            .stdin(Stdio::piped())
                            .stdout(Stdio::piped())
                            .envs(&self.options.env);
                        if let Some(ref cwd) = self.options.cwd {
                            cmd.current_dir(cwd);
                        }
                        match cmd.spawn() {
                            Ok(mut process) => {
                                self.stdin = process.stdin.take();
                                self.stdout = process.stdout.take().map(BufReader::new);
                            }
                            Err(ref err) => {
                                error = Some(error_event(err, &self.options));
                            }
                        };
                    });

                    if let Some(error) = error {
                        self.state = State::Done;
                        return Some(error.into());
                    } else {
                        self.state = State::Communicating;
                        self.rt = Some(rt);
                        continue;
                    }
                }
                State::Communicating => {
                    if self.buffered_event.is_none() {
                        if let Some(event) = self.source.next() {
                            let mut serialized_event = serde_json::to_vec(&event).expect(
                                "Serializing events to external processors should never fail",
                            );
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
                    let mut done = false;
                    let rt = self.rt.take().unwrap();

                    rt.block_on(async {
                        let mut line = String::new();
                        let should_write = self.buffered_event.is_some();
                        let write_task = async {
                            if let (Some(ref mut stdin), Some(ref buffered_event)) =
                                (&mut stdin, &self.buffered_event)
                            {
                                stdin.write(buffered_event).await.is_err()
                            } else {
                                false
                            }
                        };

                        let should_read = rv.is_none();
                        if should_read {
                            tokio::select! {
                                read = stdout.read_line(&mut line) => {
                                    match read {
                                        Ok(0) | Err(_) => done = true,
                                        Ok(_) => {
                                            rv = Some(match serde_json::from_str(&line) {
                                                Ok(event) => event,
                                                Err(ref err) => {
                                                    self.state = State::Done;
                                                    error_event(err, &self.options).into()
                                                }
                                            })
                                        }
                                    }
                                }
                                failed = write_task, if should_write => {
                                    self.buffered_event.take();
                                    if failed {
                                        self.state = State::Done;
                                        rv = Some(error_event(&"failed to write to subprocess", &self.options).into());
                                    }
                                }
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
                        self.state = State::Done;
                        return None;
                    } else {
                        self.rt = Some(rt);
                    }
                }
            }
        }
    }
}
