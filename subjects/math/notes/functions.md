# Functions

**Intuition.** A function is a reliable machine: feed it an input and it returns exactly one output — same input, same output, every time.

## Definition

A function $f$ assigns to each input $x$ a single output $f(x)$. Its **domain** is the set of inputs it accepts; its **range** is the set of outputs it can produce. The "exactly one output" rule is what separates a function from a general relation.

Functions **compose**: applying $g$ then $f$ is written $(f \circ g)(x) = f(g(x))$ — the output of the inner machine becomes the input of the outer one. Order matters.

## Worked example

Let $f(x) = x^2$ and $g(x) = x + 3$. Then

$$(f \circ g)(2) = f(g(2)) = f(5) = 25,$$

while the other order gives

$$(g \circ f)(2) = g(f(2)) = g(4) = 7.$$

Same two functions, different answers — composition is not commutative.

## Why it matters

A neural network *is* a composition of functions — each layer is one machine, and the whole model is them nested together. The chain rule, which trains the network, is precisely the rule for differentiating $f(g(x))$.

> **Common pitfall.** $f(x) = \sqrt{x}$ has domain $x \ge 0$ and $f(x) = 1/x$ excludes $x = 0$. The domain is part of the function's definition — plugging in a value the function doesn't accept isn't a small error, it's undefined.
