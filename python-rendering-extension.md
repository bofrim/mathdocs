# Python Math Rendering Extension - Implementation Design

This document describes a Python source-rendering system that lets ordinary
Python code display as mathematical notation inside an editor. The underlying
program remains normal Python. Rendering metadata is visible to static tools
but has no required runtime behavior.

The intended product is a code editor extension that renders selected Python
source ranges as Markdown plus KaTeX/LaTeX, with support for variable display
names, custom function renderers, tensor notation, and literate prose blocks.

---

## 1. Goals

### 1.1 Primary goals

- Let Python source render like mathematical text without changing Python
  semantics.
- Use `typing.Annotated` for variable, parameter, and field display metadata.
- Use decorators for function rendering metadata.
- Treat bare multiline string expressions as Markdown prose blocks.
- Support nested expression rendering, fractions, powers, norms, derivatives,
  tensor indices, contractions, exterior calculus, and gauge-theory style
  notation.
- Ship as a language-server/editor-extension feature, not as a new Python
  runtime.
- Make the rendering layer zero-cost for the executed program.

### 1.2 Non-goals

- Do not replace Python's type checker.
- Do not require custom Python syntax.
- Do not evaluate arbitrary Python while rendering.
- Do not require users to wrap numeric values in symbolic expression objects.
- Do not make rendered equations authoritative program semantics. The Python
  code remains the source of truth.

---

## 2. User model

A user writes normal Python:

```python
from typing import Annotated
import numpy as np
from mathdocs import Symbol, Tensor, render_as

theta: Annotated[float, Symbol(r"\theta")]
mu: Annotated[float, Symbol(r"\mu")]
sigma: Annotated[float, Symbol(r"\sigma")]
x: Annotated[np.ndarray, Tensor("x", ("i",))]
A: Annotated[np.ndarray, Tensor("A", ("i", "j"))]

@render_as(latex=r"\left\|{0}\right\|")
def norm(x):
    return np.linalg.norm(x)

@render_as(latex=r"{0}^{2}")
def square(x):
    return x * x

z = (abs(theta - mu) + square(norm(A @ x))) / (sigma + 1)
```

The editor can render the final assignment as:

$$
z =
\frac{
  \left|\theta - \mu\right| + \left\|A_{ij}x_j\right\|^2
}{
  \sigma + 1
}
$$

The Python interpreter still sees ordinary Python. If the editor extension is
not installed, the file still runs.

---

## 3. Zero-cost requirement

The rendering system must not impose runtime cost on the numerical program.

There are two practical profiles:

- Strict zero-cost profile: no runtime imports, no runtime decorators, and no
  evaluated annotation metadata. Use stringized annotations plus `.pyi` or
  sidecar metadata for functions.
- Convenient authoring profile: inline metadata classes and decorators are
  allowed. This is easier to read, but decorators still execute during import
  unless the project supplies no-op definitions.

The editor extension must support both. Documentation and examples can show the
convenient form first, but production guidance should identify the strict
profile clearly.

### 3.1 Strong zero-cost mode

In strong zero-cost mode, rendering metadata is erased from runtime execution
for annotations. This is the recommended variable-metadata pattern for
production code.

Users write:

```python
from __future__ import annotations
from typing import Annotated, TYPE_CHECKING

if TYPE_CHECKING:
    from mathdocs import Symbol, Tensor, render_as
else:
    def render_as(*args, **kwargs):
        def deco(fn):
            return fn
        return deco

theta: Annotated[float, "Symbol(r'\\theta')"] = 0.1
```

With `from __future__ import annotations`, annotations are stored as strings
and are not evaluated during normal import. The editor parses the source text,
so it does not need runtime annotation objects.

Python 3.14 changes default annotation behavior to deferred evaluation, but
`from __future__ import annotations` still keeps the older stringized behavior
for now. The extension should not depend on either runtime annotation model. It
should read the source syntax directly.

The decorator fallback is an identity decorator. Its cost is paid once at
import time for decorated functions. For absolute zero import-time overhead,
function render metadata must be moved into `.pyi` files or sidecars.

### 3.2 Stub metadata mode

For libraries where even identity decorators are undesirable, metadata can live
in `.pyi` files:

```python
# model.py
def norm(x): ...

# model.pyi
from mathdocs import render_as

@render_as(latex=r"\left\|{0}\right\|")
def norm(x): ...
```

