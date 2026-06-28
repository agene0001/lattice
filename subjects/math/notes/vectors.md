# Vectors

**Intuition.** A vector is one object that bundles several numbers together — read it as an arrow in space *or* as a list of features, whichever helps.

## Definition

A vector is an ordered tuple of numbers, its **components**:

$$\mathbf{v} = (v_1, v_2, \dots, v_n).$$

The count $n$ is its **dimension**. Two pictures of the same object:

- **Geometric:** an arrow from the origin to the point $(v_1, \dots, v_n)$, with a length and a direction.
- **List:** a row of measurements — a data point, a pixel's RGB, a word embedding.

Its **length** (magnitude) is the Pythagorean distance from the origin:

$$\lVert \mathbf{v} \rVert = \sqrt{v_1^2 + v_2^2 + \cdots + v_n^2}.$$

## Worked example

For $\mathbf{v} = (3, 4)$:

$$\lVert \mathbf{v} \rVert = \sqrt{3^2 + 4^2} = \sqrt{9 + 16} = \sqrt{25} = 5.$$

The arrow to $(3,4)$ has length $5$ — the hypotenuse of a 3–4–5 triangle.

## Why it matters

In machine learning every example is a vector. "Similar" examples are nearby points; a model's parameters are themselves one big vector. Thinking in vectors is what lets one formula handle a thousand features at once.

> **Common pitfall.** Order is part of the identity: $(3, 4) \neq (4, 3)$. A vector is *ordered* — the components are positions, not an unordered bag of numbers.
