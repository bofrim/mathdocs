# pyright: reportUnboundVariable=false, reportOperatorIssue=false
from typing import Annotated

import numpy as np
from mathdocs import Symbol, Tensor, render_as

"""
# Linear model

The prediction is a matrix-vector product plus a bias term.
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
