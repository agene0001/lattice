# Limits

**Intuition.** A limit answers "where is this function *heading* as the input closes in on a point?" — regardless of, and even in spite of, what happens exactly at that point.

## Definition

$$\lim_{x \to a} f(x) = L$$

means $f(x)$ gets arbitrarily close to $L$ as $x$ gets close to $a$ from both sides. The value $f(a)$ itself may be undefined or different — the limit is about the approach, not the destination. The limit exists only if the left-hand and right-hand approaches agree.

Many limits you can get by direct substitution. The interesting ones are **indeterminate** (like $\tfrac{0}{0}$), where you first simplify.

## Worked example

$$\lim_{x \to 2} \frac{x^2 - 4}{x - 2}.$$

Substituting gives $\tfrac{0}{0}$ — indeterminate. Factor the top: $x^2 - 4 = (x-2)(x+2)$, cancel the $(x-2)$, then substitute:

$$\lim_{x \to 2} \frac{(x-2)(x+2)}{x-2} = \lim_{x \to 2}(x + 2) = 4.$$

The function has a hole at $x = 2$, but it's clearly heading toward $4$.

## Why it matters

The derivative is *defined* as a limit — the limit of an average rate of change as the interval shrinks to nothing. Without limits, "instantaneous slope" has no meaning.

> **Common pitfall.** A limit existing says nothing about $f(a)$ being defined. In the example, $f(2)$ is undefined ($\tfrac{0}{0}$), yet the limit is a clean $4$. Don't conflate "value at the point" with "limit at the point."
