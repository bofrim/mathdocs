# pyright: reportUnboundVariable=false
# mathdocs: off
from __future__ import annotations

from pathlib import Path

from mathdocs import render_figure

OUTPUT_PATH = Path(__file__).with_name("training_loss.svg")


def build_loss_plot(path: Path) -> None:
    losses = [1.00, 0.72, 0.53, 0.40, 0.31, 0.25, 0.21, 0.18]
    width = 640
    height = 360
    margin = 56
    plot_width = width - 2 * margin
    plot_height = height - 2 * margin
    max_loss = max(losses)
    min_loss = min(losses)

    def point(epoch: int, loss: float) -> tuple[float, float]:
        x = margin + (epoch / (len(losses) - 1)) * plot_width
        y = margin + ((max_loss - loss) / (max_loss - min_loss)) * plot_height
        return x, y

    points = [point(epoch, loss) for epoch, loss in enumerate(losses)]
    polyline = " ".join(f"{x:.1f},{y:.1f}" for x, y in points)
    circles = "\n".join(
        f'<circle cx="{x:.1f}" cy="{y:.1f}" r="5" fill="#2563eb" />' for x, y in points
    )

    path.write_text(
        f"""<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">
  <rect width="100%" height="100%" fill="#ffffff" />
  <line x1="{margin}" y1="{height - margin}" x2="{width - margin}" y2="{height - margin}" stroke="#111827" stroke-width="2" />
  <line x1="{margin}" y1="{margin}" x2="{margin}" y2="{height - margin}" stroke="#111827" stroke-width="2" />
  <text x="{width / 2}" y="28" text-anchor="middle" font-family="Arial, sans-serif" font-size="20" fill="#111827">Training loss</text>
  <text x="{width / 2}" y="{height - 16}" text-anchor="middle" font-family="Arial, sans-serif" font-size="14" fill="#374151">Epoch</text>
  <text x="18" y="{height / 2}" text-anchor="middle" transform="rotate(-90 18 {height / 2})" font-family="Arial, sans-serif" font-size="14" fill="#374151">Loss</text>
  <polyline points="{polyline}" fill="none" stroke="#2563eb" stroke-width="4" stroke-linecap="round" stroke-linejoin="round" />
  {circles}
</svg>
""",
        encoding="utf-8",
    )


build_loss_plot(OUTPUT_PATH)
# mathdocs: on

"""
# Generated Plot

This example writes a plot image from ordinary Python code, then uses a
MathDocs directive to place that generated artifact in the rendered document.
"""

render_figure(
    "training_loss.svg",
    alt="Training loss curve",
    caption="Loss decreases over eight training epochs.",
)


final_loss = 0.18

print(final_loss)
