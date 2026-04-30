use mathdocs_ast::TextRange;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolMeta {
    pub name: String,
    pub latex: String,
    pub text: Option<String>,
    pub tensor: Option<TensorMeta>,
    pub source_range: TextRange,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TensorMeta {
    pub latex: String,
    pub indices: Vec<IndexMeta>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexMeta {
    pub name: String,
    pub variance: Variance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Variance {
    Covariant,
    Contravariant,
    Unspecified,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionRenderMeta {
    pub qualified_name: String,
    pub latex_template: Option<String>,
    pub text_template: Option<String>,
    pub precedence: Option<u8>,
    pub kind: FunctionRenderKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FunctionRenderKind {
    Template,
    PrefixOperator,
    InfixOperator,
    SpecialForm,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetadataIndex {
    pub symbols: HashMap<String, SymbolMeta>,
    pub functions: HashMap<String, FunctionRenderMeta>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SymbolLookupEntry {
    pub path: &'static [&'static str],
    pub aliases: &'static [&'static str],
    pub insert_text: &'static str,
    pub display_symbol: &'static str,
    pub description: &'static str,
}

impl MetadataIndex {
    pub fn with_builtins() -> Self {
        let mut index = Self::default();
        for (name, template) in [
            ("abs", r"\left|{0}\right|"),
            ("sqrt", r"\sqrt{{{0}}}"),
            ("math.sqrt", r"\sqrt{{{0}}}"),
            ("np.sqrt", r"\sqrt{{{0}}}"),
            ("sin", r"\sin\left({0}\right)"),
            ("math.sin", r"\sin\left({0}\right)"),
            ("np.sin", r"\sin\left({0}\right)"),
            ("cos", r"\cos\left({0}\right)"),
            ("math.cos", r"\cos\left({0}\right)"),
            ("np.cos", r"\cos\left({0}\right)"),
            ("exp", r"e^{{{0}}}"),
            ("math.exp", r"e^{{{0}}}"),
            ("np.exp", r"e^{{{0}}}"),
            ("log", r"\log\left({0}\right)"),
            ("math.log", r"\log\left({0}\right)"),
            ("np.log", r"\log\left({0}\right)"),
            ("np.linalg.norm", r"\left\|{0}\right\|"),
        ] {
            index.functions.insert(
                name.to_string(),
                FunctionRenderMeta {
                    qualified_name: name.to_string(),
                    latex_template: Some(template.to_string()),
                    text_template: None,
                    precedence: None,
                    kind: FunctionRenderKind::Template,
                },
            );
        }
        index
    }

    pub fn merge_overwrite(&mut self, other: MetadataIndex) {
        self.symbols.extend(other.symbols);
        self.functions.extend(other.functions);
    }
}

pub fn symbol_lookup_entries() -> &'static [SymbolLookupEntry] {
    &[
        SymbolLookupEntry {
            path: &["greek", "alpha"],
            aliases: &["alpha"],
            insert_text: "α",
            display_symbol: "α",
            description: "Greek lowercase alpha",
        },
        SymbolLookupEntry {
            path: &["greek", "beta"],
            aliases: &["beta"],
            insert_text: "β",
            display_symbol: "β",
            description: "Greek lowercase beta",
        },
        SymbolLookupEntry {
            path: &["greek", "gamma"],
            aliases: &["gamma"],
            insert_text: "γ",
            display_symbol: "γ",
            description: "Greek lowercase gamma",
        },
        SymbolLookupEntry {
            path: &["greek", "delta"],
            aliases: &["delta"],
            insert_text: "δ",
            display_symbol: "δ",
            description: "Greek lowercase delta",
        },
        SymbolLookupEntry {
            path: &["greek", "epsilon"],
            aliases: &["epsilon"],
            insert_text: "ε",
            display_symbol: "ε",
            description: "Greek lowercase epsilon",
        },
        SymbolLookupEntry {
            path: &["greek", "theta"],
            aliases: &["theta"],
            insert_text: "θ",
            display_symbol: "θ",
            description: "Greek lowercase theta",
        },
        SymbolLookupEntry {
            path: &["greek", "lambda"],
            aliases: &["lambda"],
            insert_text: "λ",
            display_symbol: "λ",
            description: "Greek lowercase lambda",
        },
        SymbolLookupEntry {
            path: &["greek", "mu"],
            aliases: &["mu"],
            insert_text: "μ",
            display_symbol: "μ",
            description: "Greek lowercase mu",
        },
        SymbolLookupEntry {
            path: &["greek", "nu"],
            aliases: &["nu"],
            insert_text: "ν",
            display_symbol: "ν",
            description: "Greek lowercase nu",
        },
        SymbolLookupEntry {
            path: &["greek", "rho"],
            aliases: &["rho"],
            insert_text: "ρ",
            display_symbol: "ρ",
            description: "Greek lowercase rho",
        },
        SymbolLookupEntry {
            path: &["greek", "sigma"],
            aliases: &["sigma"],
            insert_text: "σ",
            display_symbol: "σ",
            description: "Greek lowercase sigma",
        },
        SymbolLookupEntry {
            path: &["greek", "tau"],
            aliases: &["tau"],
            insert_text: "τ",
            display_symbol: "τ",
            description: "Greek lowercase tau",
        },
        SymbolLookupEntry {
            path: &["greek", "phi"],
            aliases: &["phi"],
            insert_text: "φ",
            display_symbol: "φ",
            description: "Greek lowercase phi",
        },
        SymbolLookupEntry {
            path: &["greek", "psi"],
            aliases: &["psi"],
            insert_text: "ψ",
            display_symbol: "ψ",
            description: "Greek lowercase psi",
        },
        SymbolLookupEntry {
            path: &["greek", "omega"],
            aliases: &["omega"],
            insert_text: "ω",
            display_symbol: "ω",
            description: "Greek lowercase omega",
        },
    ]
}

pub fn resolve_symbol_lookup(query: &str) -> Option<&'static SymbolLookupEntry> {
    let query = query.trim();
    if query.is_empty() {
        return None;
    }

    if let Some(path) = query.strip_prefix('/') {
        let parts = path
            .split('/')
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();
        return symbol_lookup_entries()
            .iter()
            .find(|entry| entry.path == parts.as_slice());
    }

    symbol_lookup_entries()
        .iter()
        .find(|entry| entry.aliases.contains(&query))
}

pub fn expand_symbol_lookups(source: &str) -> String {
    let mut expanded = String::with_capacity(source.len());
    let mut rest = source;

    while let Some(start) = rest.find("::") {
        expanded.push_str(&rest[..start]);
        let after_open = &rest[start + 2..];
        let Some(end) = after_open.find("::") else {
            expanded.push_str(&rest[start..]);
            return expanded;
        };
        let query = &after_open[..end];
        if let Some(entry) = resolve_symbol_lookup(query) {
            expanded.push_str(entry.insert_text);
        } else {
            expanded.push_str("::");
            expanded.push_str(query);
            expanded.push_str("::");
        }
        rest = &after_open[end + 2..];
    }

    expanded.push_str(rest);
    expanded
}

pub fn load_metadata_for_path(path: Option<&Path>, source: &str) -> MetadataIndex {
    let mut merged = MetadataIndex::with_builtins();

    if let Some(path) = path {
        if let Some(sidecar) = sidecar_path(path) {
            if let Ok(text) = fs::read_to_string(sidecar) {
                merged.merge_overwrite(extract_sidecar_metadata(&text));
            }
        }
        let stub = path.with_extension("pyi");
        if let Ok(text) = fs::read_to_string(stub) {
            merged.merge_overwrite(extract_inline_metadata(&text));
        }
    }

    merged.merge_overwrite(extract_inline_metadata(source));
    merged
}

pub fn extract_inline_metadata(source: &str) -> MetadataIndex {
    let mut index = MetadataIndex::default();
    let mut decorators: Vec<String> = Vec::new();
    let mut offset = 0usize;

    for line in source.lines() {
        let trimmed = line.trim();
        let range = TextRange {
            start: offset,
            end: offset + line.len(),
        };

        if trimmed.starts_with('@') {
            decorators.push(trimmed.to_string());
            offset += line.len() + 1;
            continue;
        }

        if let Some(name) = def_name(trimmed) {
            for decorator in decorators.drain(..) {
                if let Some(meta) = parse_render_as_decorator(name, &decorator) {
                    index.functions.insert(name.to_string(), meta);
                }
            }
        } else if !trimmed.is_empty() && !trimmed.starts_with('#') {
            decorators.clear();
        }

        if let Some((name, meta)) = parse_symbol_or_tensor_line(trimmed, range) {
            index.symbols.insert(name, meta);
        }

        offset += line.len() + 1;
    }

    index
}

pub fn extract_sidecar_metadata(source: &str) -> MetadataIndex {
    let mut index = MetadataIndex::default();
    let Ok(value) = toml::from_str::<toml::Value>(source) else {
        return index;
    };

    if let Some(symbols) = value.get("symbols").and_then(|v| v.as_table()) {
        for (name, value) in symbols {
            if let Some(latex) = value.as_str() {
                index.symbols.insert(
                    name.to_string(),
                    SymbolMeta {
                        name: name.to_string(),
                        latex: latex.to_string(),
                        text: None,
                        tensor: None,
                        source_range: TextRange { start: 0, end: 0 },
                    },
                );
            }
        }
    }

    if let Some(functions) = value.get("functions").and_then(|v| v.as_table()) {
        for (name, value) in functions {
            if let Some(table) = value.as_table() {
                let latex = table
                    .get("latex")
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string);
                let text = table
                    .get("text")
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string);
                if latex.is_some() || text.is_some() {
                    index.functions.insert(
                        name.to_string(),
                        FunctionRenderMeta {
                            qualified_name: name.to_string(),
                            latex_template: latex,
                            text_template: text,
                            precedence: None,
                            kind: FunctionRenderKind::Template,
                        },
                    );
                }
            }
        }
    }

    index
}

fn sidecar_path(path: &Path) -> Option<PathBuf> {
    let stem = path.file_stem()?.to_string_lossy();
    Some(path.with_file_name(format!("{stem}.mathdocs.toml")))
}

fn def_name(trimmed: &str) -> Option<&str> {
    let rest = trimmed.strip_prefix("def ")?;
    let end = rest.find('(')?;
    Some(rest[..end].trim())
}

fn parse_render_as_decorator(name: &str, decorator: &str) -> Option<FunctionRenderMeta> {
    if !decorator.contains("render_as") {
        return None;
    }
    let args = constructor_args(decorator, "render_as")?;
    let latex = parse_kw_string(args, "latex");
    let text = parse_kw_string(args, "text");
    let precedence = parse_kw_int(args, "precedence").map(|v| v as u8);
    Some(FunctionRenderMeta {
        qualified_name: name.to_string(),
        latex_template: latex,
        text_template: text,
        precedence,
        kind: FunctionRenderKind::Template,
    })
}

fn parse_symbol_or_tensor_line(
    trimmed: &str,
    source_range: TextRange,
) -> Option<(String, SymbolMeta)> {
    let colon = trimmed.find(':')?;
    let name = trimmed[..colon].trim();
    if name.is_empty() || !trimmed[colon..].contains("Annotated") {
        return None;
    }

    if let Some(args) = constructor_args(trimmed, "Tensor") {
        let strings = parse_all_strings(args);
        let latex = strings.first()?.clone();
        let indices = strings
            .iter()
            .skip(1)
            .map(|name| IndexMeta {
                name: name.clone(),
                variance: Variance::Covariant,
            })
            .collect::<Vec<_>>();
        return Some((
            name.to_string(),
            SymbolMeta {
                name: name.to_string(),
                latex: latex.clone(),
                text: None,
                tensor: Some(TensorMeta { latex, indices }),
                source_range,
            },
        ));
    }

    if let Some(args) = constructor_args(trimmed, "Symbol") {
        let latex = parse_first_string(args)?;
        return Some((
            name.to_string(),
            SymbolMeta {
                name: name.to_string(),
                latex,
                text: parse_kw_string(args, "text"),
                tensor: None,
                source_range,
            },
        ));
    }

    None
}

fn constructor_args<'a>(text: &'a str, ctor: &str) -> Option<&'a str> {
    let start = text.find(&format!("{ctor}("))? + ctor.len() + 1;
    let bytes = text.as_bytes();
    let mut depth = 1i32;
    let mut quote: Option<u8> = None;
    let mut idx = start;
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
            b')' | b']' | b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&text[start..idx]);
                }
            }
            _ => {}
        }
        idx += 1;
    }
    None
}

