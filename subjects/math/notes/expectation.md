# Expectation

**Intuition.** The expectation is the long-run average — the balance point of a distribution, where it would sit level if the probabilities were weights on a seesaw.

## Definition

For a discrete random variable $X$, the **expected value** is the probability-weighted sum of its values:

$$\mathbb{E}[X] = \sum_x x\, P(X = x).$$

(For continuous $X$, the sum becomes an integral, $\int x\, f(x)\,dx$.) A key property is **linearity**, which holds *always* — even when variables are dependent:

$$\mathbb{E}[aX + bY] = a\,\mathbb{E}[X] + b\,\mathbb{E}[Y].$$

## Worked example

Roll a fair die, $X \in \{1, \dots, 6\}$ each with probability $\tfrac16$:

$$\mathbb{E}[X] = \frac{1 + 2 + 3 + 4 + 5 + 6}{6} = \frac{21}{6} = 3.5.$$

The average roll is $3.5$ — a value $X$ never actually takes, which is fine: an expectation is a balance point, not a possible outcome.

## Why it matters

Loss functions are expectations ("average error over the data"), and training minimises an expected loss. Linearity is the workhorse that lets you break complicated averages into simple pieces.

> **Common pitfall.** The expectation need not be an achievable value (the die's $3.5$). And while $\mathbb{E}[X+Y] = \mathbb{E}[X] + \mathbb{E}[Y]$ always holds, $\mathbb{E}[XY] = \mathbb{E}[X]\,\mathbb{E}[Y]$ holds **only when $X$ and $Y$ are independent** — don't assume it in general.
