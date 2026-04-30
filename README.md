# MathRender

MathRender statically renders ordinary Python source as Markdown plus LaTeX.
The Python program remains the source of truth; the renderer reads annotations,
decorators, stubs, and sidecar metadata without importing or executing target
modules.

## Quick Start

```bash
uv run render examples/linear_model.py
uv run render examples/gpt_transformer.py
uv run render examples/feature_showcase.py
uv run mrpy examples/generated_plot.py
uv run render examples/generated_plot.py
cargo run -p mathrender_cli -- symbols examples/linear_model.py
cargo run -p mathrender_lsp
```

The tiny Python package lives in `python/mathrender` and provides `Symbol`,
`Tensor`, `RenderTemplate`, `Image`, render placement helpers, and the identity
`render_as` decorator.

Use `render_figure` as a top-level directive to include an already generated
plot, diagram, screenshot, or other image at an exact point in the rendered
document:

```python
from mathrender import render_figure

"""
# Training run
"""

render_figure("artifacts/loss.png", alt="Loss curve", caption="Training loss")
```

Local relative figure paths are resolved relative to the Python source file and
then emitted relative to the current working directory, so the rendered Markdown
stays portable for the directory where the render command was invoked.
