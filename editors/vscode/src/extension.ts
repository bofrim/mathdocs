import * as fs from 'fs';
import * as path from 'path';
import * as vscode from 'vscode';
import MarkdownIt from 'markdown-it';
import katex from 'katex';
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind
} from 'vscode-languageclient/node';

let client: LanguageClient | undefined;
let inlineRenderer: InlineMathOverlayController | undefined;
const inlineMathSvgHeight = 20;
const inlineMathOverlayOffset = 24;
let applyingSymbolLookupEdit = false;

const symbolLookupEntries = [
  { path: ['greek', 'alpha'], aliases: ['alpha'], insertText: 'α', displaySymbol: 'α', description: 'Greek lowercase alpha' },
  { path: ['greek', 'beta'], aliases: ['beta'], insertText: 'β', displaySymbol: 'β', description: 'Greek lowercase beta' },
  { path: ['greek', 'gamma'], aliases: ['gamma'], insertText: 'γ', displaySymbol: 'γ', description: 'Greek lowercase gamma' },
  { path: ['greek', 'delta'], aliases: ['delta'], insertText: 'δ', displaySymbol: 'δ', description: 'Greek lowercase delta' },
  { path: ['greek', 'epsilon'], aliases: ['epsilon'], insertText: 'ε', displaySymbol: 'ε', description: 'Greek lowercase epsilon' },
  { path: ['greek', 'theta'], aliases: ['theta'], insertText: 'θ', displaySymbol: 'θ', description: 'Greek lowercase theta' },
  { path: ['greek', 'lambda'], aliases: ['lambda'], insertText: 'λ', displaySymbol: 'λ', description: 'Greek lowercase lambda' },
  { path: ['greek', 'mu'], aliases: ['mu'], insertText: 'μ', displaySymbol: 'μ', description: 'Greek lowercase mu' },
  { path: ['greek', 'nu'], aliases: ['nu'], insertText: 'ν', displaySymbol: 'ν', description: 'Greek lowercase nu' },
  { path: ['greek', 'rho'], aliases: ['rho'], insertText: 'ρ', displaySymbol: 'ρ', description: 'Greek lowercase rho' },
  { path: ['greek', 'sigma'], aliases: ['sigma'], insertText: 'σ', displaySymbol: 'σ', description: 'Greek lowercase sigma' },
  { path: ['greek', 'tau'], aliases: ['tau'], insertText: 'τ', displaySymbol: 'τ', description: 'Greek lowercase tau' },
  { path: ['greek', 'phi'], aliases: ['phi'], insertText: 'φ', displaySymbol: 'φ', description: 'Greek lowercase phi' },
  { path: ['greek', 'psi'], aliases: ['psi'], insertText: 'ψ', displaySymbol: 'ψ', description: 'Greek lowercase psi' },
  { path: ['greek', 'omega'], aliases: ['omega'], insertText: 'ω', displaySymbol: 'ω', description: 'Greek lowercase omega' }
];

interface RenderedDocument {
  markdown: string;
}

interface RenderedBlock {
  kind: string;
  markdown: string;
  latex?: string | null;
  range: vscode.Range | SerializedRange;
}

interface ListBlocksResponse {
  blocks: RenderedBlock[];
}

interface SerializedRange {
  start: { line: number; character: number };
  end: { line: number; character: number };
}

interface PreviewRangeArg {
  uri: string;
  range: vscode.Range;
}

interface PreviewDocumentArg {
  uri: string;
}

interface MarkdownStringBlock {
  startLine: number;
  endLine: number;
}

interface MarkdownStringFoldArg {
  uri: string;
}

interface InlineMathDocsMetrics {
  fontSize: number;
  scale: number;
  widthPerCharacter: number;
  translateY: number;
}

interface HoverMathDocsMetrics {
  width: number;
  height: number;
  fontSize: number;
}

