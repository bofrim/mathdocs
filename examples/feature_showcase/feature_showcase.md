# MathDocs Feature Showcase

This document is written as ordinary Python source, but it is intended to read
like documentation after MathDocs converts it to Markdown and LaTeX. The
examples below cover the core pieces used by the renderer: Markdown blocks,
symbol metadata, tensor metadata, arithmetic, powers, exponentials, function
templates, indexing, and named equations.

The file does not need to be executable. MathDocs reads it statically, so the
annotations and decorators are enough to describe how each expression should be
rendered.

## Symbols and Arithmetic

Names can be assigned display metadata with `Symbol`. When an expression uses
those names, MathDocs substitutes the LaTeX form and preserves ordinary
Python operator structure.

$$
\operatorname{centered} = x - \mu
$$

$$
\operatorname{scaled} = \frac{\left(x - \mu\right)}{\sigma}
$$

$$
\operatorname{quadratic} = \frac{\left(x - \mu\right)^{2}}{\sigma^{2}}
$$

## Powers and Exponentials

Python's `**` operator renders as a superscript. Exponential functions can be
defined with a `render_as` template, which is useful when the source code uses a
plain helper such as `exp(...)` but the document should show mathematical
notation.

$$
\operatorname{growth} = x_0e^{rt}
$$

$$
\operatorname{power}_{law} = ax^{\beta}
$$

$$
\operatorname{gaussian}_{density} = \frac{e^{-\frac{\left(\left(x - \mu\right)^{2}\right)}{\left(2\sigma^{2}\right)}}}{\left(\sigma\sqrt{2\pi}\right)}
$$

$$
\operatorname{log}_{likelihood} = \log\left(\operatorname{gaussian}_{density}\right)
$$

## Tensor Metadata

`Tensor` metadata describes the rendered tensor name and its logical indices.
Matrix multiplication uses those indices to display contractions.

$$
y_{i} = A_{ij}u_{j} + b_{i}
$$

$$
C_{ik} = A_{ij}B_{jk}
$$

Explicit indexing can be used when a derivation needs to show a component-level
formula instead of inferred matrix notation.

$$
y_{i} = A_{ij}u_{j} + b_{i}
$$

$$
C_{ik} = \sum_{j=1}^{n} A_{ij}B_{jk}
$$

## Function Templates

Decorated helper functions can render as conventional mathematical notation
without changing the source expression shape. The template arguments are
numbered from left to right.

$$
s_{i} = A_{ij}u_{j}
$$

$$
p = \operatorname{softmax}\left(s\right)
$$

$$
\operatorname{regularized}_{loss} = \mathcal{L} + \alpha\left\|\theta\right\|^{2}
$$

$$
\mathbb{E}[\mathcal{L}] = \frac{\sum_{i=1}^{n} \mathcal{L}}{n}
$$

$$
\operatorname{area} = \int_{0}^{1} x^{2}\,dx
$$

## Transpose, Fractions, and Grouping

Parentheses in Python are preserved where they affect grouping. Division
renders as a fraction, and custom templates can supply notation such as
transpose.

$$
\operatorname{transposed}_{features} = \tilde{X}^T
$$

$$
\operatorname{normalized}_{distance} = \frac{\left\|y - v\right\|}{\sqrt{n}}
$$

## Equations

Assignments whose target starts with `eq_` are treated as named equations. The
rendered document shows the equation itself rather than the internal Python
variable name.

$$
\nabla_{\theta} \operatorname{regularized}_{loss} = 0
$$

$$
\sum_{i=1}^{n} p_{i} = 1
$$
