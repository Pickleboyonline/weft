"""python -m weft build <file.py> [-o DIR]

Imports the file (which traces every handler), then writes each weft.App it
defines to DIR/<app name>.weft.json.
"""

from __future__ import annotations

import argparse
import importlib.util
import sys
import traceback
from pathlib import Path

from .core import App, TracingError


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(prog="weft")
    sub = ap.add_subparsers(dest="cmd", required=True)
    b = sub.add_parser("build", help="trace a Python program into Weft IR")
    b.add_argument("file", type=Path)
    b.add_argument("-o", "--out", type=Path, default=Path("build"))
    args = ap.parse_args(argv)

    spec = importlib.util.spec_from_file_location(args.file.stem, args.file)
    if spec is None or spec.loader is None:
        print(f"weft: can't import {args.file}", file=sys.stderr)
        return 2
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    try:
        spec.loader.exec_module(module)
    except TracingError as e:
        # Point at the user's line, not at weft internals.
        here = args.file.resolve()
        frames = [f for f in traceback.extract_tb(e.__traceback__) if Path(f.filename).resolve() == here]
        where = f"{frames[-1].filename}:{frames[-1].lineno}: " if frames else ""
        print(f"{where}weft: {e}", file=sys.stderr)
        return 1

    apps = [v for v in vars(module).values() if isinstance(v, App)]
    if not apps:
        print(f"weft: {args.file} defines no weft.App", file=sys.stderr)
        return 1
    for app in apps:
        print(f"wrote {app.save(args.out / f'{app.name}.weft.json')}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