export async function activate(context: vscode.ExtensionContext) {
  client = new LanguageClient(
    'mathdocs',
    'MathDocs',
    serverOptions(context),
    clientOptions()
  );
  context.subscriptions.push(client);
  await client.start();

  context.subscriptions.push(
    vscode.commands.registerCommand('mathdocs.previewDocument', previewDocument),
    vscode.commands.registerCommand('mathdocs.previewCurrentFile', previewCurrentFile),
    vscode.commands.registerCommand('mathdocs.previewSelection', previewSelection),
    vscode.commands.registerCommand('mathdocs.previewRange', previewRange),
    vscode.commands.registerCommand('mathdocs.toggleInlineRender', toggleInlineRender),
    vscode.commands.registerCommand('mathdocs.inlineRenderSpacer', () => undefined),
    vscode.commands.registerCommand('mathdocs.collapseMarkdownString', collapseMarkdownString),
    vscode.commands.registerCommand('mathdocs.expandMarkdownString', expandMarkdownString),
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (!event.affectsConfiguration('mathdocs.inlineRender.enabled')) {
        return;
      }
      refreshCodeLenses();
      if (isInlineRenderEnabled()) {
        inlineRenderer?.refreshAllVisibleEditors();
      } else {
        inlineRenderer?.clearAllEditors();
      }
    }),
    vscode.workspace.onDidChangeTextDocument((event) => {
      refreshCodeLenses();
      triggerSymbolLookupSuggest(event).then(undefined, () => undefined);
      convertCompletedSymbolLookup(event).then(undefined, () => undefined);
    }),
    vscode.languages.registerCodeLensProvider(
      { scheme: 'file', language: 'python' },
      new InlineMathSpacerCodeLensProvider()
    ),
    vscode.languages.registerCodeLensProvider(
      { scheme: 'file', language: 'python' },
      new MarkdownStringCodeLensProvider()
    ),
    vscode.languages.registerHoverProvider(
      { scheme: 'file', language: 'python' },
      new MathDocsHoverProvider()
    ),
    vscode.languages.registerCompletionItemProvider(
      { scheme: 'file', language: 'python' },
      new SymbolLookupCompletionProvider(),
      ':',
      '/'
    ),
    vscode.languages.registerFoldingRangeProvider(
      { scheme: 'file', language: 'python' },
      new MarkdownStringFoldingRangeProvider()
    )
  );

  inlineRenderer = new InlineMathOverlayController(context);
  context.subscriptions.push(inlineRenderer);
  inlineRenderer.refreshAllVisibleEditors();
}

export async function deactivate(): Promise<void> {
  await client?.stop();
}

function serverOptions(context: vscode.ExtensionContext): ServerOptions {
  const exeName = process.platform === 'win32' ? 'mathdocs-lsp.exe' : 'mathdocs-lsp';
  const configured = vscode.workspace.getConfiguration('mathdocs').get<string>('serverPath');
  const workspaceBinary = vscode.workspace.workspaceFolders?.[0]
    ? path.join(vscode.workspace.workspaceFolders[0].uri.fsPath, 'target', 'debug', exeName)
    : '';
  const bundledBinary = path.join(context.extensionPath, 'bin', exeName);
  const command = configured
    || (workspaceBinary && fs.existsSync(workspaceBinary) ? workspaceBinary
    : (fs.existsSync(bundledBinary) ? bundledBinary
    : 'mathdocs-lsp'));
  return {
    run: { command, transport: TransportKind.stdio },
    debug: { command, transport: TransportKind.stdio }
  };
}

function clientOptions(): LanguageClientOptions {
  return {
    documentSelector: [{ scheme: 'file', language: 'python' }],
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher('**/*.{py,pyi,mathdocs.toml}')
    },
    middleware: {
      provideCodeLenses: async (document, token, next) => {
        const lenses = await next(document, token);
        if (!isInlineRenderEnabled() || !lenses) {
          return lenses;
        }

        return lenses.filter((lens) => !isRangePreviewCodeLens(lens));
      }
    }
  };
}

async function previewCurrentFile() {
  const editor = vscode.window.activeTextEditor;
  if (!editor || !client) {
    return;
  }
  const result = await client.sendRequest<RenderedDocument>('mathRender/renderDocument', {
    textDocument: { uri: editor.document.uri.toString() }
  });
  showPreview(editor.document.fileName, result.markdown);
}

async function previewDocument(arg?: PreviewDocumentArg) {
  if (!arg || !client) {
    return previewCurrentFile();
  }
  const uri = vscode.Uri.parse(arg.uri);
  const result = await client.sendRequest<RenderedDocument>('mathRender/renderDocument', {
    textDocument: { uri: uri.toString() }
  });
  showPreview(uri.fsPath || uri.toString(), result.markdown);
}

