from __future__ import annotations

import runpy
import subprocess
import sys
from pathlib import Path


def main() -> None:
    if len(sys.argv) < 2:
        print("usage: mdpy path/to/script.py [args...]", file=sys.stderr)
        raise SystemExit(2)

    script = Path(sys.argv[1])
    sys.argv = [str(script), *sys.argv[2:]]
    sys.path.insert(0, str(script.parent.resolve()))
    runpy.run_path(str(script), run_name="__main__")


def render_main() -> None:
    repo_root = next(
        parent for parent in Path(__file__).resolve().parents if (parent / "Cargo.toml").exists()
    )
    command = [
        "cargo",
        "run",
        "--quiet",
        "--manifest-path",
        str(repo_root / "Cargo.toml"),
        "-p",
        "mathdocs_cli",
        "--",
        "render",
        *sys.argv[1:],
    ]
    raise SystemExit(subprocess.run(command).returncode)
