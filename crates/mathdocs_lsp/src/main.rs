use mathdocs_ast::{
    Diagnostic as RenderDiagnostic, DiagnosticSeverity as RenderSeverity,
    Position as RenderPosition, Range as RenderRange, TextRange,
};
use mathdocs_markdown::{RenderEngine, RenderedBlock, RenderedDocument};
use mathdocs_metadata::symbol_lookup_entries;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result as RpcResult;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug)]
struct Backend {
    client: Client,
    documents: Arc<Mutex<HashMap<Url, String>>>,
    engine: RenderEngine,
}

#[derive(Debug, Clone, Deserialize)]
struct RenderDocumentParams {
    #[serde(rename = "textDocument")]
    text_document: TextDocumentIdentifier,
}

#[derive(Debug, Clone, Deserialize)]
struct RenderRangeParams {
    #[serde(rename = "textDocument")]
    text_document: TextDocumentIdentifier,
    range: Option<Range>,
}

#[derive(Debug, Clone, Serialize)]
struct ListBlocksResponse {
    blocks: Vec<RenderedBlock>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> RpcResult<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                code_lens_provider: Some(CodeLensOptions {
                    resolve_provider: Some(false),
                }),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![":".to_string(), "/".to_string()]),
                    ..CompletionOptions::default()
                }),
                ..ServerCapabilities::default()
            },
            server_info: Some(ServerInfo {
                name: "mathdocs".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "MathDocs language server initialized")
            .await;
    }

    async fn shutdown(&self) -> RpcResult<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.documents.lock().await.insert(uri.clone(), text);
        self.publish_diagnostics(uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().last() {
            self.documents
                .lock()
                .await
                .insert(params.text_document.uri.clone(), change.text);
            self.publish_diagnostics(params.text_document.uri).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents
            .lock()
            .await
            .remove(&params.text_document.uri);
        self.client
            .publish_diagnostics(params.text_document.uri, Vec::new(), None)
            .await;
    }

    async fn hover(&self, params: HoverParams) -> RpcResult<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let Some(rendered) = self.render_document_for_uri(&uri).await else {
            return Ok(None);
        };
        let block = rendered
            .blocks
            .into_iter()
            .find(|block| contains_position(block.range, pos) && block.kind == "math");
        Ok(block.map(|block| Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: block.markdown,
            }),
            range: Some(to_lsp_range(block.range)),
        }))
    }

    async fn code_lens(&self, params: CodeLensParams) -> RpcResult<Option<Vec<CodeLens>>> {
        let uri = params.text_document.uri;
        let Some((text, _path)) = self.document_text_and_path(&uri).await else {
            return Ok(Some(Vec::new()));
        };
        let Some(rendered) = self.render_document_for_uri(&uri).await else {
            return Ok(Some(Vec::new()));
        };
        let mut lenses = Vec::new();

        if let Some(range) = mathdocs_import_range(&text) {
            lenses.push(CodeLens {
                range,
                command: Some(Command {
                    title: "Preview MathDocs file".to_string(),
                    command: "mathdocs.previewDocument".to_string(),
                    arguments: Some(vec![serde_json::json!({
                        "uri": uri,
                    })]),
                }),
                data: None,
            });
        }

        lenses.extend(
            rendered
                .blocks
                .into_iter()
                .filter(|block| block.kind == "math")
                .map(|block| CodeLens {
                    range: to_lsp_range(block.range),
                    command: Some(Command {
                        title: "Preview MathDocs".to_string(),
                        command: "mathdocs.previewRange".to_string(),
                        arguments: Some(vec![serde_json::json!({
                            "uri": uri,
                            "range": to_lsp_range(block.range),
                        })]),
                    }),
                    data: None,
                }),
        );
        Ok(Some(lenses))
    }

    async fn completion(&self, params: CompletionParams) -> RpcResult<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        let Some((text, _path)) = self.document_text_and_path(&uri).await else {
            return Ok(None);
        };
        let source = mathdocs_ast::SourceFile::new(uri.to_string(), text);
        let offset = source.offset_at(RenderPosition {
            line: pos.line,
            character: pos.character,
        });
        let Some((replace_range, query)) = symbol_lookup_context(&source.source, offset) else {
            return Ok(None);
        };

        let items = symbol_lookup_entries()
            .iter()
            .filter(|entry| symbol_lookup_matches(entry, &query))
            .map(|entry| {
                let namespaced = format!("/{}", entry.path.join("/"));
                let alias = entry.aliases.first().copied().unwrap_or_default();
                let label = if query.starts_with('/') {
                    format!("::{namespaced}::")
                } else {
                    format!("::{alias}::")
                };
                CompletionItem {
                    label,
                    kind: Some(CompletionItemKind::CONSTANT),
                    detail: Some(format!("{} {}", entry.display_symbol, entry.description)),
                    filter_text: Some(format!("{alias} {namespaced} {}", entry.display_symbol)),
                    text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                        range: to_lsp_range(source.range(replace_range)),
                        new_text: entry.insert_text.to_string(),
                    })),
                    ..CompletionItem::default()
                }
            })
            .collect::<Vec<_>>();

        Ok(Some(CompletionResponse::Array(items)))
    }
}