fn parse_kw_string(args: &str, key: &str) -> Option<String> {
    let marker = format!("{key}=");
    let start = args.find(&marker)? + marker.len();
    parse_string_at(&args[start..]).map(|(value, _)| value)
}

fn parse_kw_int(args: &str, key: &str) -> Option<u64> {
    let marker = format!("{key}=");
    let start = args.find(&marker)? + marker.len();
    let digits = args[start..]
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    digits.parse().ok()
}

fn parse_first_string(args: &str) -> Option<String> {
    parse_all_strings(args).into_iter().next()
}

fn parse_all_strings(args: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut idx = 0usize;
    while idx < args.len() {
        if let Some((value, consumed)) = parse_string_at(&args[idx..]) {
            out.push(value);
            idx += consumed;
        } else {
            idx += args[idx..].chars().next().map(char::len_utf8).unwrap_or(1);
        }
    }
    out
}

fn parse_string_at(text: &str) -> Option<(String, usize)> {
    let bytes = text.as_bytes();
    let mut idx = 0usize;
    let mut raw = false;
    while idx < bytes.len() {
        match bytes[idx] {
            b'r' | b'R' => {
                raw = true;
                idx += 1;
            }
            b'u' | b'U' | b'b' | b'B' | b'f' | b'F' => idx += 1,
            b'\'' | b'"' => break,
            b' ' | b'\t' => idx += 1,
            _ => return None,
        }
    }
    let quote = *bytes.get(idx)?;
    if quote != b'\'' && quote != b'"' {
        return None;
    }
    idx += 1;
    let mut value = String::new();
    while idx < bytes.len() {
        let byte = bytes[idx];
        if byte == quote {
            return Some((value, idx + 1));
        }
        if byte == b'\\' && !raw {
            if let Some(next) = bytes.get(idx + 1).copied() {
                match next {
                    b'n' => value.push('\n'),
                    b't' => value.push('\t'),
                    b'\\' => value.push('\\'),
                    b'\'' => value.push('\''),
                    b'"' => value.push('"'),
                    other => value.push(other as char),
                }
                idx += 2;
                continue;
            }
        }
        value.push(byte as char);
        idx += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_symbols_tensors_and_functions() {
        let source = r#"
theta: Annotated[float, Symbol(r"\theta")]
A: Annotated[np.ndarray, Tensor("A", ("i", "j"))]
@render_as(latex=r"\left\|{0}\right\|")
def norm(x): ...
"#;
        let meta = extract_inline_metadata(source);
        assert_eq!(meta.symbols["theta"].latex, r"\theta");
        assert_eq!(meta.symbols["A"].tensor.as_ref().unwrap().indices.len(), 2);
        assert_eq!(
            meta.functions["norm"].latex_template.as_deref(),
            Some(r"\left\|{0}\right\|")
        );
    }

    #[test]
    fn resolves_direct_and_namespaced_symbol_lookups() {
        assert_eq!(resolve_symbol_lookup("omega").unwrap().insert_text, "ω");
        assert_eq!(
            resolve_symbol_lookup("/greek/omega").unwrap().insert_text,
            "ω"
        );
        assert_eq!(expand_symbol_lookups("x = ::omega::"), "x = ω");
    }
}