async function previewSelection() {
  const editor = vscode.window.activeTextEditor;
  if (!editor || !client) {
    return;
  }
  const result = await client.sendRequest<RenderedDocument>('mathRender/renderRange', {
    textDocument: { uri: editor.document.uri.toString() },
    range: editor.selection
  });
  showPreview(`${editor.document.fileName} selection`, result.markdown);
}

async function previewRange(arg?: PreviewRangeArg) {
  if (!arg || !client) {
    return previewSelection();
  }
  const result = await client.sendRequest<RenderedDocument>('mathRender/renderRange', {
    textDocument: { uri: arg.uri },
    range: arg.range
  });
  showPreview('MathDocs preview', result.markdown);
}

async function toggleInlineRender() {
  const configuration = vscode.workspace.getConfiguration('mathdocs.inlineRender');
  const enabled = configuration.get<boolean>('enabled', false);
  await configuration.update('enabled', !enabled, vscode.ConfigurationTarget.Global);
}

function isInlineRenderEnabled(): boolean {
  return vscode.workspace.getConfiguration('mathdocs.inlineRender').get<boolean>('enabled', false);
}

function isRangePreviewCodeLens(lens: vscode.CodeLens): boolean {
  return lens.command?.command === 'mathdocs.previewRange';
}

async function convertCompletedSymbolLookup(event: vscode.TextDocumentChangeEvent) {
  if (applyingSymbolLookupEdit || event.document.languageId !== 'python') {
    return;
  }
  if (event.contentChanges.length !== 1 || event.contentChanges[0].text !== '::') {
    return;
  }

  const editor = vscode.window.visibleTextEditors.find((candidate) => candidate.document.uri.toString() === event.document.uri.toString());
  if (!editor) {
    return;
  }

  const change = event.contentChanges[0];
  const end = change.range.end.translate(0, 2);
  const linePrefix = event.document.lineAt(end.line).text.slice(0, end.character);
  const startCharacter = linePrefix.lastIndexOf('::', Math.max(0, linePrefix.length - 3));
  if (startCharacter < 0) {
    return;
  }

  const query = linePrefix.slice(startCharacter + 2, linePrefix.length - 2);
  const replacement = resolveSymbolLookup(query);
  if (!replacement) {
    return;
  }

  applyingSymbolLookupEdit = true;
  try {
    await editor.edit((edit) => {
      edit.replace(new vscode.Range(end.line, startCharacter, end.line, end.character), replacement);
    });
  } finally {
    applyingSymbolLookupEdit = false;
  }
}

async function triggerSymbolLookupSuggest(event: vscode.TextDocumentChangeEvent) {
  if (applyingSymbolLookupEdit || event.document.languageId !== 'python') {
    return;
  }
  if (event.contentChanges.length !== 1) {
    return;
  }

  const change = event.contentChanges[0];
  if (!/^(::|[:/A-Za-z])$/.test(change.text)) {
    return;
  }

  const editor = vscode.window.visibleTextEditors.find((candidate) => candidate.document.uri.toString() === event.document.uri.toString());
  if (!editor || vscode.window.activeTextEditor !== editor) {
    return;
  }

  const position = change.range.end.translate(0, change.text.length);
  if (!symbolLookupContext(event.document, position)) {
    return;
  }

  await vscode.commands.executeCommand('editor.action.triggerSuggest');
}

function resolveSymbolLookup(query: string): string | undefined {
  if (!query) {
    return undefined;
  }
  if (query.startsWith('/')) {
    const parts = query.split('/').filter(Boolean);
    return symbolLookupEntries.find((entry) => entry.path.join('/') === parts.join('/'))?.insertText;
  }
  return symbolLookupEntries.find((entry) => entry.aliases.includes(query))?.insertText;
}

interface SymbolLookupContext {
  query: string;
  range: vscode.Range;
}

