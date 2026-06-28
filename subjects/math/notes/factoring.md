# Factoring

**Intuition.** Factoring runs multiplication backwards: instead of expanding a product into a sum, you recognise a sum as a disguised product — which is exactly what you need to find where an expression equals zero.

## Definition

To **factor** is to write an expression as a product of simpler factors. The everyday patterns:

- **Common factor:** $ab + ac = a(b + c)$.
- **Difference of squares:** $a^2 - b^2 = (a - b)(a + b)$.
- **Quadratic trinomial:** $x^2 + (p+q)x + pq = (x + p)(x + q)$ — find two numbers that multiply to the constant and add to the middle coefficient.

The payoff is the **zero-product property**: if $(x - r)(x - s) = 0$, then $x = r$ or $x = s$.

## Worked example

Solve $x^2 - 5x + 6 = 0$.

We need two numbers multiplying to $6$ and adding to $-5$: those are $-2$ and $-3$.

$$x^2 - 5x + 6 = (x - 2)(x - 3) = 0 \implies x = 2 \text{ or } x = 3.$$

## Why it matters

Roots of polynomials are where models change behaviour, where a derivative is zero (candidate optima), and where denominators blow up. Factoring is the fastest hand route to them.

> **Common pitfall.** Difference of squares factors; a **sum** of squares $a^2 + b^2$ does **not** factor over the real numbers. And always pull out a common factor first — $2x^2 - 8 = 2(x^2 - 4) = 2(x-2)(x+2)$, not $(x-2)(x+2)$ alone.
