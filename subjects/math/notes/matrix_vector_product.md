# Matrix-Vector Product

**Intuition.** Multiplying a matrix by a vector is just a stack of dot products: each row of the matrix meets the vector and produces one number of the output.

## Definition

For an $m \times n$ matrix $A$ and a vector $\mathbf{x}$ of length $n$, the product $A\mathbf{x}$ is a vector of length $m$ whose $i$-th entry is the dot product of row $i$ with $\mathbf{x}$:

$$(A\mathbf{x})_i = \sum_{j=1}^{n} a_{ij}\, x_j.$$

The inner dimensions must match: $A$'s columns ($n$) equal $\mathbf{x}$'s length ($n$), and the output inherits $A$'s row count $m$. A second reading: $A\mathbf{x}$ is a **linear combination of $A$'s columns**, weighted by the entries of $\mathbf{x}$.

## Worked example

$$\begin{bmatrix} 1 & 2 \\ 3 & 4 \end{bmatrix} \begin{bmatrix} 5 \\ 6 \end{bmatrix} = \begin{bmatrix} 1\cdot 5 + 2\cdot 6 \\ 3\cdot 5 + 4\cdot 6 \end{bmatrix} = \begin{bmatrix} 17 \\ 39 \end{bmatrix}.$$

Row 1 dotted with the vector gives $17$; row 2 gives $39$.

## Why it matters

A forward pass through a dense layer is exactly $A\mathbf{x} + \mathbf{b}$ — a matrix applied to the inputs, plus a bias. Every prediction a linear model makes is one matrix-vector product.

> **Common pitfall.** Shapes must line up: an $m \times n$ matrix only multiplies a length-$n$ vector, and the result has length $m$ (not $n$). If the inner dimensions don't match, the product simply doesn't exist.