The extension merges source and stub metadata. The runtime module has no
rendering imports, no decorators, and no annotation metadata beyond what the
project already chooses to keep.

This is the strict zero-cost mechanism for functions.

### 3.3 Editor-only sidecar metadata

A third mode stores render metadata in a sidecar file:

```toml
# model.mathdocs.toml
[symbols]
theta = "\\theta"
sigma = "\\sigma"

[functions.norm]
latex = "\\left\\|{0}\\right\\|"
```

This is useful for third-party code or generated code. Sidecars are not the
primary authoring experience but are important for adoption.

---

## 4. Python library surface

The Python package should be tiny. Its runtime job is mostly to make source
valid and type-checkable.

```python
from dataclasses import dataclass
from typing import Any, Callable

@dataclass(frozen=True)
class Symbol:
    latex: str
    text: str | None = None

@dataclass(frozen=True)
class Tensor:
    latex: str
    indices: tuple[str, ...] = ()
    variance: tuple[str, ...] | None = None
    text: str | None = None

@dataclass(frozen=True)
class RenderTemplate:
    latex: str
    text: str | None = None
    precedence: int | None = None

def render_as(**formats: str) -> Callable[[Callable[..., Any]], Callable[..., Any]]:
    def deco(fn: Callable[..., Any]) -> Callable[..., Any]:
        return fn
    return deco
```

The default implementation should not attach runtime attributes. Static tools
read source. A debugging mode may attach metadata, but that must be opt-in.

### 4.1 Variable metadata

```python
theta: Annotated[float, Symbol(r"\theta")] = 0.1
A: Annotated[np.ndarray, Tensor("A", ("i", "j"))] = np.eye(3)
```

Static interpretation:

- `theta` renders as `\theta`.
- `A` renders as `A_{ij}` when used as a tensor.
- The actual Python values are `float` and `np.ndarray`.

### 4.2 Function metadata

```python
@render_as(latex=r"\left|{0}\right|")
def abs_value(x):
    return abs(x)

@render_as(latex=r"\frac{{{0}}}{{{1}}}")
def frac(a, b):
    return a / b
```

Static interpretation:

- Calls to `abs_value(expr)` render as `\left|expr\right|`.
- Calls to `frac(a, b)` render as a fraction.

Built-in functions may have default render metadata. For example, `abs(x)` can
render as `\left|x\right|`.

---

## 5. Source format

The system uses valid Python files. A renderable document is a Python module
whose top-level statements are interpreted as render blocks.

### 5.1 Markdown blocks

A top-level bare string expression renders as Markdown:

```python
"""
# Electrodynamics

The electromagnetic field strength is the curvature of a U(1) connection.
"""
```

Output:

```markdown
# Electrodynamics

The electromagnetic field strength is the curvature of a U(1) connection.
```

This rule also applies inside functions/classes only if the editor is asked to
render that local range. The module docstring is treated as a Markdown block
only in document-rendering mode.

### 5.2 Math blocks

Assignments, annotated assignments, comparisons, and selected expression
statements can render as math:

```python
F = d(A)
eq_source = d(star(F)) == star(J)
```

Output:

$$
F = dA
$$

$$
d\star F = \star J
$$

### 5.3 Suppression

Users need a way to keep ordinary code from rendering:

```python
# mathdocs: ignore
cache = expensive_runtime_cache()
```

Range-level suppression:

```python
# mathdocs: off
...
# mathdocs: on
```

---

## 6. Static extraction pipeline

The editor extension should never import the target module to render it.

Pipeline:

1. Parse Python source into a concrete syntax tree.
2. Extract render metadata from annotations, decorators, stubs, and sidecars.
3. Build a scoped symbol table.
4. Convert selected Python expressions into a render IR.
5. Resolve names and function calls against metadata.
6. Render IR to LaTeX, Markdown, text, or editor decorations.
7. Cache results by file version and source range.

### 6.1 Parser choice

Use Rust for parsing and incremental analysis.

Recommended crates:

- `ruff_python_parser` or `rustpython-parser` for Python AST.
- `ruff_python_ast` if using the Ruff ecosystem.
- `rowan` or Ruff's own range model for stable source ranges.
- `pyo3` only for optional Python bindings, not for editor-time execution.

The parser must preserve source ranges for every node so rendered output can be
attached to editor spans.

### 6.2 Why Rust here

Parsing, range tracking, incremental re-rendering, and code generation are on
the editor hot path. They should be fast and memory-predictable. Rust is also a
good fit for shipping a native language server binary.