class SymbolLookupCompletionProvider implements vscode.CompletionItemProvider {
  provideCompletionItems(document: vscode.TextDocument, position: vscode.Position): vscode.CompletionItem[] | undefined {
    const context = symbolLookupContext(document, position);
    if (!context) {
      return undefined;
    }

    return symbolLookupEntries
      .filter((entry) => symbolLookupMatches(entry, context.query))
      .map((entry) => {
        const alias = entry.aliases[0] ?? '';
        const namespaced = `/${entry.path.join('/')}`;
        const lookup = context.query.startsWith('/') ? `::${namespaced}::` : `::${alias}::`;
        const label = `${entry.displaySymbol} ${context.query.startsWith('/') ? namespaced : alias}`;
        const item = new vscode.CompletionItem(label, vscode.CompletionItemKind.Constant);
        item.detail = `${lookup} ${entry.description}`;
        item.filterText = `${lookup} ${alias} ${namespaced} ${entry.displaySymbol}`;
        item.insertText = entry.insertText;
        item.range = context.range;
        item.sortText = `0_${entry.path.join('_')}`;
        return item;
      });
  }
}

function symbolLookupContext(document: vscode.TextDocument, position: vscode.Position): SymbolLookupContext | undefined {
  const linePrefix = document.lineAt(position.line).text.slice(0, position.character);
  const startCharacter = linePrefix.lastIndexOf('::');
  if (startCharacter < 0) {
    return undefined;
  }

  const query = linePrefix.slice(startCharacter + 2);
  if (query.includes('::') || /\s/.test(query)) {
    return undefined;
  }

  return {
    query,
    range: new vscode.Range(position.line, startCharacter, position.line, position.character)
  };
}

function symbolLookupMatches(entry: typeof symbolLookupEntries[number], query: string): boolean {
  if (!query) {
    return true;
  }
  if (query.startsWith('/')) {
    return entry.path.join('/').startsWith(query.slice(1));
  }
  return entry.aliases.some((alias) => alias.startsWith(query) || alias.includes(query));
}

async function collapseMarkdownString(arg?: MarkdownStringFoldArg) {
  await foldMarkdownStrings(arg, 'editor.fold');
}

async function expandMarkdownString(arg?: MarkdownStringFoldArg) {
  await foldMarkdownStrings(arg, 'editor.unfold');
}

async function foldMarkdownStrings(arg: MarkdownStringFoldArg | undefined, command: 'editor.fold' | 'editor.unfold') {
  const editor = await editorForMarkdownStringCommand(arg);
  if (!editor) {
    return;
  }

  const lines = findMarkdownStringBlocks(editor.document).map((block) => block.startLine);
  if (lines.length === 0) {
    return;
  }

  const originalSelection = editor.selection;
  editor.selection = new vscode.Selection(lines[0], 0, lines[0], 0);
  await vscode.commands.executeCommand(command, { selectionLines: lines });
  editor.selection = originalSelection;
}

async function editorForMarkdownStringCommand(arg?: MarkdownStringFoldArg): Promise<vscode.TextEditor | undefined> {
  if (!arg) {
    return vscode.window.activeTextEditor;
  }

  const uri = vscode.Uri.parse(arg.uri);
  const visibleEditor = vscode.window.visibleTextEditors.find((editor) => editor.document.uri.toString() === uri.toString());
  if (visibleEditor) {
    await vscode.window.showTextDocument(visibleEditor.document, visibleEditor.viewColumn);
    return visibleEditor;
  }

  const document = await vscode.workspace.openTextDocument(uri);
  return vscode.window.showTextDocument(document, { preview: false });
}

function refreshCodeLenses() {
  vscode.commands.executeCommand('editor.action.codeLens.refresh').then(undefined, () => undefined);
}

function toVsCodeRange(range: vscode.Range | SerializedRange): vscode.Range {
  if (range instanceof vscode.Range) {
    return range;
  }

  return new vscode.Range(
    new vscode.Position(range.start.line, range.start.character),
    new vscode.Position(range.end.line, range.end.character)
  );
}

class InlineMathSpacerCodeLensProvider implements vscode.CodeLensProvider {
  async provideCodeLenses(document: vscode.TextDocument): Promise<vscode.CodeLens[]> {
    if (!client || !isInlineRenderEnabled()) {
      return [];
    }

    let response: ListBlocksResponse;
    try {
      response = await client.sendRequest<ListBlocksResponse>('mathRender/listBlocks', {
        textDocument: { uri: document.uri.toString() }
      });
    } catch {
      return [];
    }

    return response.blocks
      .filter((block) => block.kind === 'math')
      .map((block) => {
        const range = toVsCodeRange(block.range);
        return new vscode.CodeLens(new vscode.Range(range.start.line, 0, range.start.line, 0), {
          title: ' ',
          command: 'mathdocs.inlineRenderSpacer'
        });
      });
  }
}

