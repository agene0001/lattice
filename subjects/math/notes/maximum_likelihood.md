# Maximum Likelihood

**Intuition.** Maximum likelihood asks: of all the parameter settings I could choose, which one makes the data I actually observed the *least surprising*? Pick the parameters that best explain what happened.

## Definition

Given data and a model with parameters $\boldsymbol\theta$, the **likelihood** is the probability of the data as a function of $\boldsymbol\theta$. For independent observations it's a product, so we maximise the **log-likelihood** (sums are easier than products, and the maximiser is the same):

$$\hat{\boldsymbol\theta} = \arg\max_{\boldsymbol\theta} \sum_{i=1}^{n} \log p(x_i \mid \boldsymbol\theta).$$

Maximising is done by setting the derivative to zero (or by gradient *ascent*).

## Worked example

A coin lands heads $7$ of $10$ flips; estimate $P(\text{heads}) = p$. The log-likelihood is

$$\ell(p) = 7\log p + 3\log(1 - p).$$

Differentiate and set to zero:

$$\ell'(p) = \frac{7}{p} - \frac{3}{1-p} = 0 \implies 7(1-p) = 3p \implies p = 0.7.$$

The estimate is exactly the observed frequency — reassuringly sensible.

## Why it matters

MLE is the principle *behind* common loss functions: minimising cross-entropy is maximising likelihood for a classifier, and minimising squared error is MLE under Gaussian noise. It connects "fit the data" to a precise probabilistic objective.

> **Common pitfall.** Maximising likelihood and minimising loss are the same thing up to a sign — we minimise the **negative** log-likelihood. Forgetting that flip turns gradient descent into gradient ascent and sends the optimiser the wrong way.
