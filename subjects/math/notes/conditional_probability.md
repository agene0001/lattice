# Conditional Probability

**Intuition.** Conditional probability is belief after evidence: once you know $B$ happened, you shrink the world down to $B$ and ask how much of it is also $A$.

## Definition

The probability of $A$ **given** $B$ is

$$P(A \mid B) = \frac{P(A \cap B)}{P(B)}, \qquad P(B) > 0.$$

You're rescaling: among the outcomes where $B$ holds, what fraction also have $A$? Rearranged, this gives the **multiplication rule** $P(A \cap B) = P(A \mid B)\,P(B)$. Events are **independent** exactly when conditioning changes nothing: $P(A \mid B) = P(A)$.

## Worked example

Draw one card from a standard deck. Given that the card is a face card ($B$, 12 cards), what's the chance it's a king ($A$, 4 kings, all face cards)?

$$P(A \mid B) = \frac{P(A \cap B)}{P(B)} = \frac{4/52}{12/52} = \frac{4}{12} = \frac{1}{3}.$$

Knowing "it's a face card" lifts the chance of a king from $\tfrac{4}{52}$ to $\tfrac13$.

## Why it matters

Almost everything a model does is conditional — $P(\text{label} \mid \text{features})$. Conditioning on evidence is the formal version of "updating your prediction once you see the input."

> **Common pitfall.** $P(A \mid B)$ and $P(B \mid A)$ are **not** the same. $P(\text{king} \mid \text{face}) = \tfrac13$, but $P(\text{face} \mid \text{king}) = 1$. Swapping the condition for the event is a real, costly error — it's exactly what Bayes' theorem exists to correct.