class InlineMathOverlayController implements vscode.Disposable {
  private readonly subscriptions: vscode.Disposable[] = [];
  private readonly editorDecorations = new Map<string, vscode.TextEditorDecorationType[]>();
  private generation = 0;

  constructor(context: vscode.ExtensionContext) {
    this.subscriptions.push(
      vscode.window.onDidChangeVisibleTextEditors(() => this.refreshAllVisibleEditors()),
      vscode.workspace.onDidChangeTextDocument((event) => this.refreshDocument(event.document)),
      vscode.window.onDidChangeActiveColorTheme(() => this.refreshAllVisibleEditors()),
      vscode.workspace.onDidChangeConfiguration((event) => {
        if (!event.affectsConfiguration('mathdocs.inlineRender.enabled')) {
          return;
        }

        if (isInlineRenderEnabled()) {
          this.refreshAllVisibleEditors();
        } else {
          this.clearAllEditors();
        }
      })
    );

    context.subscriptions.push(...this.subscriptions);
  }

  refreshAllVisibleEditors() {
    this.generation += 1;
    const generation = this.generation;
    for (const editor of vscode.window.visibleTextEditors) {
      this.refreshEditor(editor, generation);
    }
  }

  clearAllEditors() {
    for (const editor of vscode.window.visibleTextEditors) {
      this.clearEditor(editor);
    }
  }

  private refreshDocument(document: vscode.TextDocument) {
    const editor = vscode.window.visibleTextEditors.find((candidate) => candidate.document.uri.toString() === document.uri.toString());
    if (!editor) {
      return;
    }
    this.refreshEditor(editor, this.generation);
  }

  private async refreshEditor(editor: vscode.TextEditor, generation: number) {
    if (!client || !isInlineRenderEnabled() || editor.document.languageId !== 'python' || editor.document.uri.scheme !== 'file') {
      this.clearEditor(editor);
      return;
    }

    let response: ListBlocksResponse;
    try {
      response = await client.sendRequest<ListBlocksResponse>('mathRender/listBlocks', {
        textDocument: { uri: editor.document.uri.toString() }
      });
    } catch {
      this.clearEditor(editor);
      return;
    }

    if (generation !== this.generation) {
      return;
    }

    this.clearEditor(editor);
    const decorations = response.blocks
      .filter((block) => block.kind === 'math')
      .map((block) => this.renderBlockDecoration(block))
      .filter((decoration): decoration is { type: vscode.TextEditorDecorationType; range: vscode.Range } => decoration !== undefined);

    for (const decoration of decorations) {
      editor.setDecorations(decoration.type, [decoration.range]);
    }
    this.editorDecorations.set(editor.document.uri.toString(), decorations.map((decoration) => decoration.type));
  }

  private renderBlockDecoration(block: RenderedBlock): { type: vscode.TextEditorDecorationType; range: vscode.Range } | undefined {
    const latex = block.latex ?? mathBlockLatex(block.markdown);
    if (!latex) {
      return undefined;
    }

    const range = toVsCodeRange(block.range);
    const decorationType = vscode.window.createTextEditorDecorationType({
      before: {
        contentIconPath: vscode.Uri.parse(inlineMathSvgDataUri(latex)),
        height: `${inlineMathSvgHeight}px`,
        width: 'auto',
        margin: '0',
        textDecoration: `none; position: absolute; transform: translateY(-${inlineMathOverlayOffset}px); pointer-events: none; z-index: 10;`
      }
    });

    return {
      type: decorationType,
      range: new vscode.Range(range.start.line, 0, range.start.line, 0)
    };
  }

  private clearEditor(editor: vscode.TextEditor) {
    const key = editor.document.uri.toString();
    const decorations = this.editorDecorations.get(key) ?? [];
    for (const decoration of decorations) {
      decoration.dispose();
    }
    this.editorDecorations.delete(key);
  }

  dispose() {
    this.clearAllEditors();
    for (const subscription of this.subscriptions) {
      subscription.dispose();
    }
  }
}

