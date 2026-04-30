# pyright: reportUnboundVariable=false, reportOperatorIssue=false
from typing import Annotated

from mathrender import Symbol, Tensor, render_as

mu: Annotated[object, Symbol(r"\mu")]
nu: Annotated[object, Symbol(r"\nu")]
rho: Annotated[object, Symbol(r"\rho")]

A: Annotated[object, Tensor("A", (r"\mu",))]
F: Annotated[object, Tensor("F", (r"\mu", r"\nu"))]
J: Annotated[object, Tensor("J", (r"\mu",))]
alpha: Annotated[object, Symbol(r"\alpha")]
psi: Annotated[object, Symbol(r"\psi")]
e: Annotated[float, Symbol("e")]
i: Annotated[float, Symbol("i")]


@render_as(latex=r"d{0}")
def d(x): ...


@render_as(latex=r"\star {0}")
def star(x): ...


@render_as(latex=r"\partial_{0} {1}")
def partial(index, expr): ...


@render_as(latex=r"e^{{{0}}}")
def exp(x): ...


"""
# Electrodynamics as a U(1) Gauge Theory

The gauge potential is a one-form. Its curvature is the electromagnetic field
strength.
"""

F = d(A)

F_mu_nu = partial(mu, A[nu]) - partial(nu, A[mu])

"""
## Gauge transformations
"""

A_prime = A + d(alpha)
psi_prime = exp(i * e * alpha) * psi

"""
## Maxwell equations
"""

eq_bianchi = d(F) == 0
eq_source = d(star(F)) == star(J)
