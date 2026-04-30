use rustpython_parser::{ast, Parse};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextRange {
    pub start: usize,
    pub end: usize,
}

impl TextRange {
    pub fn contains(&self, offset: usize) -> bool {
        self.start <= offset && offset <= self.end
    }

    pub fn overlaps(&self, other: TextRange) -> bool {
        self.start < other.end && other.start < self.end
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    pub range: TextRange,
    pub severity: DiagnosticSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
}

#[derive(Debug, Clone)]
pub struct SourceFile {
    pub path: String,
    pub source: String,
    line_starts: Vec<usize>,
}

impl SourceFile {
    pub fn new(path: impl Into<String>, source: impl Into<String>) -> Self {
        let source = source.into();
        let mut line_starts = vec![0];
        for (idx, byte) in source.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push(idx + 1);
            }
        }
        Self {
            path: path.into(),
            source,
            line_starts,
        }
    }

    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    pub fn line_start(&self, line: usize) -> Option<usize> {
        self.line_starts.get(line).copied()
    }

    pub fn line_text(&self, line: usize) -> Option<&str> {
        let start = *self.line_starts.get(line)?;
        let end = self
            .line_starts
            .get(line + 1)
            .copied()
            .unwrap_or(self.source.len());
        Some(self.source[start..end].trim_end_matches(['\r', '\n']))
    }

    pub fn slice(&self, range: TextRange) -> &str {
        &self.source[range.start..range.end]
    }

    pub fn position_at(&self, offset: usize) -> Position {
        let offset = offset.min(self.source.len());
        let line = match self.line_starts.binary_search(&offset) {
            Ok(line) => line,
            Err(next) => next.saturating_sub(1),
        };
        Position {
            line: line as u32,
            character: (offset - self.line_starts[line]) as u32,
        }
    }

    pub fn offset_at(&self, position: Position) -> usize {
        let line = position.line as usize;
        let Some(start) = self.line_starts.get(line).copied() else {
            return self.source.len();
        };
        let end = self
            .line_starts
            .get(line + 1)
            .copied()
            .unwrap_or(self.source.len());
        (start + position.character as usize).min(end)
    }

    pub fn range(&self, range: TextRange) -> Range {
        Range {
            start: self.position_at(range.start),
            end: self.position_at(range.end),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockKind {
    Markdown {
        content: String,
    },
    Math {
        statement: String,
    },
    Image {
        src: String,
        alt: String,
        title: Option<String>,
        caption: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderConfig {
    pub auto_name_subscript: Option<bool>,
    pub auto_name_symbol: Option<bool>,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            auto_name_subscript: None,
            auto_name_symbol: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderableBlock {
    pub kind: BlockKind,
    pub range: TextRange,
    pub line: usize,
    pub config: RenderConfig,
}

#[derive(Debug, Clone)]
pub struct ParsedModule {
    pub source: SourceFile,
    pub blocks: Vec<RenderableBlock>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn parse_source(path: impl Into<String>, source: impl Into<String>) -> ParsedModule {
    let source = SourceFile::new(path, source);
    let mut diagnostics = Vec::new();

    let syntax_source = sanitize_symbol_lookups_for_python_parse(&source.source);
    if let Err(err) = ast::Suite::parse(&syntax_source, &source.path) {
        let range = TextRange {
            start: 0,
            end: source.source.len().min(1),
        };
        diagnostics.push(Diagnostic {
            code: "mathdocs::syntax".to_string(),
            message: err.to_string(),
            range,
            severity: DiagnosticSeverity::Error,
        });
    }

    let blocks = scan_renderable_blocks(&source);
    ParsedModule {
        source,
        blocks,
        diagnostics,
    }
}

fn sanitize_symbol_lookups_for_python_parse(source: &str) -> String {
    let mut sanitized = String::with_capacity(source.len());
    let mut rest = source;

    while let Some(start) = rest.find("::") {
        sanitized.push_str(&rest[..start]);
        let after_open = &rest[start + 2..];
        let Some(end) = after_open.find("::") else {
            sanitized.push_str(&rest[start..]);
            return sanitized;
        };
        sanitized.push_str("__mathdocs_symbol__");
        rest = &after_open[end + 2..];
    }

    sanitized.push_str(rest);
    sanitized
}

pub fn scan_renderable_blocks(source: &SourceFile) -> Vec<RenderableBlock> {
    let mut blocks = Vec::new();
    let mut line = 0;
    let mut disabled = false;
    let mut ignore_next = false;
    let mut config = RenderConfig::default();

    while line < source.line_count() {
        let Some(text) = source.line_text(line) else {
            break;
        };
        let trimmed = text.trim();

        if let Some(command) = standalone_mathdocs_command(trimmed) {
            match command {
                "off" => disabled = true,
                "on" => disabled = false,
                "ignore" => ignore_next = true,
                _ => apply_mathdocs_config_command(&mut config, command),
            }
            line += 1;
            continue;
        }
        if disabled || trimmed.is_empty() || trimmed.starts_with('#') {
            line += 1;
            continue;
        }
        if ignore_next {
            ignore_next = false;
            line += 1;
            continue;
        }
        if indentation(text) != 0 {
            line += 1;
            continue;
        }

        if let Some((prefix_len, quote)) = triple_string_start(trimmed) {
            let start = source.line_start(line).unwrap_or(0) + text.find(trimmed).unwrap_or(0);
            let mut content = String::new();
            let mut end_line = line;
            let after_open = &trimmed[prefix_len + 3..];
            if let Some(close) = after_open.find(quote) {
                content.push_str(&after_open[..close]);
            } else {
                if !after_open.is_empty() {
                    content.push_str(after_open);
                    content.push('\n');
                }
                end_line += 1;
                while end_line < source.line_count() {
                    let body = source.line_text(end_line).unwrap_or_default();
                    if let Some(close) = body.find(quote) {
                        content.push_str(&body[..close]);
                        break;
                    }
                    content.push_str(body);
                    content.push('\n');
                    end_line += 1;
                }
            }
            let end = source
                .line_start(end_line + 1)
                .unwrap_or(source.source.len());
            blocks.push(RenderableBlock {
                kind: BlockKind::Markdown {
                    content: normalize_markdown_block(&content),
                },
                range: TextRange { start, end },
                line,
                config,
            });
            line = end_line + 1;
            continue;
        }

        if starts_image_directive(trimmed) {
            let start = source.line_start(line).unwrap_or(0) + text.find(trimmed).unwrap_or(0);
            let (statement, end_line) = collect_call_statement(source, line, trimmed);
            if let Some(image) = image_directive(&statement) {
                let end = source
                    .line_start(end_line + 1)
                    .unwrap_or(source.source.len());
                blocks.push(RenderableBlock {
                    kind: image,
                    range: TextRange { start, end },
                    line,
                    config,
                });
            }
            line = end_line + 1;
            continue;
        }

        let (statement, block_config) = split_inline_mathdocs_command(trimmed, config);
        if is_renderable_math_statement(statement) {
            let start = source.line_start(line).unwrap_or(0) + text.find(trimmed).unwrap_or(0);
            let end = source.line_start(line + 1).unwrap_or(source.source.len());
            blocks.push(RenderableBlock {
                kind: BlockKind::Math {
                    statement: statement.to_string(),
                },
                range: TextRange { start, end },
                line,
                config: block_config,
            });
        }

        line += 1;
    }

    blocks
}

fn standalone_mathdocs_command(trimmed: &str) -> Option<&str> {
    trimmed
        .strip_prefix("# mathdocs:")
        .map(str::trim)
        .filter(|command| !command.is_empty())
}

fn split_inline_mathdocs_command(trimmed: &str, mut config: RenderConfig) -> (&str, RenderConfig) {
    let Some(comment) = comment_start(trimmed) else {
        return (trimmed, config);
    };
    let directive = trimmed[comment + 1..].trim();
    let Some(command) = directive.strip_prefix("mathdocs:").map(str::trim) else {
        return (trimmed, config);
    };
    apply_mathdocs_config_command(&mut config, command);
    (trimmed[..comment].trim_end(), config)
}

fn apply_mathdocs_config_command(config: &mut RenderConfig, command: &str) {
    let normalized = command.replace(',', " ");
    let mut current_enabled = None;
    for token in normalized.split_whitespace() {
        let item = if let Some((op, value)) = token.split_once('=') {
            current_enabled = match op {
                "enable" => Some(true),
                "disable" => Some(false),
                _ => None,
            };
            value
        } else {
            token
        };
        let Some(enabled) = current_enabled else {
            continue;
        };
        match item.trim() {
            "auto_name" => {
                config.auto_name_subscript = Some(enabled);
                config.auto_name_symbol = Some(enabled);
            }
            "auto_name/subscript" => config.auto_name_subscript = Some(enabled),
            "auto_name/symbol" => config.auto_name_symbol = Some(enabled),
            _ => {}
        }
    }
}

fn comment_start(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut quote: Option<u8> = None;
    let mut idx = 0usize;
    while idx < bytes.len() {
        let byte = bytes[idx];
        if let Some(q) = quote {
            if byte == b'\\' {
                idx += 2;
                continue;
            }
            if byte == q {
                quote = None;
            }
            idx += 1;
            continue;
        }
        match byte {
            b'\'' | b'"' => quote = Some(byte),
            b'#' => return Some(idx),
            _ => idx += 1,
        }
    }
    None
}

fn indentation(line: &str) -> usize {
    line.chars()
        .take_while(|ch| *ch == ' ' || *ch == '\t')
        .count()
}

fn triple_string_start(trimmed: &str) -> Option<(usize, &'static str)> {
    let lower = trimmed.to_ascii_lowercase();
    let prefixes = ["", "r", "u", "b", "f", "fr", "rf"];
    for prefix in prefixes {
        let Some(rest) = lower.strip_prefix(prefix) else {
            continue;
        };
        if rest.starts_with("\"\"\"") {
            return Some((prefix.len(), "\"\"\""));
        }
        if rest.starts_with("'''") {
            return Some((prefix.len(), "'''"));
        }
    }
    None
}

fn normalize_markdown_block(content: &str) -> String {
    let content = content.trim_matches('\n');
    let lines: Vec<&str> = content.lines().collect();
    let min_indent = lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| indentation(line))
        .min()
        .unwrap_or(0);
    lines
        .iter()
        .map(|line| line.get(min_indent..).unwrap_or(line).trim_end())
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_renderable_math_statement(trimmed: &str) -> bool {
    if trimmed.starts_with("from ")
        || trimmed.starts_with("import ")
        || trimmed.starts_with('@')
        || trimmed.starts_with("def ")
        || trimmed.starts_with("class ")
        || trimmed.starts_with("if ")
        || trimmed.starts_with("else")
        || trimmed.starts_with("elif ")
        || trimmed.starts_with("return ")
        || trimmed.starts_with("pass")
    {
        return false;
    }

    if trimmed.contains("Annotated[") {
        return false;
    }

    contains_top_level_assignment(trimmed) || contains_top_level_compare(trimmed)
}

fn image_directive(trimmed: &str) -> Option<BlockKind> {
    let (name, args) = call_parts(trimmed)?;
    if name != "render_figure" && name != "render_image" && name != "render_plot" {
        return None;
    }

    let args = parse_call_arguments(args)?;
    let src = args.iter().find_map(|arg| match arg {
        CallArgument::Positional(value) => Some(value.clone()),
        CallArgument::Keyword { name, value } if name == "src" => Some(value.clone()),
        _ => None,
    })?;
    let alt = keyword_argument(&args, "alt").unwrap_or_default();
    let title = keyword_argument(&args, "title").filter(|value| !value.is_empty());
    let caption = keyword_argument(&args, "caption").filter(|value| !value.is_empty());

    Some(BlockKind::Image {
        src,
        alt,
        title,
        caption,
    })
}

fn starts_image_directive(trimmed: &str) -> bool {
    trimmed.starts_with("render_figure(")
        || trimmed.starts_with("render_image(")
        || trimmed.starts_with("render_plot(")
}

fn collect_call_statement(
    source: &SourceFile,
    start_line: usize,
    first_line: &str,
) -> (String, usize) {
    let mut statement = first_line.to_string();
    let mut end_line = start_line;
    let mut depth = paren_delta(first_line);

    while depth > 0 && end_line + 1 < source.line_count() {
        end_line += 1;
        let line = source.line_text(end_line).unwrap_or_default().trim();
        statement.push('\n');
        statement.push_str(line);
        depth += paren_delta(line);
    }

    (statement, end_line)
}

fn paren_delta(text: &str) -> i32 {
    let mut depth = 0;
    let mut quote: Option<char> = None;
    let mut escaped = false;

    for ch in text.chars() {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == q {
                quote = None;
            }
            continue;
        }

        match ch {
            '\'' | '"' => quote = Some(ch),
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }
    }

    depth
}

fn call_parts(trimmed: &str) -> Option<(&str, &str)> {
    let open = trimmed.find('(')?;
    if !trimmed.ends_with(')') {
        return None;
    }
    let name = trimmed[..open].trim();
    if !name
        .chars()
        .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
    {
        return None;
    }
    Some((name, &trimmed[open + 1..trimmed.len() - 1]))
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CallArgument {
    Positional(String),
    Keyword { name: String, value: String },
}

fn parse_call_arguments(args: &str) -> Option<Vec<CallArgument>> {
    if args.trim().is_empty() {
        return Some(Vec::new());
    }

    split_top_level(args, ',')
        .into_iter()
        .filter(|arg| !arg.trim().is_empty())
        .map(|arg| {
            let arg = arg.trim();
            if let Some((name, value)) = split_keyword_arg(arg) {
                Some(CallArgument::Keyword {
                    name: name.trim().to_string(),
                    value: python_string_literal(value.trim())?,
                })
            } else {
                Some(CallArgument::Positional(python_string_literal(arg)?))
            }
        })
        .collect()
}

fn keyword_argument(args: &[CallArgument], name: &str) -> Option<String> {
    args.iter().find_map(|arg| match arg {
        CallArgument::Keyword {
            name: arg_name,
            value,
        } if arg_name == name => Some(value.clone()),
        _ => None,
    })
}

fn split_keyword_arg(arg: &str) -> Option<(&str, &str)> {
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    let mut escaped = false;

    for (idx, ch) in arg.char_indices() {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == q {
                quote = None;
            }
            continue;
        }
        match ch {
            '\'' | '"' => quote = Some(ch),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            '=' if depth == 0 => return Some((&arg[..idx], &arg[idx + 1..])),
            _ => {}
        }
    }
    None
}

fn split_top_level(text: &str, separator: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    let mut escaped = false;

    for (idx, ch) in text.char_indices() {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == q {
                quote = None;
            }
            continue;
        }
        match ch {
            '\'' | '"' => quote = Some(ch),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ch if ch == separator && depth == 0 => {
                parts.push(&text[start..idx]);
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(&text[start..]);
    parts
}

fn python_string_literal(value: &str) -> Option<String> {
    let lower = value.to_ascii_lowercase();
    let prefixes = ["", "r", "u"];
    for prefix in prefixes {
        let Some(rest) = lower.strip_prefix(prefix) else {
            continue;
        };
        if rest.starts_with("\"\"\"") || rest.starts_with("'''") {
            continue;
        }
        if rest.starts_with('"') || rest.starts_with('\'') {
            let quote = value.as_bytes().get(prefix.len()).copied()?;
            let close = matching_quote_end(value, prefix.len(), quote)?;
            if value[close + 1..].trim().is_empty() {
                return Some(unescape_python_string(
                    &value[prefix.len() + 1..close],
                    quote,
                ));
            }
        }
    }
    None
}

fn matching_quote_end(value: &str, start: usize, quote: u8) -> Option<usize> {
    let bytes = value.as_bytes();
    let mut idx = start + 1;
    while idx < bytes.len() {
        match bytes[idx] {
            b'\\' => idx += 2,
            byte if byte == quote => return Some(idx),
            _ => idx += 1,
        }
    }
    None
}

fn unescape_python_string(value: &str, quote: u8) -> String {
    let mut output = String::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            output.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => output.push('\n'),
            Some('r') => output.push('\r'),
            Some('t') => output.push('\t'),
            Some('\\') => output.push('\\'),
            Some('\'') if quote == b'\'' => output.push('\''),
            Some('"') if quote == b'"' => output.push('"'),
            Some(other) => {
                output.push('\\');
                output.push(other);
            }
            None => output.push('\\'),
        }
    }
    output
}

fn contains_top_level_assignment(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut depth = 0i32;
    let mut quote: Option<u8> = None;
    let mut idx = 0;
    while idx < bytes.len() {
        let byte = bytes[idx];
        if let Some(q) = quote {
            if byte == b'\\' {
                idx += 2;
                continue;
            }
            if byte == q {
                quote = None;
            }
            idx += 1;
            continue;
        }
        match byte {
            b'\'' | b'"' => quote = Some(byte),
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b'=' if depth == 0 => {
                let prev = idx.checked_sub(1).and_then(|i| bytes.get(i)).copied();
                let next = bytes.get(idx + 1).copied();
                if prev != Some(b'=')
                    && prev != Some(b'!')
                    && prev != Some(b'<')
                    && prev != Some(b'>')
                    && next != Some(b'=')
                {
                    return true;
                }
            }
            _ => {}
        }
        idx += 1;
    }
    false
}

fn contains_top_level_compare(text: &str) -> bool {
    let mut depth = 0i32;
    let chars: Vec<char> = text.chars().collect();
    let mut idx = 0;
    while idx + 1 < chars.len() {
        match chars[idx] {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            '=' if depth == 0 && chars[idx + 1] == '=' => return true,
            _ => {}
        }
        idx += 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_markdown_and_math_blocks() {
        let parsed = parse_source(
            "test.py",
            r#"
"""
# Title
"""
from typing import Annotated
x: Annotated[float, Symbol("x")]
y = x + 1
# mathdocs: ignore
z = x + 2
"#,
        );
        assert_eq!(parsed.blocks.len(), 2);
        assert!(matches!(parsed.blocks[0].kind, BlockKind::Markdown { .. }));
        assert!(matches!(parsed.blocks[1].kind, BlockKind::Math { .. }));
    }

    #[test]
    fn scans_image_directives() {
        let parsed = parse_source(
            "test.py",
            r#"
from mathdocs import render_figure

"""
# Results
"""
render_figure("plots/loss.png", alt="Loss curve", caption="Training loss")
y = x + 1
"#,
        );
        assert_eq!(parsed.blocks.len(), 3);
        assert!(matches!(parsed.blocks[0].kind, BlockKind::Markdown { .. }));
        assert!(matches!(
            parsed.blocks[1].kind,
            BlockKind::Image {
                ref src,
                ref alt,
                ref caption,
                ..
            } if src == "plots/loss.png"
                && alt == "Loss curve"
                && caption.as_deref() == Some("Training loss")
        ));
        assert!(matches!(parsed.blocks[2].kind, BlockKind::Math { .. }));
    }

    #[test]
    fn scans_figure_directives_with_src_keyword() {
        let parsed = parse_source(
            "test.py",
            r#"
render_figure(
    src="plots/accuracy.png",
    title="Accuracy",
)
"#,
        );
        assert_eq!(parsed.blocks.len(), 1);
        assert!(matches!(
            parsed.blocks[0].kind,
            BlockKind::Image {
                ref src,
                ref title,
                ..
            } if src == "plots/accuracy.png" && title.as_deref() == Some("Accuracy")
        ));
    }

    #[test]
    fn scans_legacy_image_and_plot_directives() {
        let parsed = parse_source(
            "test.py",
            r#"
render_image("figures/confusion-matrix.png")
render_plot("figures/loss.png")
"#,
        );
        assert_eq!(parsed.blocks.len(), 2);
        assert!(matches!(parsed.blocks[0].kind, BlockKind::Image { .. }));
        assert!(matches!(parsed.blocks[1].kind, BlockKind::Image { .. }));
    }

    #[test]
    fn accepts_symbol_lookup_syntax_for_mathdocs_blocks() {
        let parsed = parse_source("test.py", "x = ::omega:: + 1\n");
        assert!(parsed.diagnostics.is_empty());
        assert_eq!(parsed.blocks.len(), 1);
    }

    #[test]
    fn applies_mathdocs_config_commands_to_blocks() {
        let parsed = parse_source(
            "test.py",
            r#"
# mathdocs: disable=auto_name/subscript, auto_name/symbol
final_loss = 1
alpha = 2 # mathdocs: disable=auto_name/symbol
beta = 3
"#,
        );
        assert_eq!(parsed.blocks.len(), 3);
        assert_eq!(parsed.blocks[0].config.auto_name_subscript, Some(false));
        assert_eq!(parsed.blocks[0].config.auto_name_symbol, Some(false));
        assert_eq!(parsed.blocks[1].config.auto_name_subscript, Some(false));
        assert_eq!(parsed.blocks[1].config.auto_name_symbol, Some(false));
        assert_eq!(parsed.blocks[2].config.auto_name_subscript, Some(false));
        assert_eq!(parsed.blocks[2].config.auto_name_symbol, Some(false));
    }
}
