# pyright: reportUnboundVariable=false, reportOperatorIssue=false
from typing import Annotated, Any

from mathdocs import Symbol, Tensor, render_as

"""
# MathDocs Feature Showcase

This document is written as ordinary Python source, but it is intended to read
like documentation after MathDocs converts it to Markdown and LaTeX. The
examples below cover the core pieces used by the renderer: Markdown blocks,
symbol metadata, tensor metadata, arithmetic, powers, exponentials, function
templates, indexing, and named equations.

The file does not need to be executable. MathDocs reads it statically, so the
annotations and decorators are enough to describe how each expression should be
rendered.
"""

n: Annotated[int, Symbol("n")]
t: Annotated[float, Symbol("t")]
r: Annotated[float, Symbol("r")]
x: Annotated[float, Symbol("x")]
x_0: Annotated[float, Symbol("x_0")]
mu: Annotated[float, Symbol(r"\mu")]
sigma: Annotated[float, Symbol(r"\sigma")]
alpha: Annotated[float, Symbol(r"\alpha")]
beta: Annotated[float, Symbol(r"\beta")]
coefficient: Annotated[float, Symbol("a")]
pi: Annotated[float, Symbol(r"\pi")]
theta: Annotated[Any, Symbol(r"\theta")]
loss: Annotated[float, Symbol(r"\mathcal{L}")]
expected_loss: Annotated[float, Symbol(r"\mathbb{E}[\mathcal{L}]")]

i: Annotated[int, Symbol("i")]
j: Annotated[int, Symbol("j")]
k: Annotated[int, Symbol("k")]

A: Annotated[Any, Tensor("A", ("i", "j"))]
B: Annotated[Any, Tensor("B", ("j", "k"))]
C: Annotated[Any, Tensor("C", ("i", "k"))]
W: Annotated[Any, Tensor("W", ("d", "m"))]
X: Annotated[Any, Tensor("X", ("n", "d"))]
X_centered: Annotated[Any, Tensor(r"\tilde{X}", ("n", "d"))]
b: Annotated[Any, Tensor("b", ("i",))]
u: Annotated[Any, Tensor("u", ("j",))]
v: Annotated[Any, Tensor("v", ("i",))]
y: Annotated[Any, Tensor("y", ("i",))]
scores: Annotated[Any, Tensor("s", ("i",))]
probabilities: Annotated[Any, Tensor("p", ("i",))]
p: Annotated[Any, Tensor("p", ("i",))]


@render_as(latex=r"e^{{{0}}}")
def exp(value): ...


@render_as(latex=r"\sqrt{{{0}}}")
def sqrt(value): ...


@render_as(latex=r"\log\left({0}\right)")
def log(value): ...


@render_as(latex=r"{0}^T")
def transpose(value): ...


@render_as(latex=r"\left\|{0}\right\|")
def norm(value): ...


@render_as(latex=r"\operatorname{{softmax}}\left({0}\right)")
def softmax(value): ...


@render_as(latex=r"\nabla_{{{1}}} {0}")
def grad(value, variable): ...


@render_as(latex=r"\sum_{{{1}=1}}^{{{2}}} {0}")
def sum_over(value, index, upper): ...


@render_as(latex=r"\int_{{{0}}}^{{{1}}} {2}\,d{3}")
def integral(lower, upper, value, variable): ...


"""
## Symbols and Arithmetic

Names can be assigned display metadata with `Symbol`. When an expression uses
those names, MathDocs substitutes the LaTeX form and preserves ordinary
Python operator structure.
"""

centered = x - mu
scaled = (x - mu) / sigma
quadratic = (x - mu) ** 2 / sigma ** 2

"""
## Powers and Exponentials

Python's `**` operator renders as a superscript. Exponential functions can be
defined with a `render_as` template, which is useful when the source code uses a
plain helper such as `exp(...)` but the document should show mathematical
notation.
"""

growth = x_0 * exp(r * t)
power_law = coefficient * x ** beta
gaussian_density = exp(-((x - mu) ** 2) / (2 * sigma ** 2)) / (sigma * sqrt(2 * pi))
log_likelihood = log(gaussian_density)

"""
## Tensor Metadata

`Tensor` metadata describes the rendered tensor name and its logical indices.
Matrix multiplication uses those indices to display contractions.
"""

y = A @ u + b
C = A @ B

"""
Explicit indexing can be used when a derivation needs to show a component-level
formula instead of inferred matrix notation.
"""

eq_component_update = y[i] == A[i, j] * u[j] + b[i]
eq_matrix_product = C[i, k] == sum_over(A[i, j] * B[j, k], j, n)

"""
## Function Templates

Decorated helper functions can render as conventional mathematical notation
without changing the source expression shape. The template arguments are
numbered from left to right.
"""

scores = A @ u
probabilities = softmax(scores)
regularized_loss = loss + alpha * norm(theta) ** 2
expected_loss = sum_over(loss, i, n) / n
area = integral(0, 1, x ** 2, x)

"""
## Transpose, Fractions, and Grouping

Parentheses in Python are preserved where they affect grouping. Division
renders as a fraction, and custom templates can supply notation such as
transpose.
"""

transposed_features = transpose(X_centered)
normalized_distance = norm(y - v) / sqrt(n)

"""
## Equations

Assignments whose target starts with `eq_` are treated as named equations. The
rendered document shows the equation itself rather than the internal Python
variable name.
"""

eq_stationary = grad(regularized_loss, theta) == 0
eq_probability_mass = sum_over(p[i], i, n) == 1
