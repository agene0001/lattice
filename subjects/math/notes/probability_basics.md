# Probability Basics

> By the end of this lesson you should be able to set up a sample space, compute the probability of an event by counting, combine events with the addition rule (and know when you're allowed to just add), and see why these rules are the foundation under every prediction a model makes.

## Why we need a theory of "maybe"

Most interesting questions don't have certain answers. Will this email turn out to be spam? Will the patient test positive? Will the next word be "the"? Probability is the language for reasoning carefully about "maybe" — it pins a number between $0$ and $1$ to how strongly we expect something, where $0$ means impossible, $1$ means certain, and the interesting cases live in between. The whole point is to replace hand-wavy intuition (which is famously bad at chance) with arithmetic you can check.

## The vocabulary

Three words carry the whole subject; get them straight and everything else follows.

- **Sample space** $\Omega$ — the set of *all* possible outcomes of an experiment. For one die roll, $\Omega = \{1, 2, 3, 4, 5, 6\}$.
- **Event** — any subset of the sample space, i.e. a collection of outcomes you care about. "Roll an even number" is the event $\{2, 4, 6\}$.
- **Probability** $P$ — a function that assigns each event a number in $[0, 1]$.

When every outcome is **equally likely**, probability is nothing more than counting:

$$P(A) = \frac{\text{number of outcomes in } A}{\text{total number of outcomes}}.$$

## The three rules everything rests on

A probability isn't allowed to be just any number-assignment; it must obey three axioms (due to Kolmogorov). They're almost obvious, which is the point — they're the minimum for the word "probability" to mean anything:

1. **Non-negativity:** $P(A) \ge 0$. You can't have a negative chance.
2. **Normalisation:** $P(\Omega) = 1$. *Something* in the sample space must happen.
3. **Additivity:** if $A$ and $B$ are **mutually exclusive** (they can't both happen), then $P(A \cup B) = P(A) + P(B)$.

From these you can derive everything, including the **complement rule** $P(\text{not } A) = 1 - P(A)$, which is often the easy way in: the chance of "at least one" is usually best found as $1 -$ the chance of "none."

## Worked example 1 — counting

A standard die is rolled. What is $P(\text{even})$?

The sample space has $6$ equally likely outcomes; the event "even" is $\{2, 4, 6\}$, which has $3$ outcomes.

$$P(\text{even}) = \frac{3}{6} = \frac{1}{2}.$$

And by the complement rule, $P(\text{odd}) = 1 - \tfrac12 = \tfrac12$ — no need to recount.

## Worked example 2 — when you can add, and when you can't

Roll one die. Let $A = \{\text{even}\} = \{2,4,6\}$ and $B = \{\text{at least } 5\} = \{5, 6\}$. Find $P(A \text{ or } B)$.

These events **overlap**: the outcome $6$ is in both. If you naïvely add, you double-count it. The correct tool is the **general addition rule**:

$$P(A \cup B) = P(A) + P(B) - P(A \cap B).$$

Here $P(A) = \tfrac{3}{6}$, $P(B) = \tfrac{2}{6}$, and $P(A \cap B) = P(\{6\}) = \tfrac{1}{6}$:

$$P(A \cup B) = \frac{3}{6} + \frac{2}{6} - \frac{1}{6} = \frac{4}{6} = \frac{2}{3}.$$

Subtracting the intersection corrects for the $6$ we counted twice. When events are mutually exclusive, $P(A \cap B) = 0$ and the rule collapses back to plain addition — that's the special case, not the default.

## Worked example 3 — building toward data

A bag holds $3$ red and $5$ blue marbles; you draw one at random. Then $P(\text{red}) = \tfrac{3}{8}$, and the complement gives $P(\text{blue}) = 1 - \tfrac{3}{8} = \tfrac{5}{8}$ — which of course matches counting the $5$ blue out of $8$. Two routes, same answer, is a good consistency check on your reasoning.

## Why it matters

Machine-learning models don't output decisions, they output **probabilities** — $P(\text{spam}) = 0.92$, a distribution over the next token, a confidence for each class. The training objective (cross-entropy) is built directly from these numbers, and it only makes sense if they obey the axioms above: non-negative, summing to one across the possibilities. This lesson is also the on-ramp to [[conditional_probability]] (updating a probability once you have evidence) and [[bayes_theorem]] (flipping a conditional around) — the machinery behind reasoning under uncertainty.

## Common pitfalls

- **Adding probabilities of overlapping events.** $P(A \cup B) = P(A) + P(B)$ holds *only* when $A$ and $B$ can't both happen. Otherwise subtract $P(A \cap B)$.
- **Forgetting outcomes must be equally likely** before you count. $P = \frac{\text{favourable}}{\text{total}}$ assumes a fair, uniform sample space.
- **Probabilities exceeding 1.** If an answer comes out above $1$ or below $0$, a rule was misapplied — there are no exceptions to $0 \le P \le 1$.
- **Confusing "or" with "and."** "$A$ or $B$" is the union (either, possibly both); "$A$ and $B$" is the intersection (both at once). They have different sizes and different rules.

## Check yourself

<details>
<summary>1. Draw one card from a 52-card deck. What is $P(\text{heart})$?</summary>

There are $13$ hearts out of $52$ equally likely cards: $P(\text{heart}) = \tfrac{13}{52} = \tfrac{1}{4}$.
</details>

<details>
<summary>2. Roll a die. What is $P(\text{at least } 2)$? (Use the complement.)</summary>

The complement of "at least 2" is "exactly 1," with probability $\tfrac{1}{6}$. So $P(\text{at least } 2) = 1 - \tfrac{1}{6} = \tfrac{5}{6}$.
</details>

<details>
<summary>3. $A = \{1,2,3\}$ and $B = \{3,4\}$ on a fair die. Find $P(A \cup B)$.</summary>

$P(A) = \tfrac{3}{6}$, $P(B) = \tfrac{2}{6}$, and they share the outcome $3$, so $P(A \cap B) = \tfrac{1}{6}$. Then $P(A \cup B) = \tfrac{3}{6} + \tfrac{2}{6} - \tfrac{1}{6} = \tfrac{4}{6} = \tfrac{2}{3}$.
</details>

Ready to drill it? Hit **Practise this concept** for graded problems — straightforward $P(\text{event})$, complements, and combinations.
