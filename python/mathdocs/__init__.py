from __future__ import annotations

from dataclasses import dataclass
from importlib.metadata import PackageNotFoundError, version
from typing import Any, Callable

try:
    __version__ = version("mathdocs")
except PackageNotFoundError:
    __version__ = "0.0.0+unknown"

__all__ = [
    "Image",
    "RenderTemplate",
    "Symbol",
    "Tensor",
    "__version__",
    "render_as",
    "render_figure",
    "render_image",
    "render_plot",
]


@dataclass(frozen=True)
class Symbol:
    latex: str
    text: str | None = None


@dataclass(frozen=True)
class Tensor:
    latex: str
    indices: tuple[str, ...] = ()
    variance: tuple[str, ...] | None = None
    text: str | None = None


@dataclass(frozen=True)
class RenderTemplate:
    latex: str
    text: str | None = None
    precedence: int | None = None


@dataclass(frozen=True)
class Image:
    src: str
    alt: str = ""
    title: str | None = None
    caption: str | None = None


def render_as(**formats: str) -> Callable[[Callable[..., Any]], Callable[..., Any]]:
    # Kwargs are intentionally discarded at runtime; the renderer reads them
    # statically from the source AST.
    def deco(fn: Callable[..., Any]) -> Callable[..., Any]:
        return fn

    return deco


def render_figure(
    src: str,
    *,
    alt: str = "",
    title: str | None = None,
    caption: str | None = None,
) -> Image:
    return Image(src=src, alt=alt, title=title, caption=caption)


def render_image(
    src: str,
    *,
    alt: str = "",
    title: str | None = None,
    caption: str | None = None,
) -> Image:
    return render_figure(src, alt=alt, title=title, caption=caption)


def render_plot(
    src: str,
    *,
    alt: str = "",
    title: str | None = None,
    caption: str | None = None,
) -> Image:
    return render_figure(src, alt=alt, title=title, caption=caption)
