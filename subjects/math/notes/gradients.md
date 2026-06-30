# Gradients

> By the end of this lesson you should be able to compute the gradient of a multivariable function, say in plain words what its direction and length *mean*, and explain why every training loop in machine learning is built around the phrase $-\nabla f$.

## Start with a hill

Imagine standing on a hillside in the fog. You can't see the summit, but you *can* feel the ground under your feet. Two questions you could answer immediately: **which way is steepest uphill**, and **how steep is it**. The gradient is the mathematical object that answers both at once — for any function, in any number of dimensions.

For a function of one variable, $f(x)$, "steepness" is just the derivative $f'(x)$: one number, slope of the tangent line. But a loss surface in machine learning depends on *millions* of variables, and at a single point it slopes differently in every direction. We need something richer than one number. That richer thing is a **vector**.

## The idea

Here's the move. For a function $f(x, y)$ of two variables, we already know how to measure its slope in the $x$-direction (the partial derivative $\partial f / \partial x$) and in the $y$-direction ($\partial f / \partial y$). The gradient simply **collects those slopes into one vector**:

$$\nabla f = \left( \frac{\partial f}{\partial x},\; \frac{\partial f}{\partial y} \right).$$

The symbol $\nabla$ is called "nabla" or just "del." For $n$ variables it's the same idea with $n$ components:

$$\nabla f = \left( \frac{\partial f}{\partial x_1},\; \frac{\partial f}{\partial x_2},\; \dots,\; \frac{\partial f}{\partial x_n} \right).$$

Two facts give this vector its meaning, and they're the whole reason gradients matter:

1. **Direction.** $\nabla f$ points in the direction of *steepest ascent* — the compass bearing of "most uphill" from where you stand.
2. **Magnitude.** Its length $\lVert \nabla f \rVert$ is *how steep* that steepest climb is. On flat ground the gradient is the zero vector.

And the consequence we'll lean on forever: the **negative** gradient, $-\nabla f$, points in the direction of steepest *descent* — the fastest way down.

## Worked example 1 — the basic mechanic

Let $f(x, y) = x^2 + 3y^2$. Find the gradient, then evaluate it at the point $(1, 1)$.

**Step 1 — partial with respect to $x$** (treat $y$ as a constant, so $3y^2$ is just a constant and differentiates to $0$):

$$\frac{\partial f}{\partial x} = 2x.$$

**Step 2 — partial with respect to $y$** (treat $x$ as a constant):

$$\frac{\partial f}{\partial y} = 6y.$$

**Step 3 — assemble the gradient:**

$$\nabla f = (2x,\; 6y).$$

**Step 4 — evaluate at $(1, 1)$:**

$$\nabla f(1, 1) = (2,\; 6).$$

Read it back: at $(1,1)$ the function climbs fastest in the direction $(2, 6)$ — mostly along $y$, because the $3y^2$ term makes the surface steeper in that direction.

## Worked example 2 — three variables

The mechanic doesn't change as you add dimensions. Let $f(x, y, z) = x^2 + 2y^2 + 5z^2$, and evaluate the gradient at $(1, -1, 2)$.

$$\nabla f = (2x,\; 4y,\; 10z), \qquad \nabla f(1, -1, 2) = (2,\; -4,\; 20).$$

The $z$-component dominates — a small nudge in $z$ changes $f$ far more than a nudge in $x$, because of the coefficient $5$. Notice the negative middle component: increasing $y$ from $-1$ actually moves you *toward* the valley floor there, so steepest *ascent* points the other way.

## Why the direction claim is true (the intuition)

You don't have to take "steepest ascent" on faith. The change in $f$ when you take a tiny step $\mathbf{u}$ (a unit direction) is, to first order, the dot product

$$\text{change in } f \;\approx\; \nabla f \cdot \mathbf{u} \;=\; \lVert \nabla f \rVert \, \lVert \mathbf{u} \rVert \cos\theta,$$

where $\theta$ is the angle between your step and the gradient. Since $\mathbf{u}$ is a unit vector, this is largest exactly when $\cos\theta = 1$ — that is, when you step *in the same direction as $\nabla f$*. Step against it ($\cos\theta = -1$) and $f$ decreases fastest; step perpendicular ($\cos\theta = 0$) and $f$ doesn't change at all (you're walking along a contour line). The dot product from [[dot_product]] is doing all the work here.

## Why it matters: this is how models train

Training a model means choosing parameters $\boldsymbol\theta$ that make a loss $L(\boldsymbol\theta)$ as small as possible. The loss surface is a landscape in millions of dimensions, and we're trying to find its lowest valley in the fog. The gradient is the only instrument we have — and it points *uphill*, so we step the other way:

$$\boldsymbol\theta_{\text{new}} = \boldsymbol\theta_{\text{old}} - \eta\,\nabla L(\boldsymbol\theta_{\text{old}}).$$

That single line is **gradient descent** (the next concept, [[gradient_descent]]), and computing $\nabla L$ efficiently for a deep network is exactly what **backpropagation** does via the [[chain_rule]]. Everything downstream is this lesson, repeated.

## Common pitfalls

- **A gradient is a vector, not a number.** It has one component per input variable. Reporting a single number for $\nabla f$ of a two-variable function is the most common slip.
- **It points uphill.** To *minimise*, you step along $-\nabla f$. Dropping the minus sign sends the optimiser climbing the loss — the classic "my loss is going up" bug.
- **Evaluate after differentiating.** $\nabla f = (2x, 6y)$ is a function of position; the answer at a point is found by plugging that point in. $\nabla f(1,1) = (2,6)$, not $(2x, 6y)$.
- **Each partial freezes *all* other variables.** In $\partial/\partial x$, every $y$ and $z$ is treated as a constant — including inside product terms.

## Check yourself

<details>
<summary>1. Find $\nabla f$ for $f(x,y) = 4x^2 + y^2$, then evaluate at $(2, 3)$.</summary>

$\nabla f = (8x,\; 2y)$, so $\nabla f(2,3) = (16,\; 6)$.
</details>

<details>
<summary>2. At a point where $\nabla f = (0, 0)$, what does the surface look like?</summary>

It's locally flat — a candidate minimum, maximum, or saddle point. Gradient descent stops moving here because the step $-\eta\,\nabla f$ is zero. Finding these "critical points" is exactly what optimisation is hunting for.
</details>

<details>
<summary>3. You step in a direction perpendicular to $\nabla f$. How does $f$ change?</summary>

To first order, not at all: $\cos\theta = 0$ in $\nabla f \cdot \mathbf{u}$. You're walking along a contour (level set) of the function.
</details>

Ready to drill it? Hit **Practise this concept** for graded problems, from two-variable gradients up to three-variable ones.
