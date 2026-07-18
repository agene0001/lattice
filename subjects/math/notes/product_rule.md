# Product Rule

**Intuition.** The derivative of a product is *not* the product of the derivatives. When two changing quantities are multiplied, each one contributes a change while the other is held at its current value — so the total rate of change is the sum of those two contributions.

## Definition

For two differentiable functions $f$ and $g$,

$$(fg)' = f'g + fg'.$$

Read it as: "derivative of the first times the second, plus the first times the derivative of the second."

## Worked example

Differentiate $(2x + 1)(3x - 4)$. Let $f = 2x+1$ (so $f' = 2$) and $g = 3x - 4$ (so $g' = 3$):

$$(fg)' = f'g + fg' = 2(3x - 4) + (2x + 1)(3) = 6x - 8 + 6x + 3 = 12x - 5.$$

(You can check by expanding first: $(2x+1)(3x-4) = 6x^2 - 5x - 4$, whose derivative is $12x - 5$. ✓)

## Why it matters

Products of functions are everywhere in machine learning — a weight times an activation, a probability times a value in an expectation. When you can't expand first (e.g. $x^2 \ln x$), the product rule is the only way through, and it's a building block of backpropagation.

> **Common pitfall.** $(fg)' \neq f'g'$. Forgetting the *second* term — the one where you differentiate $g$ and keep $f$ — is the classic error.
