# Power Rule

**Intuition.** For a power of $x$, differentiating is mechanical: bring the exponent down as a multiplier, then drop the exponent by one. It turns the limit definition into a one-line operation.

## Definition

For any constant exponent $n$,

$$\frac{d}{dx}\, x^n = n\,x^{n-1}.$$

Combined with two facts — the derivative of a constant is $0$, and derivatives are **linear** ($\frac{d}{dx}[a\,f + b\,g] = a f' + b g'$) — this differentiates any polynomial term by term. A constant multiple rides along: $\frac{d}{dx}(a x^n) = a n x^{n-1}$.

## Worked example

Differentiate $f(x) = 3x^4 - 5x^2 + 7$.

$$f'(x) = 3 \cdot 4x^{3} - 5 \cdot 2x^{1} + 0 = 12x^3 - 10x.$$

Each term: multiply by its exponent, reduce the exponent, and the lone constant $7$ vanishes.

## Why it matters

It's the workhorse you'll reach for thousands of times — and because $n$ can be negative or fractional, it also covers $1/x = x^{-1}$ and $\sqrt{x} = x^{1/2}$ without any new rule.

> **Common pitfall.** The power rule is for a **variable raised to a constant** ($x^n$). It does **not** apply to a constant raised to a variable: $\frac{d}{dx}\,2^x \neq x\,2^{x-1}$. That's an exponential, with its own rule ($2^x \ln 2$).
