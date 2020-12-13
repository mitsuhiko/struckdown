use std::ffi::OsStr;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::{fs, io};

use anyhow::Error;
use argh::FromArgs;
use struckdown::event::AnnotatedEvent;
use struckdown::html::to_html;

fn read_file(path: &Path) -> Result<String, Error> {
    if path.as_os_str() == OsStr::new("-") {
        let mut rv = String::new();
        io::stdin().read_to_string(&mut rv)?;
        Ok(rv)
    } else {
        Ok(fs::read_to_string(&path)?)
    }
}

#[derive(FromArgs, Debug)]
/// Small utility to play around with struckdown.
struct Cli {
    #[argh(subcommand)]
    command: Command,
}

#[derive(FromArgs, Debug)]
#[argh(subcommand)]
enum Command {
    Parse(ParseCommand),
    RenderStream(RenderStreamCommand),
}

/// Parses a markdown document.
///
/// This parses a markdown document and emits a JSON event stream.
#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "parse")]
struct ParseCommand {
    #[argh(positional, default = "PathBuf::from(\"-\")")]
    path: PathBuf,
}

/// Renders a token stream to HTML.
#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "render-stream")]
struct RenderStreamCommand {
    #[argh(positional, default = "PathBuf::from(\"-\")")]
    path: PathBuf,
}

fn parse_cmd(cmd: ParseCommand) -> Result<(), Error> {
    let source = read_file(&cmd.path)?;
    for event in struckdown::parser::parse(&source) {
        let out = serde_json::to_string(&event)?;
        println!("{}", out);
    }
    Ok(())
}

fn render_stream_cmd(cmd: RenderStreamCommand) -> Result<(), Error> {
    let source = read_file(&cmd.path)?;
    let events = source
        .lines()
        .map(|line| -> Result<AnnotatedEvent, Error> { Ok(serde_json::from_str(&line)?) })
        .collect::<Result<Vec<_>, _>>()?;
    println!("{}", to_html(events.into_iter()));
    Ok(())
}

fn run() -> Result<(), Error> {
    let cli: Cli = argh::from_env();

    match cli.command {
        Command::Parse(args) => parse_cmd(args)?,
        Command::RenderStream(args) => render_stream_cmd(args)?,
    }

    Ok(())
}

fn main() {
    run().unwrap();
}
