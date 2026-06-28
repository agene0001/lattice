# Algebraic Manipulation

**Intuition.** An equation is a balance scale: whatever you do to one side you must do to the other, and "solving" just means rearranging until the unknown sits alone.

## Definition

To **isolate a variable**, undo the operations attached to it in reverse order, applying the same step to both sides. The legal moves all preserve equality:

- add or subtract the same quantity from both sides,
- multiply or divide both sides by the same **non-zero** quantity,
- distribute: $a(b + c) = ab + ac$,
- combine like terms: $3x + 2x = 5x$.

## Worked example

Solve $2(x + 1) = 3x - 4$ for $x$.

$$2x + 2 = 3x - 4 \quad\text{(distribute)}$$
$$2 + 4 = 3x - 2x \quad\text{(collect terms)}$$
$$6 = x.$$

Check by substituting back: $2(6+1) = 14$ and $3(6) - 4 = 14$. ✓ Substituting back is how you catch your own slips.

## Why it matters

Every closed-form result you'll derive — the slope of a line, a least-squares solution, a maximum-likelihood estimate — is reached by exactly these moves. Fluency here is what lets you focus on the idea instead of the bookkeeping.

> **Common pitfall.** When you multiply or divide both sides by a quantity, it must be non-zero, and you must apply it to *every* term — including the ones you're tempted to ignore. Dividing $2x + 4 = 6$ by $2$ gives $x + 2 = 3$, not $x + 4 = 3$.
