# Derivative of $\ln x$

**Intuition.** The natural logarithm grows ever more slowly, and its rate of change is exactly $1/x$ — steep near zero, nearly flat for large $x$. It's the inverse of $e^x$, and its derivative is strikingly simple.

## Definition

$$\frac{d}{dx} \ln x = \frac{1}{x}.$$

A constant multiplier inside doesn't change the answer, because $\ln(ax) = \ln a + \ln x$ and the constant $\ln a$ differentiates to zero:

$$\frac{d}{dx} \ln(ax) = \frac{1}{x}.$$

## Worked example

Differentiate $\ln(5x)$. Split it: $\ln(5x) = \ln 5 + \ln x$. The $\ln 5$ is a constant, so

$$\frac{d}{dx} \ln(5x) = 0 + \frac{1}{x} = \frac{1}{x}.$$

## Why it matters

Logs turn products into sums, which is why machine learning optimizes the **log-likelihood** rather than the likelihood itself. The gradient of a log-loss (cross-entropy) leans on this derivative — the $1/x$ is what flows backward through training.

> **Common pitfall.** The coefficient inside the log does **not** appear in the derivative: $\frac{d}{dx}\ln(ax) = \frac{a}{ax} = \frac{1}{x}$. It's tempting to write $\frac{a}{x}$ — but the $a$ cancels.