Python remains useful for:

- author-facing metadata classes;
- optional CLI prototypes;
- tests that compare source snippets to rendered output;
- notebooks or documentation tooling.

---

## 7. Metadata model

The Rust analyzer should normalize all metadata into a single internal form.

```rust
struct SymbolMeta {
    name: String,
    latex: String,
    text: Option<String>,
    tensor: Option<TensorMeta>,
    source_range: TextRange,
}

struct TensorMeta {
    indices: Vec<IndexMeta>,
}

struct IndexMeta {
    name: String,
    variance: Variance,
}

enum Variance {
    Covariant,
    Contravariant,
    Unspecified,
}

struct FunctionRenderMeta {
    qualified_name: String,
    latex_template: Option<String>,
    text_template: Option<String>,
    kind: FunctionRenderKind,
}

enum FunctionRenderKind {
    Template,
    PrefixOperator,
    InfixOperator,
    SpecialForm,
}
```

### 7.1 Annotation extraction

Supported forms:

```python
x: Annotated[float, Symbol("x")]
theta: Annotated[float, Symbol(r"\theta")]
A: Annotated[np.ndarray, Tensor("A", ("i", "j"))]
```

The analyzer should parse these as syntax, not evaluate them. It only needs to
recognize known constructors by name:

- `Symbol(...)`
- `Tensor(...)`
- `Index(...)`
- `Display(...)`, if added later

Unknown `Annotated` metadata is ignored.

### 7.2 Decorator extraction

Supported forms:

```python
@render_as(latex=r"\sqrt{{{0}}}")
def sqrt(x): ...
```

The analyzer records metadata for the function symbol `sqrt`. It should also
support qualified decorators:

```python
@mathdocs.render_as(latex=r"\operatorname{tr}\left({0}\right)")
def trace(x): ...
```

### 7.3 Stub and sidecar merge order

Metadata priority should be deterministic:

1. Inline source metadata.
2. Adjacent `.pyi` metadata.
3. Project sidecar metadata.
4. Built-in default metadata.

Higher-priority metadata overrides lower-priority metadata for the same
qualified name.

---

## 8. Render IR

The analyzer should not render directly from the Python AST. It should lower
Python nodes into a small render-specific IR.

```rust
enum Expr {
    Name(NameExpr),
    Literal(LiteralExpr),
    Unary(UnaryExpr),
    Binary(BinaryExpr),
    Compare(CompareExpr),
    Call(CallExpr),
    Attribute(AttributeExpr),
    Subscript(SubscriptExpr),
    Assignment(AssignmentExpr),
    TensorProduct(TensorProductExpr),
    Fraction(Box<Expr>, Box<Expr>),
    Group(Box<Expr>),
    RawLatex(String),
    Unsupported(UnsupportedExpr),
}
```

Every IR node carries:

- source range;
- precedence;
- optional inferred tensor indices;
- optional diagnostics.

### 8.1 Lowering examples

Python:

```python
z = (abs(theta - mu) + square(norm(A @ x))) / sqrt(square(sigma) + 1)
```

IR:

```text
Assignment
  target: Name(z)
  value: Fraction
    numerator: Binary(+)
      Call(abs)
        Binary(-)
          Name(theta)
          Name(mu)
      Call(square)
        Call(norm)
          Binary(@)
            Name(A)
            Name(x)
    denominator: Call(sqrt)
      Binary(+)
        Call(square)
          Name(sigma)
        Literal(1)
```

LaTeX:

$$
z =
\frac{
  \left|\theta - \mu\right| + \left\|A_{ij}x_j\right\|^2
}{
  \sqrt{\sigma^2 + 1}
}
$$

---

## 9. Rendering rules

### 9.1 Names

If a name has `Symbol` metadata, use that display form:

```python
theta: Annotated[float, Symbol(r"\theta")]
```

`theta` renders as:

$$
\theta
$$

Otherwise render the Python identifier. Identifiers containing underscores
should render with subscripts by default:

```python
F_mu_nu
```

Default rendering:

$$
F_{\mu\nu}
$$

This default should be configurable because some projects use underscores for
ordinary names.

### 9.2 Binary operators

Default mappings:

```text
a + b  -> a + b
a - b  -> a - b
a * b  -> ab, or a \cdot b when needed
a / b  -> \frac{a}{b}
a ** b -> a^{b}
a @ b  -> tensor/matrix contraction if metadata exists, otherwise ab
```

