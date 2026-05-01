#!/usr/bin/env bash
# Regenerate examples/<name>/<name>.md for every example.
#
# Run from the repo root:
#   bash scripts/render_examples.sh

set -euo pipefail

cd "$(dirname "$0")/.."

EXAMPLES=(
    linear_model
    electrodynamics
    feature_showcase
    gpt_transformer
    sidecar_demo
    generated_plot
)

# generated_plot writes its SVG via the Python interpreter; build it first so
# render_figure has a real artifact to point at.
uv run python -m mathdocs examples/generated_plot/generated_plot.py >/dev/null

for name in "${EXAMPLES[@]}"; do
    src="examples/${name}/${name}.py"
    out="examples/${name}/${name}.md"
    echo "rendering ${src} -> ${out}"
    cargo run --quiet -p mathdocs_cli -- render "${src}" >"${out}"
done
