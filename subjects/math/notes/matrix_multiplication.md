# Matrix Multiplication

**Intuition.** Multiplying two matrices composes two transformations into one — and mechanically, every entry of the result is a dot product of a row from the left with a column from the right.

## Definition

For $A$ of shape $m \times n$ and $B$ of shape $n \times p$, the product $AB$ has shape $m \times p$, with

$$(AB)_{ij} = \sum_{k=1}^{n} a_{ik}\, b_{kj}.$$

The inner dimensions must agree (the $n$'s), and they vanish in the result. Conceptually, applying $AB$ to a vector is the same as applying $B$ first, then $A$ — composition of linear maps.

## Worked example

$$\begin{bmatrix} 1 & 2 \\ 3 & 4 \end{bmatrix} \begin{bmatrix} 0 & 1 \\ 1 & 0 \end{bmatrix} = \begin{bmatrix} 1\cdot 0 + 2\cdot 1 & 1\cdot 1 + 2\cdot 0 \\ 3\cdot 0 + 4\cdot 1 & 3\cdot 1 + 4\cdot 0 \end{bmatrix} = \begin{bmatrix} 2 & 1 \\ 4 & 3 \end{bmatrix}.$$

The right matrix swaps columns — composition in action.

## Why it matters

A deep network is a chain of matrix multiplications (with nonlinearities between). Stacking layers is literally multiplying their weight matrices, and the cost of training is dominated by these products.

> **Common pitfall.** Matrix multiplication is **not commutative**: $AB \neq BA$ in general (often one product isn't even defined). The order encodes which transformation happens first — you can't swap the factors.
