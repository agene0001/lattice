# Newton's Second Law

**Intuition.** A net force makes a mass accelerate, and the heavier the object, the less it accelerates for the same push. Force is what *changes* motion — not what sustains it.

## Definition

The net force on an object equals its mass times its acceleration:

$$F_{\text{net}} = ma.$$

Force is measured in **newtons** ($\text{N}$), where $1\,\text{N} = 1\,\text{kg}\cdot\text{m/s}^2$ — the unit is *defined* by this law. "Net" is essential: it's the vector sum of all forces acting.

## A picture: the free-body diagram

Before computing, draw the forces as arrows from the object. Here a block is pushed right by $F$ while gravity $W$ pulls down and the surface pushes up with the normal force $N$:

<figure>
<svg viewBox="0 0 220 170" role="img" aria-label="Free-body diagram of a block on a surface">
  <rect x="80" y="70" width="52" height="32" fill="none" stroke="currentColor" stroke-width="2" />
  <line x1="106" y1="102" x2="106" y2="150" stroke="currentColor" stroke-width="2" />
  <polygon points="106,158 101,146 111,146" fill="currentColor" />
  <text x="114" y="140" fill="currentColor" font-size="13">W = mg</text>
  <line x1="106" y1="70" x2="106" y2="20" stroke="currentColor" stroke-width="2" />
  <polygon points="106,12 101,24 111,24" fill="currentColor" />
  <text x="114" y="32" fill="currentColor" font-size="13">N</text>
  <line x1="132" y1="86" x2="196" y2="86" stroke="currentColor" stroke-width="2" />
  <polygon points="204,86 192,81 192,91" fill="currentColor" />
  <text x="158" y="78" fill="currentColor" font-size="13">F</text>
</svg>
<figcaption>The <em>net</em> of these arrows is what goes into F = ma.</figcaption>
</figure>

## Worked example

A net force accelerates a $2\,\text{kg}$ mass at $3\,\text{m/s}^2$:

$$F = ma = (2\,\text{kg})(3\,\text{m/s}^2) = 6\,\text{N}.$$

## Why it matters

This is the hinge between kinematics (describing motion) and dynamics (explaining it). Given the forces, you get the acceleration, and the kinematic equations take you the rest of the way to position and velocity.

> **Common pitfall.** $F = ma$ uses the **net** force. If several forces act, add them (as vectors) first — a common error is plugging in a single applied force while ignoring friction or gravity.
