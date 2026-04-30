use mathrender_ast::{
    parse_source, BlockKind, Diagnostic, DiagnosticSeverity, Range, RenderConfig, RenderableBlock,
    SourceFile, TextRange,
};
use mathrender_ir::lower_statement;
use mathrender_latex::{render_statement, RenderOptions};
use mathrender_metadata::{load_metadata_for_path, MetadataIndex};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderedBlock {
    pub kind: String,
    pub range: Range,
    pub markdown: String,
    pub latex: Option<String>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderedDocument {
    pub kind: String,
    pub range: Range,
    pub markdown: String,
    pub blocks: Vec<RenderedBlock>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
pub struct RenderEngine {
    pub options: RenderOptions,
}

impl Default for RenderEngine {
    fn default() -> Self {
        Self {
            options: RenderOptions::default(),
        }
    }
}

impl RenderEngine {
    pub fn render_path(&self, path: &Path) -> std::io::Result<RenderedDocument> {
        let source = std::fs::read_to_string(path)?;
        Ok(self.render_source(Some(path), &path.to_string_lossy(), &source))
    }

    pub fn render_source(
        &self,
        path: Option<&Path>,
        display_path: &str,
        source: &str,
    ) -> RenderedDocument {
        let parsed = parse_source(display_path, source);
        let metadata = load_metadata_for_path(path, source);
        let options = self.options_for_path(path);
        self.render_parsed(
            &parsed.source,
            &parsed.blocks,
            parsed.diagnostics,
            &metadata,
            &options,
            path,
            None,
        )
    }

    pub fn render_source_range(
        &self,
        path: Option<&Path>,
        display_path: &str,
        source: &str,
        range: TextRange,
    ) -> RenderedDocument {
        let parsed = parse_source(display_path, source);
        let metadata = load_metadata_for_path(path, source);
        let options = self.options_for_path(path);
        self.render_parsed(
            &parsed.source,
            &parsed.blocks,
            parsed.diagnostics,
            &metadata,
            &options,
            path,
            Some(range),
        )
    }

    pub fn list_blocks(
        &self,
        path: Option<&Path>,
        display_path: &str,
        source: &str,
    ) -> Vec<RenderedBlock> {
        self.render_source(path, display_path, source).blocks
    }

    fn render_parsed(
        &self,
        source: &SourceFile,
        blocks: &[RenderableBlock],
        mut diagnostics: Vec<Diagnostic>,
        metadata: &MetadataIndex,
        base_options: &RenderOptions,
        path: Option<&Path>,
        only_range: Option<TextRange>,
    ) -> RenderedDocument {
        let mut rendered_blocks = Vec::new();
        let mut markdown_parts = Vec::new();

        for block in blocks {
            if only_range.is_some_and(|range| !block.range.overlaps(range)) {
                continue;
            }
            let rendered = match &block.kind {
                BlockKind::Markdown { content } => RenderedBlock {
                    kind: "markdown".to_string(),
                    range: source.range(block.range),
                    markdown: content.clone(),
                    latex: None,
                    diagnostics: Vec::new(),
                },
                BlockKind::Math { statement } => {
                    let lowered = lower_statement(statement, block.range);
                    let options = apply_block_config(base_options, block.config);
                    let latex =
                        render_statement(&lowered.statement, metadata, &options, block.range);
                    let mut block_diagnostics = lowered.diagnostics;
                    block_diagnostics.extend(latex.diagnostics);
                    RenderedBlock {
                        kind: "math".to_string(),
                        range: source.range(block.range),
                        markdown: format!("$$\n{}\n$$", latex.latex),
                        latex: Some(latex.latex),
                        diagnostics: block_diagnostics,
                    }
                }
                BlockKind::Image {
                    src,
                    alt,
                    title,
                    caption,
                } => RenderedBlock {
                    kind: "image".to_string(),
                    range: source.range(block.range),
                    markdown: render_image_markdown(
                        &resolve_figure_src(src, path),
                        alt,
                        title.as_deref(),
                        caption.as_deref(),
                    ),
                    latex: None,
                    diagnostics: Vec::new(),
                },
            };
            diagnostics.extend(rendered.diagnostics.clone());
            markdown_parts.push(rendered.markdown.clone());
            rendered_blocks.push(rendered);
        }

        if rendered_blocks.is_empty() && diagnostics.is_empty() {
            diagnostics.push(Diagnostic {
                code: "mathrender::empty".to_string(),
                message: "no renderable blocks found".to_string(),
                range: TextRange { start: 0, end: 0 },
                severity: DiagnosticSeverity::Information,
            });
        }

        RenderedDocument {
            kind: "markdown".to_string(),
            range: source.range(TextRange {
                start: 0,
                end: source.source.len(),
            }),
            markdown: markdown_parts.join("\n\n"),
            blocks: rendered_blocks,
            diagnostics,
        }
    }

    fn options_for_path(&self, path: Option<&Path>) -> RenderOptions {
        let mut options = self.options.clone();
        if let Some(config_path) = path.and_then(find_mathrender_toml) {
            apply_render_options_for_path(&mut options, &config_path);
        }
        options
    }
}

fn apply_block_config(base: &RenderOptions, config: RenderConfig) -> RenderOptions {
    let mut options = base.clone();
    if let Some(enabled) = config.auto_name_subscript {
        options.underscore_subscripts = enabled;
    }
    if let Some(enabled) = config.auto_name_symbol {
        options.auto_name_symbol = enabled;
    }
    options
}

fn apply_render_options_for_path(options: &mut RenderOptions, config_path: &Path) {
    let Ok(text) = std::fs::read_to_string(config_path) else {
        return;
    };
    let Ok(value) = toml::from_str::<toml::Value>(&text) else {
        return;
    };
    apply_render_options_table(options, &value);
}

fn find_mathrender_toml(path: &Path) -> Option<PathBuf> {
    let mut dir = path.parent()?;
    loop {
        let candidate = dir.join("mathrender.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
        dir = dir.parent()?;
    }
}

fn apply_render_options_table(options: &mut RenderOptions, value: &toml::Value) {
    if let Some(auto_name) = value.get("auto_name") {
        if let Some(enabled) = auto_name.as_bool() {
            options.underscore_subscripts = enabled;
            options.auto_name_symbol = enabled;
        }
        if let Some(table) = auto_name.as_table() {
            if let Some(enabled) = table.get("subscript").and_then(|v| v.as_bool()) {
                options.underscore_subscripts = enabled;
            }
            if let Some(enabled) = table.get("symbol").and_then(|v| v.as_bool()) {
                options.auto_name_symbol = enabled;
            }
        }
    }

    if let Some(render) = value.get("render").and_then(|v| v.as_table()) {
        if let Some(enabled) = render.get("auto_name").and_then(|v| v.as_bool()) {
            options.underscore_subscripts = enabled;
            options.auto_name_symbol = enabled;
        }
        if let Some(enabled) = render.get("auto_name_subscript").and_then(|v| v.as_bool()) {
            options.underscore_subscripts = enabled;
        }
        if let Some(enabled) = render.get("auto_name_symbol").and_then(|v| v.as_bool()) {
            options.auto_name_symbol = enabled;
        }
    }
}

#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn parses_auto_name_render_options() {
        let value =
            toml::from_str::<toml::Value>("[auto_name]\nsubscript = false\nsymbol = false\n")
                .unwrap();
        let mut options = RenderOptions::default();
        apply_render_options_table(&mut options, &value);
        assert!(!options.underscore_subscripts);
        assert!(!options.auto_name_symbol);
    }

    #[test]
    fn finds_mathrender_toml_in_parent_dirs() {
        let root = std::env::temp_dir().join(format!(
            "mathrender-config-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let nested = root.join("nested");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(root.join("mathrender.toml"), "auto_name = false\n").unwrap();
        let found = find_mathrender_toml(&nested.join("example.py"));
        assert_eq!(found, Some(root.join("mathrender.toml")));
        std::fs::remove_dir_all(root).unwrap();
    }
}

fn resolve_figure_src(src: &str, source_path: Option<&Path>) -> String {
    if !is_local_relative_src(src) {
        return src.to_string();
    }

    let Some(source_path) = source_path else {
        return src.to_string();
    };
    let Some(source_dir) = source_path.parent() else {
        return src.to_string();
    };

    let target = absolutize(source_dir.join(src));
    std::env::current_dir()
        .ok()
        .and_then(|cwd| relative_path(&absolutize(cwd), &target))
        .unwrap_or(target)
        .to_string_lossy()
        .into_owned()
}

fn is_local_relative_src(src: &str) -> bool {
    let trimmed = src.trim();
    !trimmed.is_empty()
        && !trimmed.starts_with('#')
        && !trimmed.starts_with("data:")
        && !trimmed.contains("://")
        && !Path::new(trimmed).is_absolute()
}

fn absolutize(path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        return normalize_path(path);
    }
    let absolute = std::env::current_dir()
        .map(|cwd| cwd.join(&path))
        .unwrap_or(path);
    normalize_path(absolute)
}

fn normalize_path(path: PathBuf) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            component => normalized.push(component.as_os_str()),
        }
    }

    normalized
}

