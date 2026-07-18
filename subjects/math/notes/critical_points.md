# Critical Points

**Intuition.** At the very top of a hill or the bottom of a valley, the ground is momentarily flat — the slope is zero. A **critical point** is any input where the derivative is zero (or undefined): the places where a function *might* reach a maximum or minimum.

## Definition

$x = c$ is a critical point of $f$ when

$$f'(c) = 0.$$

For a quadratic $f(x) = ax^2 + bx + c$, setting $f'(x) = 2ax + b = 0$ gives the single critical point

$$x = -\frac{b}{2a}.$$

## Worked example

Find the critical point of $f(x) = 2x^2 - 8x + 3$. Differentiate and set to zero:

$$f'(x) = 4x - 8 = 0 \implies x = 2.$$

## Why it matters

Training a model *is* the search for a critical point of the loss: gradient descent walks downhill until the gradient is zero. In many dimensions the condition becomes $\nabla f = \mathbf{0}$ — every partial derivative vanishing at once. Finding those points is the entire objective.

> **Common pitfall.** $f'(c) = 0$ makes $c$ a *candidate*, not automatically a minimum — it could be a maximum or a saddle. Classifying it needs the [[second_derivative]] (the next step, [[optimization]]).
