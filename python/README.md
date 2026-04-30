# mathdocs

`mathdocs` renders ordinary Python source files as Markdown with inline LaTeX
math. The renderer reads annotations, decorators, and docstrings statically — it
never imports or executes the target module — so a `.py` file *is* the document.

## Installation

```bash
pip install mathdocs
```

## A small example

A complete, runnable Python file:

```python
from typing import Annotated

import numpy as np
from mathdocs import Symbol, Tensor, render_as

"""
# Linear model

The prediction is a matrix-vector product plus a bias term, and the loss is
the residual norm scaled by the noise standard deviation.
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

Render it:

```bash
mathdocs render linear_model.py
```

Output:

````markdown
# Linear model

The prediction is a matrix-vector product plus a bias term, and the loss is
the residual norm scaled by the noise standard deviation.

$$
\operatorname{loss} = \frac{\left\|y_{i} - \left(A_{ij}x_{j} + b_{i}\right)\right\|}{\sigma}
$$
````

The Python module still type-checks and runs unchanged. The annotations and
`render_as` decorator are *metadata* that the renderer reads from the source —
nothing executes when MathDocs builds the document.

## What the helpers do

- **`Symbol(latex)`** — annotate a scalar variable with how it should appear in
  math (e.g. `sigma: Annotated[float, Symbol(r"\sigma")]` renders as $\sigma$).
- **`Tensor(name, indices)`** — annotate an array with its tensor name and
  index labels; the renderer attaches the indices automatically when the
  variable appears in an expression.
- **`@render_as(latex="...")`** — give a function a LaTeX template. `{0}`,
  `{1}` etc. are filled with rendered argument expressions.
- **`render_figure(path, caption=...)`** — drop a pre-generated image into the
  output at a specific point. Useful when a script produces a plot beside the
  source it documents.

## CLI

```bash
mathdocs render path/to/file.py        # print rendered Markdown
mathdocs symbols path/to/file.py       # list discovered symbols
mathdocs check path/to/file.py         # report diagnostics
```

`python -m mathdocs <script.py>` runs a Python script with the script
directory added to `sys.path` — useful when a script needs to generate
artifacts (plots, tables) before the renderer reads it.

The MathDocs project (including the VS Code extension) lives at
<https://github.com/bofrim/mathdocs>.
