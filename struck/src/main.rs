use std::ffi::OsStr;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::{fs, io};

use anyhow::Error;
use argh::FromArgs;

use serde::Deserialize;
use struckdown::html::to_html;
use struckdown::processors::BuiltinProcessor;
use struckdown::{event::AnnotatedEvent, pipeline::Pipeline};

fn read_file<P: AsRef<Path>>(path: &P) -> Result<String, Error> {
    let path = path.as_ref();
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
    Render(RenderCommand),
    Process(ProcessCommand),
}

/// Parses a markdown document.
///
/// This parses a markdown document and emits a JSON event stream.
#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "parse")]
struct ParseCommand {
    /// path to the file to read.
    #[argh(positional, default = "PathBuf::from(\"-\")")]
    path: PathBuf,
}

/// Process according a config file.
///
/// This processes a stream according to a config file.
#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "process")]
struct ProcessCommand {
    /// path to the config file.
    #[argh(positional)]
    config: PathBuf,
}

/// Renders a token stream to HTML.
#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "render")]
struct RenderCommand {
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

fn render_cmd(cmd: RenderCommand) -> Result<(), Error> {
    let source = read_file(&cmd.path)?;
    let events = source
        .lines()
        .map(|line| -> Result<AnnotatedEvent, Error> { Ok(serde_json::from_str(&line)?) })
        .collect::<Result<Vec<_>, _>>()?;
    println!("{}", to_html(events.into_iter(), &Default::default()));
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct ProcessConfig {
    processors: Vec<BuiltinProcessor>,
}

fn process_cmd(cmd: ProcessCommand) -> Result<(), Error> {
    let command_source = read_file(&cmd.config)?;
    let config: ProcessConfig = serde_yaml::from_str(&command_source)?;

    let mut pipeline = Pipeline::new();
    for processor in config.processors {
        pipeline.add_processor(processor);
    }

    let source = read_file(&"-")?;
    let events = source
        .lines()
        .map(|line| -> Result<AnnotatedEvent, Error> { Ok(serde_json::from_str(&line)?) })
        .collect::<Result<Vec<_>, _>>()?;

    for event in pipeline.apply(events.into_iter()) {
        let out = serde_json::to_string(&event)?;
        println!("{}", out);
    }

    Ok(())
}

fn run() -> Result<(), Error> {
    let cli: Cli = argh::from_env();

    match cli.command {
        Command::Parse(args) => parse_cmd(args)?,
        Command::Render(args) => render_cmd(args)?,
        Command::Process(args) => process_cmd(args)?,
    }

    Ok(())
}

fn main() {
    run().unwrap();
}
