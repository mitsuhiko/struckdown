# struckdown

This is an experimental markdown/cmark processing pipeline library inspired by Python's
docutils. The idea is to support custom roles and directives for markdown similar to how
how reStructuredText works.

## Contents

Here is what exists today:

- `struckdown`: a Rust library that implements a structured markdown processing
  library based on `pulldown-cmark`.
- `struck`: an experimental command line executable to play around with the library.

## Example

The functionality of the library is best explained through the `struck` command:

The following command parses a structured markdown file into a JSON event stream:

```
$ struck parse example/video.md > event-stream
```

This command takes a JSON event stream on stdin, processes it according to the
given config file and emits the changed events to stdout:

```
$ struck process example/process.yml < event-stream > new-event-stream
```

This takes an event stream and renders it to HTML:

```
$ struck render < new-event-stream
```