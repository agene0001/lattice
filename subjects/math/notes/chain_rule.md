# Chain Rule

**Intuition.** To differentiate a function inside a function, multiply the rates: how fast the outer responds to its input, times how fast the inner responds to $x$. Rates of change compose by multiplying.

## Definition

If $y = f(g(x))$, then

$$\frac{dy}{dx} = f'(g(x)) \cdot g'(x).$$

In words: derivative of the outer (evaluated at the inner, left untouched) **times** the derivative of the inner. With the Leibniz notation it reads as a satisfying cancellation:

$$\frac{dy}{dx} = \frac{dy}{du}\cdot\frac{du}{dx}, \qquad u = g(x).$$

## Worked example

Differentiate $y = (3x^2 + 1)^5$. Outer is $u^5$, inner is $u = 3x^2 + 1$.

$$\frac{dy}{dx} = 5(3x^2 + 1)^{4} \cdot \underbrace{6x}_{g'(x)} = 30x\,(3x^2 + 1)^4.$$

Differentiate the outer power, keep the inside intact, then multiply by the inside's derivative.

## Why it matters

This is the engine of **backpropagation**. A network is nested functions, and the chain rule propagates the loss's gradient backward through every layer by multiplying local derivatives. Backprop is the chain rule applied at scale.

> **Common pitfall.** Don't forget the $\cdot\, g'(x)$ factor. Writing $\frac{d}{dx}(3x^2+1)^5 = 5(3x^2+1)^4$ — and stopping — is the classic mistake; the inner derivative $6x$ is what makes it the chain rule rather than the power rule.
