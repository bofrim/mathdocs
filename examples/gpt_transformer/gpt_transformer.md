# GPT-Style Decoder Transformer

This example walks through the forward pass of a decoder-only transformer, the
architecture family used by GPT models. The Python source is not meant to run as
ordinary numerical code. Instead, MathDocs reads the annotations, template
decorators, Markdown blocks, and symbolic assignments to produce a technical
note with equations.

A GPT model receives a sequence of token ids, maps them into vectors, repeatedly
applies masked self-attention and feed-forward blocks, and finally predicts the
next-token distribution.

## Token and Position Embeddings

The input sequence is first converted into dense vectors. Token embeddings carry
lexical identity, while position embeddings give each timestep an address in the
sequence. GPT-style models combine these two sources with addition.

$$
X = E + P
$$

The result is a length-`T` matrix of hidden states. Each row is the model's
current representation of one token position.

## Query, Key, and Value Projections

Self-attention starts by projecting the hidden states into three views. Queries
ask what each position is looking for, keys advertise what each position
contains, and values carry the information that will be mixed into the next
representation.

$$
Q_{tk} = X_{td}{W_Q}_{dk}
$$

$$
K_{tk} = X_{td}{W_K}_{dk}
$$

$$
V_{tv} = X_{td}{W_V}_{dv}
$$

## Causal Attention Scores

The raw compatibility score compares every query against every key. Dividing by
the square root of the key dimension keeps the dot products numerically stable.
The causal mask `M` prevents a token from looking at future positions.

$$
S = \frac{\left(Q_{tk}K^T\right)}{\sqrt{d_k}} + M
$$

After masking, a softmax converts scores into attention weights. Each row is a
distribution over the positions that the current token is allowed to read.

$$
A = \operatorname{softmax}\left(S\right)
$$

## Weighted Value Mixing

Attention weights mix the value vectors. The output at each position is a
content-dependent weighted sum of previous value vectors, which is the mechanism
that lets a decoder condition next-token prediction on its context.

$$
H_{tv} = A_{tt}V_{tv}
$$

## Multi-Head Attention

In a full block, this attention calculation is performed in parallel for several
heads. Each head can specialize in a different relationship, such as local
syntax, long-distance references, or delimiter matching. The heads are
concatenated and projected back into the model dimension.

$$
H_{vd} = \operatorname{concat}\left(H_1, \ldots, H_h\right){W_O}_{vd}
$$

## Residual Path and Normalization

Modern GPT blocks preserve information through residual connections. Layer
normalization controls the scale of activations before they enter the next
sub-layer.

$$
R_1 = X + H
$$

$$
N_1 = \operatorname{LN}\left(R_1\right)
$$

## Position-Wise Feed-Forward Network

The feed-forward network is applied independently at every sequence position.
It expands the model dimension, applies a nonlinearity, and projects back down.
This gives each token representation a learned nonlinear transformation after
attention has mixed information across positions.

$$
G = \operatorname{GELU}\left({N_1}_{td}{W_1}_{dm} + {b_1}_{tm}\right)
$$

$$
F_{td} = G_{tm}{W_2}_{md} + {b_2}_{td}
$$

The second residual connection merges the feed-forward update back into the
stream. Stacking many copies of this block yields the deep transformer trunk.

$$
Y = N_1 + F
$$

## Vocabulary Projection

The final hidden states are projected into vocabulary logits. A softmax turns
those logits into a probability distribution for the next token at each
position.

$$
Z_{tv} = Y_{td}{W_U}_{dv} + {b_U}_{tv}
$$

$$
P = \operatorname{softmax}\left(Z\right)
$$

## Training Objective

During language-model training, the target at each position is the following
token from the original text. Cross entropy rewards the model for assigning high
probability to the observed next token.

$$
\mathcal{L} = \operatorname{CE}\left(P, y\right)
$$
