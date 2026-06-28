# Dot Product

**Intuition.** The dot product measures how much two vectors point the *same way* — it collapses a pair of vectors down to a single number that grows when they align and shrinks (even goes negative) when they diverge.

## Definition

For two vectors of the same length, $\mathbf{a} = (a_1, a_2, \dots, a_n)$ and $\mathbf{b} = (b_1, b_2, \dots, b_n)$, the dot product multiplies matching components and adds the results:

$$\mathbf{a} \cdot \mathbf{b} = \sum_{i=1}^{n} a_i b_i = a_1 b_1 + a_2 b_2 + \cdots + a_n b_n.$$

The output is a single scalar, not a vector. Geometrically it also equals $\lVert \mathbf{a} \rVert \, \lVert \mathbf{b} \rVert \cos\theta$, where $\theta$ is the angle between them — so a dot product of $0$ means the vectors are perpendicular.

## Worked example

Take $\mathbf{a} = (2, -1, 3)$ and $\mathbf{b} = (4, 5, 1)$. Pair up the components and sum:

$$\mathbf{a} \cdot \mathbf{b} = (2)(4) + (-1)(5) + (3)(1) = 8 - 5 + 3 = 6.$$

The result, $6$, is positive, so these two vectors point in broadly the same direction.

## Why it matters

Every neuron in a network computes a dot product of its inputs with its weights before anything else happens. Master this and a forward pass stops being mysterious — it's dot products all the way down.

> **Common pitfall.** The dot product is *not* componentwise multiplication. $(2,-1,3)\cdot(4,5,1)$ is the single number $6$, **not** the vector $(8,-5,3)$. You add the products together; you don't keep them separate.