impl Backend {
    async fn render_document(&self, params: RenderDocumentParams) -> RpcResult<RenderedDocument> {
        Ok(self
            .render_document_for_uri(&params.text_document.uri)
            .await
            .unwrap_or_else(empty_document))
    }

    async fn render_range(&self, params: RenderRangeParams) -> RpcResult<RenderedDocument> {
        let uri = params.text_document.uri;
        let Some((text, path)) = self.document_text_and_path(&uri).await else {
            return Ok(empty_document());
        };
        let display_path = uri.to_string();
        let rendered = if let Some(range) = params.range {
            let source = mathdocs_ast::SourceFile::new(display_path.clone(), text.clone());
            self.engine.render_source_range(
                path.as_deref(),
                &display_path,
                &text,
                TextRange {
                    start: source.offset_at(RenderPosition {
                        line: range.start.line,
                        character: range.start.character,
                    }),
                    end: source.offset_at(RenderPosition {
                        line: range.end.line,
                        character: range.end.character,
                    }),
                },
            )
        } else {
            self.engine
                .render_source(path.as_deref(), &display_path, &text)
        };
        Ok(rendered)
    }

    async fn render_hover(
        &self,
        params: TextDocumentPositionParams,
    ) -> RpcResult<Option<RenderedBlock>> {
        let Some(rendered) = self
            .render_document_for_uri(&params.text_document.uri)
            .await
        else {
            return Ok(None);
        };
        Ok(rendered
            .blocks
            .into_iter()
            .find(|block| contains_position(block.range, params.position)))
    }

    async fn list_blocks(&self, params: RenderDocumentParams) -> RpcResult<ListBlocksResponse> {
        let rendered = self
            .render_document_for_uri(&params.text_document.uri)
            .await
            .unwrap_or_else(empty_document);
        Ok(ListBlocksResponse {
            blocks: rendered.blocks,
        })
    }

    async fn render_document_for_uri(&self, uri: &Url) -> Option<RenderedDocument> {
        let (text, path) = self.document_text_and_path(uri).await?;
        let display_path = uri.to_string();
        Some(
            self.engine
                .render_source(path.as_deref(), &display_path, &text),
        )
    }

    async fn document_text_and_path(&self, uri: &Url) -> Option<(String, Option<PathBuf>)> {
        if let Some(text) = self.documents.lock().await.get(uri).cloned() {
            return Some((text, uri.to_file_path().ok()));
        }
        let path = uri.to_file_path().ok()?;
        let text = std::fs::read_to_string(&path).ok()?;
        Some((text, Some(path)))
    }