fn relative_path(from: &Path, to: &Path) -> Option<PathBuf> {
    let from_components = normalized_components(from)?;
    let to_components = normalized_components(to)?;

    if from_components.first() != to_components.first() {
        return None;
    }

    let mut shared = 0;
    while shared < from_components.len()
        && shared < to_components.len()
        && from_components[shared] == to_components[shared]
    {
        shared += 1;
    }

    let mut relative = PathBuf::new();
    for _ in shared..from_components.len() {
        relative.push("..");
    }
    for component in &to_components[shared..] {
        relative.push(component);
    }

    if relative.as_os_str().is_empty() {
        relative.push(".");
    }

    Some(relative)
}

fn normalized_components(path: &Path) -> Option<Vec<String>> {
    let normalized = normalize_path(path.to_path_buf());
    normalized
        .components()
        .map(|component| match component {
            std::path::Component::Prefix(prefix) => {
                Some(prefix.as_os_str().to_string_lossy().into_owned())
            }
            std::path::Component::RootDir => Some(std::path::MAIN_SEPARATOR.to_string()),
            std::path::Component::Normal(value) => Some(value.to_string_lossy().into_owned()),
            std::path::Component::CurDir | std::path::Component::ParentDir => None,
        })
        .collect()
}

fn render_image_markdown(
    src: &str,
    alt: &str,
    title: Option<&str>,
    caption: Option<&str>,
) -> String {
    let image = if let Some(title) = title {
        format!(
            "![{}]({} \"{}\")",
            escape_markdown_image_alt(alt),
            escape_markdown_url(src),
            escape_markdown_title(title)
        )
    } else {
        format!(
            "![{}]({})",
            escape_markdown_image_alt(alt),
            escape_markdown_url(src)
        )
    };

    if let Some(caption) = caption {
        format!("{image}\n\n_{caption}_")
    } else {
        image
    }
}

