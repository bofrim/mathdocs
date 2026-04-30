use mathdocs_ast::{Diagnostic, DiagnosticSeverity, TextRange};
use mathdocs_ir::{qualified_name, BinaryOp, CompareOp, Expr, Statement, UnaryOp};
use mathdocs_metadata::{MetadataIndex, SymbolMeta};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderOptions {
    pub underscore_subscripts: bool,
    pub auto_name_symbol: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            underscore_subscripts: true,
            auto_name_symbol: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderedLatex {
    pub latex: String,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
struct RenderedExpr {
    latex: String,
    precedence: u8,
    indices: Option<Vec<String>>,
    tensor_notation: bool,
    diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Default)]
struct RenderContext {
    expected_indices: Option<Vec<String>>,
    force_tensor: bool,
    suppress_tensor_indices: bool,
}

pub fn render_statement(
    statement: &Statement,
    metadata: &MetadataIndex,
    options: &RenderOptions,
    range: TextRange,
) -> RenderedLatex {
    match statement {
        Statement::Assignment { target, value } => {
            let rhs = render_expr(value, metadata, options, &RenderContext::default(), range);
            if is_named_equation(target, value) {
                return RenderedLatex {
                    latex: rhs.latex,
                    diagnostics: rhs.diagnostics,
                };
            }
            let target_context = RenderContext {
                expected_indices: rhs.indices.clone(),
                force_tensor: rhs.indices.is_some(),
                suppress_tensor_indices: false,
            };
            let lhs = render_expr(target, metadata, options, &target_context, range);
            let mut diagnostics = rhs.diagnostics;
            diagnostics.extend(lhs.diagnostics);
            RenderedLatex {
                latex: format!("{} = {}", lhs.latex, rhs.latex),
                diagnostics,
            }
        }
        Statement::Expr(expr) => {
            let expr = render_expr(expr, metadata, options, &RenderContext::default(), range);
            RenderedLatex {
                latex: expr.latex,
                diagnostics: expr.diagnostics,
            }
        }
    }
}

fn is_named_equation(target: &Expr, value: &Expr) -> bool {
    matches!(target, Expr::Name(name) if name.starts_with("eq_"))
        && matches!(value, Expr::Compare { .. })
}

fn render_expr(
    expr: &Expr,
    metadata: &MetadataIndex,
    options: &RenderOptions,
    context: &RenderContext,
    range: TextRange,
) -> RenderedExpr {
    match expr {
        Expr::Name(name) => render_name(name, metadata, options, context),
        Expr::Literal(value) => RenderedExpr {
            latex: escape_latex_literal(value),
            precedence: 100,
            indices: None,
            tensor_notation: false,
            diagnostics: Vec::new(),
        },
        Expr::StringLiteral(value) => RenderedExpr {
            latex: format!(r"\text{{{}}}", escape_text(value)),
            precedence: 100,
            indices: None,
            tensor_notation: false,
            diagnostics: Vec::new(),
        },
        Expr::Unary { op, expr } => {
            let inner = render_expr(expr, metadata, options, context, range);
            let symbol = match op {
                UnaryOp::Plus => "+",
                UnaryOp::Minus => "-",
            };
            RenderedExpr {
                latex: format!("{symbol}{}", maybe_group(&inner, 7)),
                precedence: 7,
                indices: inner.indices,
                tensor_notation: inner.tensor_notation,
                diagnostics: inner.diagnostics,
            }
        }
        Expr::Binary { op, left, right } => {
            render_binary(*op, left, right, metadata, options, range)
        }
        Expr::Compare { op, left, right } => {
            let left = render_expr(left, metadata, options, context, range);
            let right = render_expr(right, metadata, options, context, range);
            let symbol = match op {
                CompareOp::Eq => "=",
            };
            let mut diagnostics = left.diagnostics;
            diagnostics.extend(right.diagnostics);
            RenderedExpr {
                latex: format!("{} {symbol} {}", left.latex, right.latex),
                precedence: 1,
                indices: None,
                tensor_notation: false,
                diagnostics,
            }
        }
        Expr::Call { func, args } => render_call(func, args, metadata, options, range),
        Expr::Attribute { value, attr } => {
            let rendered = render_expr(value, metadata, options, context, range);
            RenderedExpr {
                latex: format!(r"\operatorname{{{}}}", escape_text(attr)),
                precedence: 100,
                indices: rendered.indices,
                tensor_notation: rendered.tensor_notation,
                diagnostics: rendered.diagnostics,
            }
        }
        Expr::Subscript { value, indices } => {
            render_subscript(value, indices, metadata, options, range)
        }
        Expr::Group(inner) => {
            let inner = render_expr(inner, metadata, options, context, range);
            RenderedExpr {
                latex: format!(r"\left({}\right)", inner.latex),
                precedence: 100,
                indices: inner.indices,
                tensor_notation: inner.tensor_notation,
                diagnostics: inner.diagnostics,
            }
        }
        Expr::Unsupported(text) => RenderedExpr {
            latex: format!(r"\text{{{}}}", escape_text(text)),
            precedence: 100,
            indices: None,
            tensor_notation: false,
            diagnostics: vec![Diagnostic {
                code: "mathdocs::unsupported".to_string(),
                message: format!("unsupported expression: {text}"),
                range,
                severity: DiagnosticSeverity::Warning,
            }],
        },
    }
}

fn render_name(
    name: &str,
    metadata: &MetadataIndex,
    options: &RenderOptions,
    context: &RenderContext,
) -> RenderedExpr {
    let symbol = metadata.symbols.get(name);
    let indices = symbol.and_then(|meta| meta.tensor.as_ref()).map(|tensor| {
        tensor
            .indices
            .iter()
            .map(|idx| idx.name.clone())
            .collect::<Vec<_>>()
    });

    let mut tensor_notation = false;
    let latex = if let Some(meta) = symbol {
        if meta.tensor.is_some()
            && !context.suppress_tensor_indices
            && (context.force_tensor || context.expected_indices.is_some())
        {
            tensor_notation = true;
            tensor_latex(
                &meta.latex,
                context
                    .expected_indices
                    .as_ref()
                    .or(indices.as_ref())
                    .map(Vec::as_slice)
                    .unwrap_or(&[]),
                metadata,
            )
        } else {
            meta.latex.clone()
        }
    } else if context.expected_indices.is_some() && !context.suppress_tensor_indices {
        tensor_notation = true;
        let (base_name, prime) = strip_prime_suffix(name);
        let base = format!("{}{prime}", identifier_base_latex(base_name, options));
        tensor_latex(
            &base,
            context.expected_indices.as_deref().unwrap_or(&[]),
            metadata,
        )
    } else {
        identifier_latex(name, options)
    };

    RenderedExpr {
        latex,
        precedence: 100,
        indices,
        tensor_notation,
        diagnostics: Vec::new(),
    }
}

fn render_binary(
    op: BinaryOp,
    left: &Expr,
    right: &Expr,
    metadata: &MetadataIndex,
    options: &RenderOptions,
    range: TextRange,
) -> RenderedExpr {
    match op {
        BinaryOp::Div => {
            let left = render_expr(left, metadata, options, &RenderContext::default(), range);
            let right = render_expr(right, metadata, options, &RenderContext::default(), range);
            let mut diagnostics = left.diagnostics;
            diagnostics.extend(right.diagnostics);
            RenderedExpr {
                latex: format!(r"\frac{{{}}}{{{}}}", left.latex, right.latex),
                precedence: 100,
                indices: None,
                tensor_notation: false,
                diagnostics,
            }
        }
        BinaryOp::Pow => {
            let left = render_expr(left, metadata, options, &RenderContext::default(), range);
            let right = render_expr(right, metadata, options, &RenderContext::default(), range);
            let latex = format!("{}^{{{}}}", maybe_group(&left, 8), right.latex);
            let mut diagnostics = left.diagnostics;
            diagnostics.extend(right.diagnostics);
            RenderedExpr {
                latex,
                precedence: 8,
                indices: left.indices,
                tensor_notation: left.tensor_notation,
                diagnostics,
            }
        }
        BinaryOp::MatMul => render_matmul(left, right, metadata, options, range),
        BinaryOp::Add | BinaryOp::Sub => {
            let initial_left =
                render_expr(left, metadata, options, &RenderContext::default(), range);
            let initial_right =
                render_expr(right, metadata, options, &RenderContext::default(), range);
            let expected = if initial_left.tensor_notation {
                initial_left.indices.clone()
            } else if initial_right.tensor_notation {
                initial_right.indices.clone()
            } else {
                None
            };
            let child_context = RenderContext {
                expected_indices: expected.clone(),
                force_tensor: false,
                suppress_tensor_indices: false,
            };
            let left = render_expr(left, metadata, options, &child_context, range);
            let right = render_expr(right, metadata, options, &child_context, range);
            let symbol = if op == BinaryOp::Add { "+" } else { "-" };
            let latex = format!(
                "{} {symbol} {}",
                maybe_group(&left, 3),
                maybe_group(&right, 4)
            );
            let mut diagnostics = initial_left.diagnostics;
            diagnostics.extend(initial_right.diagnostics);
            diagnostics.extend(left.diagnostics);
            diagnostics.extend(right.diagnostics);
            RenderedExpr {
                latex,
                precedence: 3,
                indices: expected,
                tensor_notation: child_context.expected_indices.is_some(),
                diagnostics,
            }
        }
        BinaryOp::Mul => {
            let left = render_expr(left, metadata, options, &RenderContext::default(), range);
            let right = render_expr(right, metadata, options, &RenderContext::default(), range);
            let latex = format!("{}{}", maybe_group(&left, 5), maybe_group(&right, 6));
            let mut diagnostics = left.diagnostics;
            diagnostics.extend(right.diagnostics);
            RenderedExpr {
                latex,
                precedence: 5,
                indices: None,
                tensor_notation: false,
                diagnostics,
            }
        }
    }
}

fn render_matmul(
    left: &Expr,
    right: &Expr,
    metadata: &MetadataIndex,
    options: &RenderOptions,
    range: TextRange,
) -> RenderedExpr {
    let operands = flatten_matmul(left)
        .into_iter()
        .chain(flatten_matmul(right))
        .collect::<Vec<_>>();
    let force = RenderContext {
        expected_indices: None,
        force_tensor: true,
        suppress_tensor_indices: false,
    };

    let mut latex = String::new();
    let mut diagnostics = Vec::new();
    let mut free_indices: Option<Vec<String>> = None;

    for operand in operands {
        let rendered = render_expr(operand, metadata, options, &force, range);
        let operand_indices = rendered.indices.clone().unwrap_or_default();
        if latex.is_empty() {
            latex.push_str(&rendered.latex);
            free_indices = Some(operand_indices);
        } else {
            latex.push_str(&rendered.latex);
            let current = free_indices.get_or_insert_with(Vec::new);
            if let (Some(left_last), Some(right_first)) =
                (current.last().cloned(), operand_indices.first().cloned())
            {
                if left_last == right_first {
                    current.pop();
                    current.extend(operand_indices.into_iter().skip(1));
                } else {
                    diagnostics.push(Diagnostic {
                        code: "mathdocs::tensor-index".to_string(),
                        message: format!(
                            "cannot infer contraction for indices ending in {left_last} and starting with {right_first}"
                        ),
                        range,
                        severity: DiagnosticSeverity::Warning,
                    });
                    current.extend(operand_indices);
                }
            } else {
                current.extend(operand_indices);
            }
        }
        diagnostics.extend(rendered.diagnostics);
    }

    RenderedExpr {
        latex,
        precedence: 5,
        indices: free_indices,
        tensor_notation: true,
        diagnostics,
    }
}

fn flatten_matmul(expr: &Expr) -> Vec<&Expr> {
    match expr {
        Expr::Binary {
            op: BinaryOp::MatMul,
            left,
            right,
        } => flatten_matmul(left)
            .into_iter()
            .chain(flatten_matmul(right))
            .collect(),
        _ => vec![expr],
    }
}

fn render_call(
    func: &Expr,
    args: &[Expr],
    metadata: &MetadataIndex,
    options: &RenderOptions,
    range: TextRange,
) -> RenderedExpr {
    let qualified = qualified_name(func);
    let short = qualified
        .as_deref()
        .and_then(|name| name.rsplit('.').next())
        .unwrap_or("call");
    let rendered_args = args
        .iter()
        .map(|arg| render_expr(arg, metadata, options, &RenderContext::default(), range))
        .collect::<Vec<_>>();
    let mut diagnostics = rendered_args
        .iter()
        .flat_map(|arg| arg.diagnostics.clone())
        .collect::<Vec<_>>();
    let arg_latex = rendered_args
        .iter()
        .map(|arg| arg.latex.clone())
        .collect::<Vec<_>>();

    let meta = qualified
        .as_ref()
        .and_then(|name| metadata.functions.get(name))
        .or_else(|| metadata.functions.get(short));

    if let Some(meta) = meta {
        if let Some(template) = &meta.latex_template {
            let (latex, mut template_diags) = apply_template(template, &arg_latex, range);
            diagnostics.append(&mut template_diags);
            return RenderedExpr {
                latex,
                precedence: meta.precedence.unwrap_or(100),
                indices: None,
                tensor_notation: false,
                diagnostics,
            };
        }
    }

    RenderedExpr {
        latex: format!(
            r"\operatorname{{{}}}\left({}\right)",
            escape_text(short),
            arg_latex.join(", ")
        ),
        precedence: 100,
        indices: None,
        tensor_notation: false,
        diagnostics,
    }
}

fn render_subscript(
    value: &Expr,
    indices: &[Expr],
    metadata: &MetadataIndex,
    options: &RenderOptions,
    range: TextRange,
) -> RenderedExpr {
    let base_context = RenderContext {
        suppress_tensor_indices: true,
        ..RenderContext::default()
    };
    let base = render_expr(value, metadata, options, &base_context, range);
    let mut diagnostics = base.diagnostics;
    let mut rendered_indices = Vec::new();
    for index in indices {
        let rendered = render_index(index, metadata, options, range);
        rendered_indices.push(rendered.latex);
        diagnostics.extend(rendered.diagnostics);
    }
    let indices = rendered_indices
        .iter()
        .map(|idx| idx.trim_start_matches('\\').to_string())
        .collect::<Vec<_>>();
    RenderedExpr {
        latex: tensor_latex(&base.latex, &rendered_indices, metadata),
        precedence: 100,
        indices: Some(indices),
        tensor_notation: true,
        diagnostics,
    }
}

fn render_index(
    expr: &Expr,
    metadata: &MetadataIndex,
    options: &RenderOptions,
    range: TextRange,
) -> RenderedExpr {
    match expr {
        Expr::Name(name) => {
            let latex = metadata
                .symbols
                .get(name)
                .map(|meta| meta.latex.clone())
                .unwrap_or_else(|| index_piece_latex(name, options));
            RenderedExpr {
                latex,
                precedence: 100,
                indices: None,
                tensor_notation: false,
                diagnostics: Vec::new(),
            }
        }
        Expr::Literal(value) => RenderedExpr {
            latex: value.clone(),
            precedence: 100,
            indices: None,
            tensor_notation: false,
            diagnostics: Vec::new(),
        },
        _ => render_expr(expr, metadata, options, &RenderContext::default(), range),
    }
}

fn tensor_latex(base: &str, indices: &[String], metadata: &MetadataIndex) -> String {
    if indices.is_empty() {
        return base.to_string();
    }
    let rendered = indices
        .iter()
        .map(|idx| {
            metadata
                .symbols
                .get(idx)
                .map(|meta| meta.latex.clone())
                .unwrap_or_else(|| explicit_index_piece_latex(idx))
        })
        .collect::<Vec<_>>()
        .join("");
    format!("{}_{{{rendered}}}", indexed_base_latex(base))
}

fn indexed_base_latex(base: &str) -> String {
    if base.contains('_') || base.contains('^') {
        format!("{{{base}}}")
    } else {
        base.to_string()
    }
}

fn maybe_group(expr: &RenderedExpr, parent_precedence: u8) -> String {
    if expr.precedence < parent_precedence {
        format!(r"\left({}\right)", expr.latex)
    } else {
        expr.latex.clone()
    }
}

fn identifier_latex(name: &str, options: &RenderOptions) -> String {
    let (base, prime) = strip_prime_suffix(name);
    if options.underscore_subscripts && base.contains('_') {
        let mut parts = base.split('_');
        let head = parts.next().unwrap_or_default();
        let subs = parts
            .map(|part| index_piece_latex(part, options))
            .collect::<Vec<_>>()
            .join("");
        return format!(
            "{}_{{{}}}{prime}",
            identifier_base_latex(head, options),
            subs
        );
    }
    format!("{}{prime}", identifier_base_latex(base, options))
}

fn identifier_base_latex(name: &str, options: &RenderOptions) -> String {
    if options.auto_name_symbol {
        if let Some(greek) = greek_latex(name) {
            return greek.to_string();
        }
    }
    if !options.auto_name_symbol && name.chars().all(|ch| ch.is_alphabetic()) {
        return format!(r"\operatorname{{{}}}", escape_text(name));
    }
    if let Some(greek) = unicode_greek_latex(name) {
        return greek.to_string();
    }
    if name.len() == 1 {
        return name.to_string();
    }
    if name.chars().all(|ch| ch.is_ascii_alphabetic()) {
        return format!(r"\operatorname{{{}}}", escape_text(name));
    }
    escape_latex_literal(name)
}

fn index_piece_latex(name: &str, options: &RenderOptions) -> String {
    if name.starts_with('\\') {
        return name.to_string();
    }
    if options.auto_name_symbol {
        if let Some(greek) = greek_latex(name) {
            return greek.to_string();
        }
    }
    escape_latex_literal(name)
}

fn explicit_index_piece_latex(name: &str) -> String {
    if name.starts_with('\\') {
        return name.to_string();
    }
    greek_latex(name)
        .map(ToString::to_string)
        .unwrap_or_else(|| escape_latex_literal(name))
}

fn strip_prime_suffix(name: &str) -> (&str, &'static str) {
    if let Some(base) = name.strip_suffix("_prime") {
        (base, "'")
    } else {
        (name, "")
    }
}

fn greek_latex(name: &str) -> Option<&'static str> {
    match name {
        "α" => Some(r"\alpha"),
        "alpha" => Some(r"\alpha"),
        "β" => Some(r"\beta"),
        "beta" => Some(r"\beta"),
        "γ" => Some(r"\gamma"),
        "gamma" => Some(r"\gamma"),
        "δ" => Some(r"\delta"),
        "delta" => Some(r"\delta"),
        "ε" => Some(r"\epsilon"),
        "epsilon" => Some(r"\epsilon"),
        "θ" => Some(r"\theta"),
        "theta" => Some(r"\theta"),
        "λ" => Some(r"\lambda"),
        "lambda" => Some(r"\lambda"),
        "μ" => Some(r"\mu"),
        "mu" => Some(r"\mu"),
        "ν" => Some(r"\nu"),
        "nu" => Some(r"\nu"),
        "ρ" => Some(r"\rho"),
        "rho" => Some(r"\rho"),
        "σ" => Some(r"\sigma"),
        "sigma" => Some(r"\sigma"),
        "τ" => Some(r"\tau"),
        "tau" => Some(r"\tau"),
        "φ" => Some(r"\phi"),
        "phi" => Some(r"\phi"),
        "ψ" => Some(r"\psi"),
        "psi" => Some(r"\psi"),
        "ω" => Some(r"\omega"),
        "omega" => Some(r"\omega"),
        _ => None,
    }
}

