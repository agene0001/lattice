# Random Variables

**Intuition.** A random variable is a function that pins a number onto every outcome, so you can do arithmetic with chance instead of juggling word-descriptions of events.

## Definition

A **random variable** $X$ maps each outcome in the sample space to a real number. Two flavours:

- **Discrete:** countable values, described by a **probability mass function** $P(X = x)$, with all the masses summing to $1$.
- **Continuous:** values on a range, described by a **probability density function** $f(x)$, where probabilities are *areas* under the curve and $P(X = x) = 0$ for any single point.

## Worked example

Flip two fair coins; let $X$ be the number of heads. The outcomes $\{TT, HT, TH, HH\}$ map to:

$$P(X=0) = \tfrac14, \quad P(X=1) = \tfrac24, \quad P(X=2) = \tfrac14.$$

The masses sum to $1$, as they must. $X$ has turned "how many heads?" into a numeric distribution.

## Why it matters

Data, model outputs, and noise are all modelled as random variables. Treating an uncertain quantity as a numeric object is what lets us define its mean, its spread, and the likelihood of the data we actually saw.

> **Common pitfall.** For a **continuous** variable, the density $f(x)$ is not a probability and can exceed $1$ — only the area over an interval is a probability, and $P(X = x) = 0$ at any exact point. Don't read $f(x)$ as "the chance of $x$."