The renderer must use precedence-aware grouping:

```python
(a + b) / (c + d)
```

renders as:

$$
\frac{a + b}{c + d}
$$

### 9.3 Calls

If a function has template metadata:

```python
@render_as(latex=r"\left\|{0}\right\|")
def norm(x): ...
```

then:

```python
norm(A @ x)
```

renders as:

$$
\left\|A_{ij}x_j\right\|
$$

Otherwise, use ordinary function notation:

```python
foo(a, b)
```

$$
\operatorname{foo}(a, b)
$$

### 9.4 Attributes

Attributes render as qualified names by default:

```python
np.sin(x)
```

$$
\operatorname{sin}(x)
$$

Known modules can be collapsed:

- `math.sin`, `np.sin` -> `\sin`
- `math.exp`, `np.exp` -> `\exp`
- `np.linalg.norm` -> `\left\|{0}\right\|`

### 9.5 Subscripts

Subscripted expressions map naturally to indexed notation:

```python
A[i, j]
```

$$
A_{ij}
$$

Slicing can render as set/range notation or stay as code depending on context:

```python
x[1:]
```

$$
x_{1:}
$$

This should be marked as a lower-confidence rendering unless the user opts into
slice math rendering.

---

## 10. Tensor rendering

Tensor rendering is the main reason to use metadata rather than only string
templates.

### 10.1 Tensor declarations

```python
from typing import Annotated
import numpy as np

A: Annotated[np.ndarray, Tensor("A", ("i", "j"))]
B: Annotated[np.ndarray, Tensor("B", ("j", "k"))]
x: Annotated[np.ndarray, Tensor("x", ("k",))]
y: Annotated[np.ndarray, Tensor("y", ("i",))]
```

### 10.2 Matrix-vector product

Input:

```python
y = A @ x
```

Output:

$$
y_i = A_{ij}x_j
$$

The analyzer infers:

- `A` has free index `i` and contracted index `j`;
- `x` has contracted index `j`;
- output has free index `i`;
- assignment target `y` has index `i`.

If `x` was declared with index `k`, the contraction may either:

- preserve declared names and render `A_{ij}x_k`, with a diagnostic that no
  contraction is obvious; or
- alpha-rename the vector index to match the matrix's second index.

The first prototype should prefer explicitness and diagnostics over aggressive
renaming.

### 10.3 Matrix-matrix-vector product

Input:

```python
y = A @ B @ x
```

Output:

$$
y_i = A_{ij}B_{jk}x_k
$$

IR index propagation:

```text
A       : (i, j)
B       : (j, k)
A @ B   : (i, k), latex A_{ij}B_{jk}
x       : (k)
(A@B)@x : (i), latex A_{ij}B_{jk}x_k
```

### 10.4 Explicit index access

Input:

```python
F_mu_nu = partial(mu, A[nu]) - partial(nu, A[mu])
```

Output:

$$
F_{\mu\nu}
=
\partial_\mu A_\nu
-
\partial_\nu A_\mu
$$

The renderer can combine name parsing (`F_mu_nu`) with explicit subscript
nodes (`A[nu]`).

### 10.5 Variance

Optional variance metadata:

```python
F: Annotated[np.ndarray, Tensor("F", (("mu", "down"), ("nu", "up")))]
```

Output:

$$
F_{\mu}^{\nu}
$$

The short tuple form should be supported later. The first prototype can treat
all indices as lower indices.

---

## 11. Differential and gauge notation

The system should support mathematical operators through decorated functions or
built-in render metadata.

### 11.1 Authoring code

```python
from typing import Annotated
from mathdocs import Symbol, Tensor, render_as

mu: Annotated[Index, Symbol(r"\mu")]
nu: Annotated[Index, Symbol(r"\nu")]
rho: Annotated[Index, Symbol(r"\rho")]

A: Annotated[OneForm, Tensor("A", (r"\mu",))]
F: Annotated[TwoForm, Tensor("F", (r"\mu", r"\nu"))]
J: Annotated[VectorDensity, Tensor("J", (r"\mu",))]
alpha: Annotated[Scalar, Symbol(r"\alpha")]
psi: Annotated[Spinor, Symbol(r"\psi")]
e: Annotated[float, Symbol("e")]

@render_as(latex=r"d{0}")
def d(x): ...

@render_as(latex=r"\star {0}")
def star(x): ...

@render_as(latex=r"\partial_{0} {1}")
def partial(index, expr): ...

@render_as(latex=r"e^{{{0}}}")
def exp(x): ...

"""
# Electrodynamics as a U(1) Gauge Theory

The gauge potential is a one-form. Its curvature is the electromagnetic field
strength.
"""

F = d(A)

F_mu_nu = partial(mu, A[nu]) - partial(nu, A[mu])

"""
## Gauge transformations
"""

A_prime = A + d(alpha)
psi_prime = exp(i * e * alpha) * psi

"""
## Maxwell equations
"""

eq_bianchi = d(F) == 0
eq_source = d(star(F)) == star(J)
```