class MarkdownStringCodeLensProvider implements vscode.CodeLensProvider {
  provideCodeLenses(document: vscode.TextDocument): vscode.CodeLens[] {
    return findMarkdownStringBlocks(document).flatMap((block) => {
      const range = new vscode.Range(block.startLine, 0, block.startLine, 0);
      const arg: MarkdownStringFoldArg = {
        uri: document.uri.toString()
      };
      return [
        new vscode.CodeLens(range, {
          title: 'Collapse all markdown',
          command: 'mathdocs.collapseMarkdownString',
          arguments: [arg]
        }),
        new vscode.CodeLens(range, {
          title: 'Expand all markdown',
          command: 'mathdocs.expandMarkdownString',
          arguments: [arg]
        })
      ];
    });
  }
}

class MarkdownStringFoldingRangeProvider implements vscode.FoldingRangeProvider {
  provideFoldingRanges(document: vscode.TextDocument): vscode.FoldingRange[] {
    return findMarkdownStringBlocks(document).map(
      (block) => new vscode.FoldingRange(block.startLine, block.endLine)
    );
  }
}

class MathDocsHoverProvider implements vscode.HoverProvider {
  async provideHover(
    document: vscode.TextDocument,
    position: vscode.Position
  ): Promise<vscode.Hover | undefined> {
    if (!client) {
      return undefined;
    }

    const block = await client.sendRequest<RenderedBlock | null>('mathRender/renderHover', {
      textDocument: { uri: document.uri.toString() },
      position
    });
    if (!block || block.kind !== 'math') {
      return undefined;
    }

    const latex = mathBlockLatex(block.markdown);
    if (!latex) {
      return undefined;
    }

    const markdown = new vscode.MarkdownString(undefined, true);
    markdown.supportHtml = true;
    markdown.isTrusted = false;
    markdown.appendMarkdown(mathHoverImageMarkdown(latex));
    return new vscode.Hover(markdown, toVsCodeRange(block.range));
  }
}

function findMarkdownStringBlocks(document: vscode.TextDocument): MarkdownStringBlock[] {
  const text = document.getText();
  const blocks: MarkdownStringBlock[] = [];
  let offset = 0;

  while (offset < text.length) {
    const start = findStandaloneTripleQuoteStart(text, offset);
    if (!start) {
      break;
    }

    const closeOffset = text.indexOf(start.quote, start.contentOffset);
    if (closeOffset === -1) {
      break;
    }

    const startPosition = document.positionAt(start.tokenOffset);
    const closePosition = document.positionAt(closeOffset);
    const afterCloseOffset = closeOffset + start.quote.length;

    if (
      startPosition.line < closePosition.line &&
      isStandaloneOpeningLine(document, startPosition.line, start.tokenOffset, start.contentOffset) &&
      isStandaloneClosingLine(document, closePosition.line, closeOffset, afterCloseOffset)
    ) {
      blocks.push({ startLine: startPosition.line, endLine: closePosition.line });
    }

    offset = afterCloseOffset;
  }

  return blocks;
}

function findStandaloneTripleQuoteStart(text: string, fromOffset: number): {
  tokenOffset: number;
  contentOffset: number;
  quote: '"""' | "'''";
} | undefined {
  for (let offset = fromOffset; offset < text.length; offset += 1) {
    const quoteStart = tripleQuoteStartAt(text, offset);
    if (quoteStart) {
      return quoteStart;
    }
  }
  return undefined;
}

function tripleQuoteStartAt(text: string, offset: number): {
  tokenOffset: number;
  contentOffset: number;
  quote: '"""' | "'''";
} | undefined {
  if (text.startsWith('"""', offset) || text.startsWith("'''", offset)) {
    const quote = text.slice(offset, offset + 3) as '"""' | "'''";
    return { tokenOffset: offset, contentOffset: offset + 3, quote };
  }

  if (offset > 0 && isIdentifierCharacter(text[offset - 1])) {
    return undefined;
  }

  let cursor = offset;
  while (cursor < text.length && isStringPrefixCharacter(text[cursor])) {
    cursor += 1;
  }

  if (cursor === offset || cursor - offset > 3 || (!text.startsWith('"""', cursor) && !text.startsWith("'''", cursor))) {
    return undefined;
  }

  const quote = text.slice(cursor, cursor + 3) as '"""' | "'''";
  return { tokenOffset: offset, contentOffset: cursor + 3, quote };
}

