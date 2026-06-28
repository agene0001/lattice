# Exponent Rules

**Intuition.** An exponent is shorthand for repeated multiplication, and every "rule" is just what happens when you line those repeated factors up and count them.

## Definition

For any base $a$ and exponents $m, n$:

$$a^m \cdot a^n = a^{m+n}, \qquad \frac{a^m}{a^n} = a^{m-n}, \qquad (a^m)^n = a^{mn}.$$

Two consequences fall out of the quotient rule:

$$a^0 = 1 \quad (a \neq 0), \qquad a^{-n} = \frac{1}{a^n}.$$

Fractional exponents are roots: $a^{1/n} = \sqrt[n]{a}$, so $a^{m/n} = \sqrt[n]{a^m}$.

## Worked example

Simplify $\dfrac{x^5 \cdot x^{-2}}{x^{4}}$.

$$\frac{x^5 \cdot x^{-2}}{x^4} = \frac{x^{5 + (-2)}}{x^4} = \frac{x^3}{x^4} = x^{3-4} = x^{-1} = \frac{1}{x}.$$

Add the exponents on top, then subtract the bottom — the negative result becomes a reciprocal.

## Why it matters

The power rule for derivatives, polynomial features, and the way variances scale all lean on these identities. They also keep you honest with very large and very small numbers in scientific notation.

> **Common pitfall.** The rules only combine powers of the **same base**. $a^m \cdot b^n$ does **not** simplify, and $a^m + a^n$ is **not** $a^{m+n}$ — addition of powers has no shortcut. The exponent rules are about multiplying and dividing, never adding.
