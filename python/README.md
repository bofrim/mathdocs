# mathdocs

`mathdocs` renders ordinary Python source files as Markdown with inline LaTeX
math. The renderer reads annotations, decorators, stubs, and sidecar metadata
statically — it never imports or executes the target module. Installing the
package gives you both the helper API and the `mathdocs` CLI.

## Installation

```bash
pip install mathdocs
```

## CLI

```bash
mathdocs render examples/linear_model.py        # print rendered Markdown
mathdocs symbols examples/linear_model.py       # list discovered symbols
mathdocs check examples/linear_model.py         # report diagnostics
```

## Helper API

Import the helpers in ordinary Python code:

```python
from mathdocs import Symbol, Tensor, render_as, render_figure

theta = Symbol(r"\theta", text="angle")
stress = Tensor(r"\sigma", indices=("i", "j"))


@render_as(latex=r"x^2")
def square(x: float) -> float:
    return x * x


render_figure(
    "artifacts/training_loss.svg",
    alt="Training loss curve",
    caption="Loss decreases over eight training epochs.",
)
```

Top-level `render_figure(...)` calls are static placement directives. They tell
the renderer where to include an already generated plot, diagram, screenshot, or
other image in the final Markdown document. `render_image(...)` and
`render_plot(...)` remain available as aliases.

`python -m mathdocs <script.py>` runs a Python script with the script
directory added to `sys.path`, which is useful for examples that generate
local artifacts before the renderer reads the source:

```bash
python -m mathdocs examples/generated_plot.py
```

The MathDocs project (including the VS Code extension) lives at
<https://github.com/bofrim/mathdocs>.