fn unicode_greek_latex(name: &str) -> Option<&'static str> {
    match name {
        "α" => Some(r"\alpha"),
        "β" => Some(r"\beta"),
        "γ" => Some(r"\gamma"),
        "δ" => Some(r"\delta"),
        "ε" => Some(r"\epsilon"),
        "θ" => Some(r"\theta"),
        "λ" => Some(r"\lambda"),
        "μ" => Some(r"\mu"),
        "ν" => Some(r"\nu"),
        "ρ" => Some(r"\rho"),
        "σ" => Some(r"\sigma"),
        "τ" => Some(r"\tau"),
        "φ" => Some(r"\phi"),
        "ψ" => Some(r"\psi"),
        "ω" => Some(r"\omega"),
        _ => None,
    }
}

fn apply_template(template: &str, args: &[String], range: TextRange) -> (String, Vec<Diagnostic>) {
    let mut out = String::new();
    let mut diagnostics = Vec::new();
    let chars = template.chars().collect::<Vec<_>>();
    let mut idx = 0usize;
    while idx < chars.len() {
        match chars[idx] {
            '{' if chars.get(idx + 1) == Some(&'{') => {
                out.push('{');
                idx += 2;
            }
            '}' if chars.get(idx + 1) == Some(&'}') => {
                out.push('}');
                idx += 2;
            }
            '{' => {
                let start = idx + 1;
                let mut end = start;
                while end < chars.len() && chars[end].is_ascii_digit() {
                    end += 1;
                }
                if end < chars.len() && chars[end] == '}' && end > start {
                    let arg_idx = chars[start..end]
                        .iter()
                        .collect::<String>()
                        .parse::<usize>()
                        .unwrap_or(usize::MAX);
                    if let Some(arg) = args.get(arg_idx) {
                        out.push_str(arg);
                    } else {
                        diagnostics.push(Diagnostic {
                            code: "mathdocs::template-arity".to_string(),
                            message: format!(
                                "template references argument {arg_idx}, but call has {} argument(s)",
                                args.len()
                            ),
                            range,
                            severity: DiagnosticSeverity::Warning,
                        });
                    }
                    idx = end + 1;
                } else {
                    out.push(chars[idx]);
                    idx += 1;
                }
            }
            ch => {
                out.push(ch);
                idx += 1;
            }
        }
    }
    (out, diagnostics)
}

