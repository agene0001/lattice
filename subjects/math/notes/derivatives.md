# Derivatives

**Intuition.** The derivative is the instantaneous rate of change — how fast the output moves when you nudge the input, and equivalently the slope of the curve's tangent line at that point.

## Definition

The derivative of $f$ at $x$ is the limit of the average rate of change as the interval shrinks to zero:

$$f'(x) = \lim_{h \to 0} \frac{f(x + h) - f(x)}{h}.$$

When this limit exists, $f$ is **differentiable** at $x$. Notation varies: $f'(x)$, $\dfrac{df}{dx}$, $\dfrac{dy}{dx}$ all mean the same thing. A positive derivative means rising; negative, falling; zero, momentarily flat.

## Worked example

Differentiate $f(x) = x^2$ from the definition:

$$f'(x) = \lim_{h \to 0} \frac{(x+h)^2 - x^2}{h} = \lim_{h \to 0} \frac{2xh + h^2}{h} = \lim_{h \to 0}(2x + h) = 2x.$$

So the slope of $y = x^2$ at $x = 3$ is $6$.

## Why it matters

Training a model means minimising a loss, and the derivative tells you which way the loss is going and how steeply. No derivatives, no gradient descent — the optimiser would be blind.

> **Common pitfall.** The derivative is a **function**, not a single number: $f'(x) = 2x$ gives a different slope at every point. Evaluate it at a specific $x$ to get the slope *there*; don't treat $f'$ as one fixed value.