### 11.2 Rendered Markdown and KaTeX

```markdown
# Electrodynamics as a U(1) Gauge Theory

The gauge potential is a one-form. Its curvature is the electromagnetic field
strength.

$$
F = dA
$$

$$
F_{\mu\nu}
=
\partial_\mu A_\nu
-
\partial_\nu A_\mu
$$

## Gauge transformations

$$
A' = A + d\alpha
$$

$$
\psi' = e^{i e \alpha}\psi
$$

## Maxwell equations

$$
dF = 0
$$

$$
d\star F = \star J
$$
```

Displayed:

$$
F = dA
$$

$$
F_{\mu\nu}
=
\partial_\mu A_\nu
-
\partial_\nu A_\mu
$$

$$
A' = A + d\alpha
$$

$$
\psi' = e^{i e \alpha}\psi
$$

$$
dF = 0
$$

$$
d\star F = \star J
$$

---

## 12. Editor extension architecture

### 12.1 Components

```text
VS Code / editor client
  |
  | LSP requests, decorations, hover, code lens
  v
mathdocs language server (Rust)
  |
  | parse, index, render, diagnostics
  v
project source files, .pyi stubs, sidecars
```

### 12.2 Language server responsibilities

- Maintain a parsed snapshot of open Python files.
- Track project-level metadata from imports, stubs, and sidecars.
- Provide render previews for:
  - current line;
  - selected range;
  - top-level document;
  - hover tooltips;
  - inline ghost renderings.
- Provide diagnostics for unsupported or ambiguous renderings.
- Export rendered Markdown for documentation builds.

### 12.3 Editor features

Minimum viable extension:

- command: `MathDocs: Preview Current File`;
- command: `MathDocs: Preview Selection`;
- hover: show rendered equation for expression under cursor;
- code lens above renderable top-level assignments;
- diagnostics for unknown render metadata.

Later features:

- inline rendered overlays;
- split-pane rendered document;
- copy as LaTeX;
- copy as Markdown;
- export HTML with KaTeX;
- render-on-save documentation output.

### 12.4 LSP extensions

Standard LSP does not define a math-render request. Add custom methods:

```text
mathRender/renderRange
mathRender/renderDocument
mathRender/renderHover
mathRender/listBlocks
```

Example response:

```json
{
  "kind": "markdown",
  "range": {
    "start": { "line": 42, "character": 0 },
    "end": { "line": 42, "character": 78 }
  },
  "markdown": "$$\\ny = A_{ij}x_j + b_i\\n$$",
  "diagnostics": []
}
```

---

## 13. Rust crates and binaries

Suggested workspace:

```text
crates/
  mathdocs_ast/        Python parse wrappers and source ranges
  mathdocs_metadata/   Annotation/decorator/stub/sidecar extraction
  mathdocs_ir/         Render IR and lowering
  mathdocs_latex/      LaTeX renderer
  mathdocs_markdown/   Markdown document assembly
  mathdocs_lsp/        Language server
  mathdocs_cli/        CLI for tests and docs
python/
  mathdocs/            Tiny Python package with metadata stubs
editors/
  vscode/                VS Code extension client
```

### 13.1 CLI

The CLI is useful before the editor extension exists:

```bash
mathdocs render examples/electrodynamics/electrodynamics.py --format markdown
mathdocs render examples/electrodynamics/electrodynamics.py --range 20:1-28:1
mathdocs symbols examples/electrodynamics/electrodynamics.py
```

### 13.2 Python package

The Python package should be publishable separately:

```text
mathdocs/
  __init__.py
  py.typed
```

The package should work with `mypy`, `pyright`, and normal Python imports.

---

## 14. Diagnostics

Rendering diagnostics should be non-blocking. They should never prevent the
Python program from running.

Examples:

- Unknown metadata constructor: `Annotated[float, FancySymbol("x")]`.
- Template arity mismatch: template uses `{1}` but function has one argument.
- Ambiguous tensor contraction: `A @ x` where declared indices do not align.
- Unsupported Python construct: lambda, comprehension, walrus, pattern match.
- Potentially misleading rendering: division rendered as fraction across a very
  large expression.

Diagnostic example:

```text
warning[mathdocs::tensor-index]:
  cannot infer contraction for A_{ij} @ x_k; no shared index
```

---

## 15. Testing strategy

### 15.1 Golden rendering tests

Input:

```python
theta: Annotated[float, Symbol(r"\theta")]
mu: Annotated[float, Symbol(r"\mu")]
z = abs(theta - mu)
```

Expected LaTeX:

```latex
z = \left|\theta - \mu\right|
```

Golden tests should operate on:

- expression rendering;
- document rendering;
- range rendering;
- metadata extraction;
- tensor index inference;
- diagnostics.

### 15.2 Parser compatibility tests

Run parser tests across supported Python syntax versions:

- Python 3.10;
- Python 3.11;
- Python 3.12;
- Python 3.13;
- Python 3.14.

The renderer does not need to support every expression form initially, but it
must parse files without crashing.

### 15.3 Editor integration tests

Use fixture projects and assert LSP responses:

- open file;
- request render range;
- edit symbol annotation;
- confirm rendered output updates incrementally;
- confirm diagnostics clear after fix.

---

## 16. Security model

The extension must not import or execute user code during rendering.

Allowed:

- parse source text;
- parse `.pyi` text;
- parse sidecar TOML;
- read project configuration.

Disallowed:

- importing the target module;
- evaluating annotations;
- calling decorated functions;
- running user-defined render hooks inside the language server.

If custom render hooks are needed later, they should run in a separate,
explicitly enabled sandboxed process.

---

## 17. Implementation phases

### Phase 1: static expression renderer

- Parse one Python file.
- Extract `Symbol` metadata from `Annotated`.
- Render names, literals, arithmetic, powers, division, assignment.
- Support Markdown string blocks.
- Provide CLI output as Markdown.

### Phase 2: function templates

- Extract `@render_as(...)` metadata.
- Render nested calls with template substitution.
- Add built-in render rules for `abs`, `sqrt`, `sin`, `cos`, `exp`, `log`.
- Add diagnostics for template arity.

### Phase 3: tensors

- Extract `Tensor` metadata.
- Render subscripts and `@`.
- Track simple contraction indices.
- Render assignment targets with inferred output indices.

### Phase 4: language server

- Implement document open/change/close.
- Implement `mathRender/renderRange` and `mathRender/renderDocument`.
- Build VS Code preview command.
- Add hover previews.

### Phase 5: stubs, sidecars, and zero-cost hardening

- Merge `.pyi` metadata.
- Read sidecar TOML.
- Document strong zero-cost patterns.
- Add project configuration.

### Phase 6: richer math

- Differential forms.
- Variational notation.
- Summations and reductions.
- Piecewise expressions.
- Configurable style profiles.

---

## 18. Open design decisions

- Whether inline `Annotated[..., Symbol(...)]` should be recommended over
  stringized metadata in production examples.
- Whether function decorators should be the primary function-rendering mechanism
  or whether `.pyi` stubs should be preferred for strict zero-cost projects.
- How aggressive tensor index alpha-renaming should be.
- Whether underscore-to-subscript should be on by default.
- How much formatting control users need before the renderer becomes too
  configurable to reason about.

---

## 19. Minimal end-to-end example

Input file:

```python
from __future__ import annotations
from typing import Annotated
import numpy as np
from mathdocs import Symbol, Tensor, render_as

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
    return np.linalg.norm(v)

loss = norm(y - (A @ x + b)) / sigma
```

Rendered Markdown:

```markdown
# Linear model

The prediction is a matrix-vector product plus a bias term.

$$
\operatorname{loss}
=
\frac{
  \left\|y_i - \left(A_{ij}x_j + b_i\right)\right\|
}{
  \sigma
}
$$
```

Displayed:

$$
\operatorname{loss}
=
\frac{
  \left\|y_i - \left(A_{ij}x_j + b_i\right)\right\|
}{
  \sigma
}
$$

This example demonstrates the intended shape of the product: ordinary Python
source, static metadata, no required runtime symbolic layer, and editor-side
rendering suitable for code review, documentation, and mathematical reading.