function isStandaloneOpeningLine(
  document: vscode.TextDocument,
  lineNumber: number,
  tokenOffset: number,
  contentOffset: number
): boolean {
  const line = document.lineAt(lineNumber);
  const tokenColumn = tokenOffset - document.offsetAt(line.range.start);
  const afterQuoteColumn = contentOffset - document.offsetAt(line.range.start);
  return tokenColumn === 0 && line.text.slice(afterQuoteColumn).trim().length === 0;
}

function isStandaloneClosingLine(
  document: vscode.TextDocument,
  lineNumber: number,
  closeOffset: number,
  afterCloseOffset: number
): boolean {
  const line = document.lineAt(lineNumber);
  const closeColumn = closeOffset - document.offsetAt(line.range.start);
  const afterQuoteColumn = afterCloseOffset - document.offsetAt(line.range.start);
  return line.text.slice(0, closeColumn).trim().length === 0 && line.text.slice(afterQuoteColumn).trim().length === 0;
}

function isStringPrefixCharacter(value: string): boolean {
  return value === 'r' || value === 'R' || value === 'u' || value === 'U' || value === 'b' || value === 'B' || value === 'f' || value === 'F';
}

function isIdentifierCharacter(value: string): boolean {
  return /[A-Za-z0-9_]/.test(value);
}

function mathBlockLatex(markdown: string): string | undefined {
  const match = markdown.match(/^\s*\$\$\s*\n?([\s\S]*?)\n?\s*\$\$\s*$/);
  return match?.[1]?.trim();
}

function mathHoverImageMarkdown(latex: string): string {
  try {
    return `![${escapeMarkdownAlt(latex)}](${mathHoverSvgDataUri(latex)})`;
  } catch {
    return `\`\`\`tex\n${latex}\n\`\`\``;
  }
}

function mathHoverSvgDataUri(latex: string): string {
  const metrics = hoverMathDocsMetrics(latex);
  const html = katex.renderToString(latex, {
    displayMode: true,
    throwOnError: false,
    output: 'html'
  });
  const fg = vscode.window.activeColorTheme.kind === vscode.ColorThemeKind.Light ? '#1f2328' : '#d4d4d4';
  const css = hoverKatexCss();
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${metrics.width}" height="${metrics.height}">
  <foreignObject x="0" y="0" width="${metrics.width}" height="${metrics.height}">
    <div xmlns="http://www.w3.org/1999/xhtml">
      <style>${escapeHtml(css)}
        .mathdocs-hover-viewport { width:${metrics.width}px; min-height:${metrics.height}px; overflow:visible; display:flex; align-items:center; }
        .mathdocs-hover-math { color:${fg}; font-size:${metrics.fontSize}px; line-height:1.25; white-space:nowrap; padding:12px 0; }
        .mathdocs-hover-math .katex-display { margin: 0; text-align: left; }
      </style>
      <div class="mathdocs-hover-viewport"><div class="mathdocs-hover-math">${html}</div></div>
    </div>
  </foreignObject>
</svg>`;
  return `data:image/svg+xml;base64,${Buffer.from(svg).toString('base64')}`;
}

function hoverMathDocsMetrics(latex: string): HoverMathDocsMetrics {
  const width = Math.min(980, Math.max(360, latex.length * 11));
  if (/\\begin\{(?:cases|matrix|pmatrix|bmatrix|aligned|array)\}/.test(latex)) {
    return { width, height: 220, fontSize: 19 };
  }
  if (hasTallInlineMath(latex)) {
    return { width, height: 132, fontSize: 19 };
  }
  return { width, height: 76, fontSize: 20 };
}

function inlineMathSvgDataUri(latex: string): string {
  const metrics = inlineMathDocsMetrics(latex);
  const width = Math.min(900, Math.max(180, latex.length * metrics.widthPerCharacter));
  const height = inlineMathSvgHeight;
  const html = katex.renderToString(latex, {
    displayMode: false,
    throwOnError: false,
    output: 'html'
  });
  const fg = vscode.window.activeColorTheme.kind === vscode.ColorThemeKind.Light ? '#6e7681' : '#8b8795';
  const css = hoverKatexCss();
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}">
  <foreignObject x="0" y="0" width="${width}" height="${height}">
    <div xmlns="http://www.w3.org/1999/xhtml">
      <style>${escapeHtml(css)}
        .mathdocs-inline-viewport { width:${width}px; height:${height}px; overflow:hidden; display:flex; align-items:center; }
        .mathdocs-inline-math { color:${fg}; font-size:${metrics.fontSize}px; line-height:1; white-space:nowrap; transform:translateY(${metrics.translateY}px) scale(${metrics.scale}); transform-origin:left center; }
        .mathdocs-inline-math .katex { font-size:1em; }
      </style>
      <div class="mathdocs-inline-viewport"><div class="mathdocs-inline-math">${html}</div></div>
    </div>
  </foreignObject>
</svg>`;
  return `data:image/svg+xml;base64,${Buffer.from(svg).toString('base64')}`;
}