    async fn publish_diagnostics(&self, uri: Url) {
        let Some((text, path)) = self.document_text_and_path(&uri).await else {
            return;
        };
        let display_path = uri.to_string();
        let source_file = mathdocs_ast::SourceFile::new(display_path.clone(), text.clone());
        let rendered = self
            .engine
            .render_source(path.as_deref(), &display_path, &text);
        let diagnostics = rendered
            .diagnostics
            .iter()
            .map(|diagnostic| render_diagnostic_to_lsp(diagnostic, &source_file))
            .collect::<Vec<_>>();
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }
}

fn empty_document() -> RenderedDocument {
    RenderedDocument {
        kind: "markdown".to_string(),
        range: RenderRange {
            start: RenderPosition {
                line: 0,
                character: 0,
            },
            end: RenderPosition {
                line: 0,
                character: 0,
            },
        },
        markdown: String::new(),
        blocks: Vec::new(),
        diagnostics: Vec::new(),
    }
}

fn render_diagnostic_to_lsp(
    diagnostic: &RenderDiagnostic,
    source: &mathdocs_ast::SourceFile,
) -> Diagnostic {
    Diagnostic {
        range: to_lsp_range(source.range(diagnostic.range)),
        severity: Some(match diagnostic.severity {
            RenderSeverity::Error => DiagnosticSeverity::ERROR,
            RenderSeverity::Warning => DiagnosticSeverity::WARNING,
            RenderSeverity::Information => DiagnosticSeverity::INFORMATION,
        }),
        code: Some(NumberOrString::String(diagnostic.code.clone())),
        source: Some("mathdocs".to_string()),
        message: diagnostic.message.clone(),
        ..Diagnostic::default()
    }
}

fn to_lsp_range(range: RenderRange) -> Range {
    Range {
        start: Position {
            line: range.start.line,
            character: range.start.character,
        },
        end: Position {
            line: range.end.line,
            character: range.end.character,
        },
    }
}

fn contains_position(range: RenderRange, position: Position) -> bool {
    let start = Position {
        line: range.start.line,
        character: range.start.character,
    };
    let end = Position {
        line: range.end.line,
        character: range.end.character,
    };
    position >= start && position <= end
}

fn symbol_lookup_context(source: &str, offset: usize) -> Option<(TextRange, String)> {
    let prefix = source.get(..offset)?;
    let start = prefix.rfind("::")?;
    let query_start = start + 2;
    let query = &source[query_start..offset];
    if query.contains("::") || query.contains('\n') || query.contains('\r') {
        return None;
    }
    Some((TextRange { start, end: offset }, query.to_string()))
}

fn symbol_lookup_matches(entry: &mathdocs_metadata::SymbolLookupEntry, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    if let Some(path_query) = query.strip_prefix('/') {
        let path = entry.path.join("/");
        return path.starts_with(path_query);
    }
    entry
        .aliases
        .iter()
        .any(|alias| alias.starts_with(query) || alias.contains(query))
}

fn mathdocs_import_range(text: &str) -> Option<Range> {
    text.lines().enumerate().find_map(|(line_index, line)| {
        let trimmed = line.trim_start();
        if trimmed == "import mathdocs"
            || trimmed.starts_with("import mathdocs as ")
            || trimmed.starts_with("from mathdocs import ")
        {
            Some(Range {
                start: Position {
                    line: line_index as u32,
                    character: (line.len() - trimmed.len()) as u32,
                },
                end: Position {
                    line: line_index as u32,
                    character: line.len() as u32,
                },
            })
        } else {
            None
        }
    })
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let documents = Arc::new(Mutex::new(HashMap::new()));
    let (service, socket) = LspService::build(|client| Backend {
        client,
        documents,
        engine: RenderEngine::default(),
    })
    .custom_method("mathRender/renderDocument", Backend::render_document)
    .custom_method("mathRender/renderRange", Backend::render_range)
    .custom_method("mathRender/renderHover", Backend::render_hover)
    .custom_method("mathRender/listBlocks", Backend::list_blocks)
    .finish();
    Server::new(stdin, stdout, socket).serve(service).await;
}
