# Gradients

**Intuition.** The gradient packs all the partial derivatives into one vector that points in the direction of steepest ascent — the compass needle for "uphill" on a multivariable surface.

## Definition

For $f(x_1, \dots, x_n)$, the gradient is the vector of partial derivatives:

$$\nabla f = \left( \frac{\partial f}{\partial x_1}, \frac{\partial f}{\partial x_2}, \dots, \frac{\partial f}{\partial x_n} \right).$$

Two facts give it meaning at every point: it **points in the direction of fastest increase**, and its **magnitude** is how steep that increase is. The negative gradient, $-\nabla f$, points downhill — the fastest way to decrease $f$.

## Worked example

Let $f(x, y) = x^2 + 3y^2$.

$$\nabla f = (2x,\; 6y).$$

At the point $(1, 1)$, $\nabla f = (2, 6)$ — steepest ascent leans mostly in the $y$ direction, because the surface climbs faster there.

## Why it matters

Gradient descent — the algorithm behind nearly all model training — repeatedly steps in the direction $-\nabla f$ to push the loss down. The gradient is the single object that tells the optimiser which way to move all parameters at once.

> **Common pitfall.** The gradient is a **vector**, not a scalar — one component per input variable. And it points *uphill*; to minimise a loss you step along $-\nabla f$. Dropping the minus sign sends you climbing the loss instead of descending it.
