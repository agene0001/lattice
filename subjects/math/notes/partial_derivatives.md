# Partial Derivatives

**Intuition.** When a function depends on several variables, a partial derivative measures its sensitivity to *one* of them while the others are pinned in place — change one knob, watch the output, ignore the rest.

## Definition

For $f(x, y)$, the partial derivative with respect to $x$ treats $y$ as a constant:

$$\frac{\partial f}{\partial x} = \lim_{h \to 0} \frac{f(x + h,\, y) - f(x,\, y)}{h}.$$

The curly $\partial$ distinguishes it from the ordinary $d$. Mechanically, you differentiate as usual in $x$ and freeze every other variable as if it were a number.

## Worked example

Let $f(x, y) = x^2 y + 3y^2$.

$$\frac{\partial f}{\partial x} = 2xy \quad (\text{treat } y \text{ as constant}),$$
$$\frac{\partial f}{\partial y} = x^2 + 6y \quad (\text{treat } x \text{ as constant}).$$

In the first, $3y^2$ is a constant (zero derivative); in the second, $x^2 y$ has constant coefficient $x^2$.

## Why it matters

A loss depends on millions of parameters at once. The partial derivative with respect to each weight says how that single weight affects the loss — and collecting them all gives the gradient that training follows.

> **Common pitfall.** "Hold the others constant" means *every* other variable, including products and cross-terms. In $x^2 y$, the partial in $x$ is $2xy$ — the $y$ stays as a multiplier, it does not disappear.
