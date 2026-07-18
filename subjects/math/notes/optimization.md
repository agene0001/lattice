# Optimization

**Intuition.** Optimization is finding the input that makes a function as small (or as large) as possible — the lowest point of a valley. It's two steps: locate the flat spots, then check which ones are actually the bottom.

## The method

1. **Find critical points:** solve $f'(x) = 0$.
2. **Classify them with the second derivative:** if $f''(c) > 0$ the curve bends up, so $c$ is a **minimum**; if $f''(c) < 0$ it's a **maximum**.

For an upward parabola $f(x) = ax^2 + bx + c$ with $a > 0$, the minimum sits at $x = -\frac{b}{2a}$, and its value is

$$f\!\left(-\tfrac{b}{2a}\right) = \frac{4ac - b^2}{4a}.$$

## Worked example

Minimize $f(x) = x^2 - 4x + 7$. Here $a=1, b=-4, c=7$. The vertex is at $x = -\frac{-4}{2} = 2$, and the minimum value is

$$\frac{4(1)(7) - (-4)^2}{4(1)} = \frac{28 - 16}{4} = 3.$$

Since $f''(x) = 2 > 0$, it really is a minimum.

## Why it matters

This *is* machine learning. Training minimizes a loss function; the parameters that make the loss smallest are the trained model. In high dimensions you rarely solve $f'=0$ by hand — you follow the negative gradient downhill ([[gradient_descent]]) — but the logic is identical: flat gradient, curving upward.

> **Common pitfall.** A zero derivative alone doesn't prove you've found a minimum — always check the curvature. And on non-convex loss surfaces there can be *many* minima, not one.