function inlineMathDocsMetrics(latex: string): InlineMathDocsMetrics {
  if (hasTallInlineMath(latex)) {
    return {
      fontSize: 14,
      scale: 0.68,
      widthPerCharacter: 6.25,
      translateY: -1
    };
  }

  return {
    fontSize: 15,
    scale: 0.86,
    widthPerCharacter: 7,
    translateY: 0
  };
}

function hasTallInlineMath(latex: string): boolean {
  return /\\(?:dfrac|tfrac|frac|sqrt|sum|prod|int|iint|iiint|oint|lim)\b|\\begin\{(?:cases|matrix|pmatrix|bmatrix|aligned|array)\}/.test(latex);
}

function hoverKatexCss(): string {
  try {
    return fs.readFileSync(path.join(__dirname, 'katex', 'katex.min.css'), 'utf8');
  } catch {
    return '';
  }
}

function escapeMarkdownAlt(value: string): string {
  return value.replace(/[\]\n\r]/g, ' ');
}

function showPreview(title: string, markdown: string) {
  const katexDist = path.join(__dirname, 'katex');
  const panel = vscode.window.createWebviewPanel(
    'mathdocsPreview',
    `MathDocs: ${path.basename(title)}`,
    vscode.ViewColumn.Beside,
    {
      enableScripts: false,
      localResourceRoots: [vscode.Uri.file(katexDist)]
    }
  );
  panel.webview.html = renderMarkdown(markdown, panel.webview);
}

function renderMarkdown(markdown: string, webview: vscode.Webview): string {
  const md = new MarkdownIt({ html: false, linkify: true });
  const katexCss = loadKatexCss(webview);
  const mathBlocks: string[] = [];
  const withPlaceholders = markdown.replace(/\$\$\s*\n?([\s\S]*?)\n?\s*\$\$/g, (_match, latex) => {
    const placeholder = `@@MATHDOCS_KATEX_BLOCK_${mathBlocks.length}@@`;
    try {
      mathBlocks.push(katex.renderToString(latex.trim(), { displayMode: true, throwOnError: false }));
    } catch {
      mathBlocks.push(`<pre>${escapeHtml(latex)}</pre>`);
    }
    return placeholder;
  });
  let body = md.render(withPlaceholders);
  mathBlocks.forEach((html, index) => {
    const placeholder = `@@MATHDOCS_KATEX_BLOCK_${index}@@`;
    body = body
      .replace(new RegExp(`<p>\\s*${placeholder}\\s*</p>`, 'g'), html)
      .replaceAll(placeholder, html);
  });

  return `<!doctype html>
<html>
<head>
  <meta charset="utf-8">
  <style>
    ${katexCss}
    body { font-family: var(--vscode-font-family); color: var(--vscode-foreground); padding: 24px; line-height: 1.5; }
    .katex-display { overflow-x: auto; overflow-y: hidden; padding: 8px 0; }
    code, pre { font-family: var(--vscode-editor-font-family); }
  </style>
</head>
<body>${body}</body>
</html>`;
}

function loadKatexCss(webview: vscode.Webview): string {
  try {
    const cssPath = path.join(__dirname, 'katex', 'katex.min.css');
    const cssDir = path.dirname(cssPath);
    return fs
      .readFileSync(cssPath, 'utf8')
      .replace(/url\((fonts\/[^)]+)\)/g, (_match, fontPath) => {
        const fontUri = webview.asWebviewUri(vscode.Uri.file(path.join(cssDir, fontPath)));
        return `url(${fontUri})`;
      });
  } catch {
    return '';
  }
}

function escapeHtml(value: string): string {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
}
