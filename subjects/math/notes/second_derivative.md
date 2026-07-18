# Second Derivative

**Intuition.** The first derivative tells you the slope; the **second derivative** tells you how the slope itself is changing — the *curvature*. Positive means the curve bends upward (like a valley), negative means it bends downward (like a hill).

## Definition

The second derivative is just the derivative of the derivative:

$$f''(x) = \frac{d}{dx}\left( f'(x) \right) = \frac{d^2 f}{dx^2}.$$

## Worked example

Let $f(x) = x^3 - 2x^2 + 5x - 1$. Differentiate once:

$$f'(x) = 3x^2 - 4x + 5.$$

Then differentiate again:

$$f''(x) = 6x - 4.$$

## Why it matters

Curvature is the whole game in optimization. The sign of $f''$ tells you whether a critical point is a minimum ($f'' > 0$) or a maximum ($f'' < 0$) — the **second-derivative test**. In many variables this generalizes to the **Hessian** matrix, which second-order optimizers (Newton's method) use to take smarter steps than plain gradient descent.

> **Common pitfall.** Don't stop after one derivative — the second derivative requires differentiating *again*. And a constant or linear term, which survives the first derivative, may vanish in the second.
