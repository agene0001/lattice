# Derivative of $e^x$

**Intuition.** The exponential $e^x$ is the one function that is *its own derivative* — its rate of change at any point equals its value there. That self-referential property is why it shows up everywhere growth or decay is proportional to the current amount.

## Definition

$$\frac{d}{dx} e^x = e^x.$$

With a constant multiplier inside, the chain rule brings it down:

$$\frac{d}{dx} e^{ax} = a\, e^{ax}.$$

## Worked example

Differentiate $e^{3x}$. The inner function is $u = 3x$, with $u' = 3$, so

$$\frac{d}{dx} e^{3x} = 3\, e^{3x}.$$

## Why it matters

The exponential is the backbone of machine learning's probability machinery: the **sigmoid** $\sigma(x) = \frac{1}{1 + e^{-x}}$ and the **softmax** are built from $e^x$, and their clean derivatives (which reuse this rule via the chain rule) are what make them trainable by gradient descent.

> **Common pitfall.** $\frac{d}{dx}e^{ax} = a\,e^{ax}$, not $e^{ax}$ and not $ax\,e^{ax}$. The chain rule multiplies by the derivative of the exponent, which is the constant $a$.
