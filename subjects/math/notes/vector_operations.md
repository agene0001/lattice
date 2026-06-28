# Vector Operations

**Intuition.** You combine vectors the obvious way — component by component — and those two moves, adding and scaling, are enough to build every point in the space.

## Definition

For vectors of equal length and a scalar $c$:

$$\mathbf{a} + \mathbf{b} = (a_1 + b_1, \dots, a_n + b_n), \qquad c\,\mathbf{a} = (c\,a_1, \dots, c\,a_n).$$

Addition is tip-to-tail; scalar multiplication stretches ($|c| > 1$), shrinks ($|c| < 1$), or flips ($c < 0$) the arrow without changing the line it lies on.

A **linear combination** mixes both: $c_1 \mathbf{a} + c_2 \mathbf{b}$. The set of all linear combinations of some vectors is their **span**.

## Worked example

With $\mathbf{a} = (1, 2)$, $\mathbf{b} = (3, 0)$, form $2\mathbf{a} - \mathbf{b}$:

$$2\mathbf{a} = (2, 4), \qquad 2\mathbf{a} - \mathbf{b} = (2 - 3,\; 4 - 0) = (-1, 4).$$

## Why it matters

A layer's output is a linear combination of its inputs weighted by learned coefficients — then bent by a nonlinearity. Gradient descent itself is one vector operation: $\boldsymbol\theta_{\text{new}} = \boldsymbol\theta - \eta\,\mathbf{g}$, a vector minus a scaled vector.

> **Common pitfall.** You can only add vectors of the **same dimension**, and you add matching components — $(1,2) + (3,0) = (4,2)$, never $(1,2) + 3$. A scalar scales every component; it doesn't attach to just one.
