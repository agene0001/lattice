# Bayes' Theorem

**Intuition.** Bayes' theorem flips a conditional you don't know into one you do — turning $P(\text{evidence} \mid \text{cause})$ into $P(\text{cause} \mid \text{evidence})$, which is usually what you actually want.

## Definition

$$P(A \mid B) = \frac{P(B \mid A)\,P(A)}{P(B)}.$$

The pieces have names: $P(A)$ is the **prior** (belief before evidence), $P(B \mid A)$ the **likelihood**, $P(A \mid B)$ the **posterior** (belief after). The denominator expands by the law of total probability:

$$P(B) = P(B \mid A)\,P(A) + P(B \mid \neg A)\,P(\neg A).$$

## Worked example

A test is 99% accurate ($P(+\mid D) = 0.99$, $P(+\mid \neg D) = 0.01$) for a disease with prevalence $P(D) = 0.01$. Given a positive test, the chance of disease is:

$$P(D \mid +) = \frac{0.99 \times 0.01}{0.99 \times 0.01 + 0.01 \times 0.99} = \frac{0.0099}{0.0198} = 0.5.$$

Only **50%** — because the disease is rare, false positives are as numerous as true ones.

## Why it matters

Bayes is the backbone of probabilistic inference: spam filters, Bayesian models, and the very logic of updating predictions as data arrives. It also formalises why a rare condition resists even an accurate test.

> **Common pitfall.** Ignoring the prior — the **base-rate fallacy**. A 99%-accurate test on a rare disease still yields a coin-flip posterior. The likelihood alone is not the answer; the prior pulls it strongly toward rarity.
