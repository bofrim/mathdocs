from __future__ import annotations

import runpy
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