fn escape_text(value: &str) -> String {
    value
        .replace('\\', r"\textbackslash{}")
        .replace('{', r"\{")
        .replace('}', r"\}")
}

fn escape_latex_literal(value: &str) -> String {
    value.replace('_', r"\_")
}

#[allow(dead_code)]
fn symbol_tensor(meta: &SymbolMeta) -> Option<Vec<String>> {
    meta.tensor
        .as_ref()
        .map(|tensor| tensor.indices.iter().map(|idx| idx.name.clone()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mathdocs_ast::TextRange;
    use mathdocs_ir::lower_statement;
    use mathdocs_metadata::extract_inline_metadata;

    #[test]
    fn renders_templates_and_tensors() {
        let source = r#"
A: Annotated[np.ndarray, Tensor("A", ("i", "j"))]
x: Annotated[np.ndarray, Tensor("x", ("j",))]
y: Annotated[np.ndarray, Tensor("y", ("i",))]
@render_as(latex=r"\left\|{0}\right\|")
def norm(v): ...
"#;
        let metadata = extract_inline_metadata(source);
        let lowered = lower_statement("loss = norm(y - A @ x)", TextRange { start: 0, end: 0 });
        let rendered = render_statement(
            &lowered.statement,
            &metadata,
            &RenderOptions::default(),
            TextRange { start: 0, end: 0 },
        );
        assert!(rendered.latex.contains(r"\left\|"));
        assert!(rendered.latex.contains("A_{ij}x_{j}"));
    }

    #[test]
    fn groups_tensor_names_before_appending_indices() {
        let source = r#"
X: Annotated[object, Tensor("X", ("t", "d"))]
W_Q: Annotated[object, Tensor("W_Q", ("d", "k"))]
Q: Annotated[object, Tensor("Q", ("t", "k"))]
"#;
        let metadata = extract_inline_metadata(source);
        let lowered = lower_statement("Q = X @ W_Q", TextRange { start: 0, end: 0 });
        let rendered = render_statement(
            &lowered.statement,
            &metadata,
            &RenderOptions::default(),
            TextRange { start: 0, end: 0 },
        );
        assert!(rendered.latex.contains(r"{W_Q}_{dk}"));
        assert!(!rendered.latex.contains(r"W_Q_{dk}"));
    }

    #[test]
    fn renders_symbol_lookup_expansions() {
        let metadata = mathdocs_metadata::MetadataIndex::with_builtins();
        let lowered = lower_statement("ω = ::omega:: + 1", TextRange { start: 0, end: 0 });
        let rendered = render_statement(
            &lowered.statement,
            &metadata,
            &RenderOptions::default(),
            TextRange { start: 0, end: 0 },
        );
        assert!(lowered.diagnostics.is_empty());
        assert_eq!(rendered.latex, r"\omega = \omega + 1");
    }

    #[test]
    fn can_disable_auto_name_subscripts() {
        let metadata = mathdocs_metadata::MetadataIndex::with_builtins();
        let lowered = lower_statement("final_loss = 1", TextRange { start: 0, end: 0 });
        let rendered = render_statement(
            &lowered.statement,
            &metadata,
            &RenderOptions {
                underscore_subscripts: false,
                ..RenderOptions::default()
            },
            TextRange { start: 0, end: 0 },
        );
        assert_eq!(rendered.latex, r"final\_loss = 1");
    }

    #[test]
    fn can_disable_auto_name_symbols() {
        let metadata = mathdocs_metadata::MetadataIndex::with_builtins();
        let lowered = lower_statement("alpha = beta", TextRange { start: 0, end: 0 });
        let rendered = render_statement(
            &lowered.statement,
            &metadata,
            &RenderOptions {
                auto_name_symbol: false,
                ..RenderOptions::default()
            },
            TextRange { start: 0, end: 0 },
        );
        assert_eq!(
            rendered.latex,
            r"\operatorname{alpha} = \operatorname{beta}"
        );
    }
}
