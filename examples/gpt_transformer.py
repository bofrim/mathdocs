# pyright: reportUnboundVariable=false, reportOperatorIssue=false
from typing import Annotated

from mathrender import Symbol, Tensor, render_as

"""
# GPT-Style Decoder Transformer

This example walks through the forward pass of a decoder-only transformer, the
architecture family used by GPT models. The Python source is not meant to run as
ordinary numerical code. Instead, MathRender reads the annotations, template
decorators, Markdown blocks, and symbolic assignments to produce a technical
note with equations.

A GPT model receives a sequence of token ids, maps them into vectors, repeatedly
applies masked self-attention and feed-forward blocks, and finally predicts the
next-token distribution.
"""

T: Annotated[int, Symbol("T")]
d_model: Annotated[int, Symbol(r"d_{\mathrm{model}}")]
d_k: Annotated[int, Symbol(r"d_k")]
n_heads: Annotated[int, Symbol("h")]

tokens: Annotated[object, Tensor("x", ("t",))]
positions: Annotated[object, Tensor("p", ("t",))]
E_token: Annotated[object, Tensor("E", ("t", "d"))]
E_position: Annotated[object, Tensor("P", ("t", "d"))]
X: Annotated[object, Tensor("X", ("t", "d"))]

W_Q: Annotated[object, Tensor("W_Q", ("d", "k"))]
W_K: Annotated[object, Tensor("W_K", ("d", "k"))]
W_V: Annotated[object, Tensor("W_V", ("d", "v"))]
W_O: Annotated[object, Tensor("W_O", ("v", "d"))]

Q: Annotated[object, Tensor("Q", ("t", "k"))]
K: Annotated[object, Tensor("K", ("t", "k"))]
V: Annotated[object, Tensor("V", ("t", "v"))]
S: Annotated[object, Tensor("S", ("t", "t"))]
M: Annotated[object, Tensor("M", ("t", "t"))]
A_attn: Annotated[object, Tensor("A", ("t", "t"))]
H: Annotated[object, Tensor("H", ("t", "d"))]

head_1: Annotated[object, Tensor("H_1", ("t", "v"))]
head_n: Annotated[object, Tensor("H_h", ("t", "v"))]

R_1: Annotated[object, Tensor("R_1", ("t", "d"))]
N_1: Annotated[object, Tensor("N_1", ("t", "d"))]
F_hidden: Annotated[object, Tensor("G", ("t", "m"))]
F_out: Annotated[object, Tensor("F", ("t", "d"))]
Y: Annotated[object, Tensor("Y", ("t", "d"))]

W_1: Annotated[object, Tensor("W_1", ("d", "m"))]
b_1: Annotated[object, Tensor("b_1", ("m",))]
W_2: Annotated[object, Tensor("W_2", ("m", "d"))]
b_2: Annotated[object, Tensor("b_2", ("d",))]

W_U: Annotated[object, Tensor("W_U", ("d", "v"))]
b_U: Annotated[object, Tensor("b_U", ("v",))]
Z: Annotated[object, Tensor("Z", ("t", "v"))]
P_next: Annotated[object, Tensor("P", ("t", "v"))]
target: Annotated[object, Tensor("y", ("t",))]
loss: Annotated[float, Symbol(r"\mathcal{L}")]


@render_as(latex=r"\sqrt{{{0}}}")
def sqrt(x): ...


@render_as(latex=r"{0}^T")
def transpose(x): ...


@render_as(latex=r"\operatorname{{softmax}}\left({0}\right)")
def softmax(x): ...


@render_as(latex=r"\operatorname{{LN}}\left({0}\right)")
def layer_norm(x): ...


@render_as(latex=r"\operatorname{{GELU}}\left({0}\right)")
def gelu(x): ...


@render_as(latex=r"\operatorname{{concat}}\left({0}, \ldots, {1}\right)")
def concat(first, last): ...


@render_as(latex=r"\operatorname{{CE}}\left({0}, {1}\right)")
def cross_entropy(prediction, expected): ...


"""
## Token and Position Embeddings

The input sequence is first converted into dense vectors. Token embeddings carry
lexical identity, while position embeddings give each timestep an address in the
sequence. GPT-style models combine these two sources with addition.
"""

X = E_token + E_position

"""
The result is a length-`T` matrix of hidden states. Each row is the model's
current representation of one token position.
"""

"""
## Query, Key, and Value Projections

Self-attention starts by projecting the hidden states into three views. Queries
ask what each position is looking for, keys advertise what each position
contains, and values carry the information that will be mixed into the next
representation.
"""

Q = X @ W_Q
K = X @ W_K
V = X @ W_V

"""
## Causal Attention Scores

The raw compatibility score compares every query against every key. Dividing by
the square root of the key dimension keeps the dot products numerically stable.
The causal mask `M` prevents a token from looking at future positions.
"""

S = (Q @ transpose(K)) / sqrt(d_k) + M

"""
After masking, a softmax converts scores into attention weights. Each row is a
distribution over the positions that the current token is allowed to read.
"""

A_attn = softmax(S)

"""
## Weighted Value Mixing

Attention weights mix the value vectors. The output at each position is a
content-dependent weighted sum of previous value vectors, which is the mechanism
that lets a decoder condition next-token prediction on its context.
"""

H = A_attn @ V

"""
## Multi-Head Attention

In a full block, this attention calculation is performed in parallel for several
heads. Each head can specialize in a different relationship, such as local
syntax, long-distance references, or delimiter matching. The heads are
concatenated and projected back into the model dimension.
"""

H = concat(head_1, head_n) @ W_O

"""
## Residual Path and Normalization

Modern GPT blocks preserve information through residual connections. Layer
normalization controls the scale of activations before they enter the next
sub-layer.
"""

R_1 = X + H
N_1 = layer_norm(R_1)

"""
## Position-Wise Feed-Forward Network

The feed-forward network is applied independently at every sequence position.
It expands the model dimension, applies a nonlinearity, and projects back down.
This gives each token representation a learned nonlinear transformation after
attention has mixed information across positions.
"""

F_hidden = gelu(N_1 @ W_1 + b_1)
F_out = F_hidden @ W_2 + b_2

"""
The second residual connection merges the feed-forward update back into the
stream. Stacking many copies of this block yields the deep transformer trunk.
"""

Y = N_1 + F_out

"""
## Vocabulary Projection

The final hidden states are projected into vocabulary logits. A softmax turns
those logits into a probability distribution for the next token at each
position.
"""

Z = Y @ W_U + b_U
P_next = softmax(Z)

"""
## Training Objective

During language-model training, the target at each position is the following
token from the original text. Cross entropy rewards the model for assigning high
probability to the observed next token.
"""

loss = cross_entropy(P_next, target)
