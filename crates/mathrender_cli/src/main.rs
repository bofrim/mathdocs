use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use mathrender_ast::{Position, TextRange};
use mathrender_markdown::RenderEngine;
use mathrender_metadata::load_metadata_for_path;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "mathrender")]
#[command(about = "Render Python math metadata as Markdown and LaTeX")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Render {
        file: PathBuf,
        #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
        format: OutputFormat,
        #[arg(long)]
        range: Option<String>,
    },
    Symbols {
        file: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Check {
        file: PathBuf,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Markdown,
    Latex,
    Json,
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args.command {
        Command::Render {
            file,
            format,
            range,
        } => render(file, format, range),
        Command::Symbols { file, json } => symbols(file, json),
        Command::Check { file } => check(file),
    }
}

fn render(file: PathBuf, format: OutputFormat, range: Option<String>) -> Result<()> {
    let source = std::fs::read_to_string(&file)
        .with_context(|| format!("failed to read {}", file.display()))?;
    let engine = RenderEngine::default();
    let rendered = if let Some(range) = range {
        let parsed_range = parse_line_range(&source, &range)?;
        engine.render_source_range(Some(&file), &file.to_string_lossy(), &source, parsed_range)
    } else {
        engine.render_source(Some(&file), &file.to_string_lossy(), &source)
    };

    match format {
        OutputFormat::Markdown => println!("{}", rendered.markdown),
        OutputFormat::Latex => {
            for block in rendered.blocks {
                if let Some(latex) = block.latex {
                    println!("{latex}");
                }
            }
        }
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&rendered)?),
    }
    Ok(())
}

fn symbols(file: PathBuf, json: bool) -> Result<()> {
    let source = std::fs::read_to_string(&file)
        .with_context(|| format!("failed to read {}", file.display()))?;
    let metadata = load_metadata_for_path(Some(&file), &source);
    if json {
        println!("{}", serde_json::to_string_pretty(&metadata)?);
    } else {
        for symbol in metadata.symbols.values() {
            if let Some(tensor) = &symbol.tensor {
                let indices = tensor
                    .indices
                    .iter()
                    .map(|idx| idx.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                println!("{} => {} ({indices})", symbol.name, symbol.latex);
            } else {
                println!("{} => {}", symbol.name, symbol.latex);
            }
        }
        for function in metadata.functions.values() {
            if let Some(latex) = &function.latex_template {
                println!("{}() => {}", function.qualified_name, latex);
            }
        }
    }
    Ok(())
}

fn check(file: PathBuf) -> Result<()> {
    let rendered = RenderEngine::default().render_path(&file)?;
    if rendered.diagnostics.is_empty() {
        println!("ok");
        return Ok(());
    }
    for diagnostic in rendered.diagnostics {
        println!(
            "{:?}[{}]: {}",
            diagnostic.severity, diagnostic.code, diagnostic.message
        );
    }
    Ok(())
}

fn parse_line_range(source: &str, value: &str) -> Result<TextRange> {
    let (start, end) = value
        .split_once('-')
        .context("range must be formatted as start_line:start_col-end_line:end_col")?;
    let start = parse_position(start)?;
    let end = parse_position(end)?;
    let file = mathrender_ast::SourceFile::new("<range>", source);
    Ok(TextRange {
        start: file.offset_at(start),
        end: file.offset_at(end),
    })
}

fn parse_position(value: &str) -> Result<Position> {
    let (line, character) = value
        .split_once(':')
        .context("position must be formatted as line:character")?;
    Ok(Position {
        line: line.parse::<u32>()?.saturating_sub(1),
        character: character.parse::<u32>()?.saturating_sub(1),
    })
}
