# Variance

**Intuition.** Variance measures spread: how far, on average, the outcomes stray from the mean. Small variance means tightly clustered; large variance means all over the place.

## Definition

The **variance** is the expected squared deviation from the mean $\mu = \mathbb{E}[X]$:

$$\operatorname{Var}(X) = \mathbb{E}\big[(X - \mu)^2\big].$$

Squaring keeps deviations positive (so they don't cancel) and punishes large misses more. A handy equivalent form:

$$\operatorname{Var}(X) = \mathbb{E}[X^2] - (\mathbb{E}[X])^2.$$

The **standard deviation** $\sigma = \sqrt{\operatorname{Var}(X)}$ returns the spread to the original units.

## Worked example

Let $X$ be $0$ or $4$, each with probability $\tfrac12$. Then $\mu = 2$, and

$$\operatorname{Var}(X) = \tfrac12 (0 - 2)^2 + \tfrac12 (4 - 2)^2 = \tfrac12(4) + \tfrac12(4) = 4,$$

so $\sigma = 2$. The values sit exactly $2$ away from the mean — matching $\sigma$.

## Why it matters

Variance quantifies uncertainty in estimates, drives the bias–variance tradeoff, and explains why weight initialisation and feature scaling matter — uncontrolled variance makes training unstable.

> **Common pitfall.** Variance is in **squared units**, so it's not directly comparable to the data — take the square root (the standard deviation) for that. Also, $\operatorname{Var}(X) = \mathbb{E}[X^2] - (\mathbb{E}[X])^2$, **not** $\mathbb{E}[X^2] - \mathbb{E}[X]$; the mean is squared.
