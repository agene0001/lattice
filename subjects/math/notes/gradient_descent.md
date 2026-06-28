# Gradient Descent

**Intuition.** To find the bottom of a valley in fog, feel which way is downhill and take a step — then repeat. Gradient descent is that loop, with the gradient as the slope underfoot.

## Definition

To minimise a loss $L(\boldsymbol\theta)$ over parameters $\boldsymbol\theta$, repeatedly step opposite the gradient:

$$\boldsymbol\theta_{t+1} = \boldsymbol\theta_t - \eta\,\nabla L(\boldsymbol\theta_t).$$

Here $\eta > 0$ is the **learning rate** (step size). The negative sign points downhill; the gradient's magnitude makes steps large where the surface is steep and small as it flattens near a minimum.

## Worked example

Minimise $L(\theta) = \theta^2$, so $L'(\theta) = 2\theta$. Start at $\theta_0 = 4$ with $\eta = 0.1$:

$$\theta_1 = 4 - 0.1(2\cdot 4) = 4 - 0.8 = 3.2,$$
$$\theta_2 = 3.2 - 0.1(2 \cdot 3.2) = 3.2 - 0.64 = 2.56.$$

Each step multiplies $\theta$ by $0.8$, marching steadily toward the minimum at $\theta = 0$.

## Why it matters

This single update rule trains essentially every modern model. Backprop supplies the gradient; gradient descent (and its variants — SGD, Adam) consumes it to nudge millions of parameters downhill, step after step.

> **Common pitfall.** The learning rate is delicate. Too small and training crawls; too large and the steps **overshoot** the minimum and can diverge — with $\eta = 1$ above, $\theta_1 = 4 - 8 = -4$, bouncing instead of settling. It's the first knob to check when training won't converge.