fn escape_markdown_image_alt(value: &str) -> String {
    value.replace('\\', "\\\\").replace(']', "\\]")
}

fn escape_markdown_url(value: &str) -> String {
    value.replace(' ', "%20").replace(')', "%29")
}

fn escape_markdown_title(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_linear_model_fixture() {
        let source = r#"
from typing import Annotated
from mathrender import Symbol, Tensor, render_as

"""
# Linear model

The prediction is a matrix-vector product plus a bias term.
"""

A: Annotated[np.ndarray, Tensor("A", ("i", "j"))]
x: Annotated[np.ndarray, Tensor("x", ("j",))]
b: Annotated[np.ndarray, Tensor("b", ("i",))]
y: Annotated[np.ndarray, Tensor("y", ("i",))]
sigma: Annotated[float, Symbol(r"\sigma")]

@render_as(latex=r"\left\|{0}\right\|")
def norm(v):
    return v

loss = norm(y - (A @ x + b)) / sigma
"#;
        let rendered = RenderEngine::default().render_source(None, "linear.py", source);
        assert!(rendered.markdown.contains("# Linear model"));
        assert!(rendered.markdown.contains(r"\left\|"));
        assert!(rendered.markdown.contains(r"\sigma"));
    }

    #[test]
    fn renders_image_directive_in_document_order() {
        let source = r#"
from mathrender import render_figure

"""
# Results
"""
render_figure("plots/loss.png", alt="Loss curve", caption="Training loss")
y = x + 1
"#;
        let rendered = RenderEngine::default().render_source(None, "images.py", source);
        assert!(rendered
            .markdown
            .contains("# Results\n\n![Loss curve](plots/loss.png)\n\n_Training loss_\n\n$$"));
        assert_eq!(rendered.blocks[1].kind, "image");
        assert_eq!(rendered.blocks[1].latex, None);
    }

    #[test]
    fn resolves_relative_figure_paths_from_source_file() {
        let source = r#"
from mathrender import render_figure

render_figure("plots/loss.png", alt="Loss curve")
"#;
        let path = std::env::current_dir()
            .unwrap()
            .join("docs")
            .join("report.py");
        let rendered =
            RenderEngine::default().render_source(Some(&path), &path.to_string_lossy(), source);
        assert!(rendered
            .markdown
            .contains("![Loss curve](docs/plots/loss.png)"));
    }

    #[test]
    fn leaves_remote_and_absolute_figure_paths_unchanged() {
        assert_eq!(
            resolve_figure_src("https://example.com/figure.png", None),
            "https://example.com/figure.png"
        );
        assert_eq!(
            resolve_figure_src("/var/tmp/figure.png", None),
            "/var/tmp/figure.png"
        );
    }
}
