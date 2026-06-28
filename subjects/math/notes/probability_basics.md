# Probability Basics

**Intuition.** Probability is a number between $0$ and $1$ that measures how strongly we expect something to happen — $0$ is impossible, $1$ is certain, and everything interesting lives in between.

## Definition

A **sample space** $\Omega$ is the set of all possible outcomes; an **event** is a subset of $\Omega$. A probability $P$ obeys three axioms:

1. $P(A) \ge 0$ for every event $A$,
2. $P(\Omega) = 1$ (something must happen),
3. for **mutually exclusive** events, $P(A \cup B) = P(A) + P(B)$.

When outcomes are equally likely, probability is just counting:

$$P(A) = \frac{\text{favourable outcomes}}{\text{total outcomes}}.$$

The general addition rule (when events can overlap) is $P(A \cup B) = P(A) + P(B) - P(A \cap B)$.

## Worked example

Roll a fair die. Let $A = \{\text{even}\}$ and $B = \{\text{at least } 5\}$.

$$P(A) = \tfrac{3}{6}, \quad P(B) = \tfrac{2}{6}, \quad P(A \cap B) = P(\{6\}) = \tfrac{1}{6}.$$
$$P(A \cup B) = \tfrac{3}{6} + \tfrac{2}{6} - \tfrac{1}{6} = \tfrac{4}{6} = \tfrac{2}{3}.$$

## Why it matters

Models output probabilities, losses are built from them (cross-entropy), and uncertainty is the language of every prediction. The axioms are the rules those outputs must obey to be coherent.

> **Common pitfall.** You may only add probabilities directly when the events **can't both happen**. If they overlap, adding double-counts the intersection — subtract $P(A \cap B)$ to fix it.
