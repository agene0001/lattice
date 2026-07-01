# Units & Dimensions

**Intuition.** A physical quantity is a number *and* a unit — "5" is meaningless until you say 5 what. The unit carries the physics; drop it and you can't tell a force from an energy.

## The SI base units

Everything is built from seven base units. The four you'll use constantly:

| Quantity | Unit | Symbol |
|---|---|---|
| length | metre | $\text{m}$ |
| mass | kilogram | $\text{kg}$ |
| time | second | $\text{s}$ |
| current | ampere | $\text{A}$ |

Derived units are combinations: speed is $\text{m/s}$, force is $\text{kg}\cdot\text{m/s}^2$ (called a newton, $\text{N}$).

## Converting units

Multiply by a conversion factor equal to 1. To convert $2\,\text{km}$ to metres, use $1\,\text{km} = 1000\,\text{m}$:

$$2\,\text{km} \times \frac{1000\,\text{m}}{1\,\text{km}} = 2000\,\text{m}.$$

The $\text{km}$ cancels, leaving metres.

## Why it matters

**Dimensional analysis** is a free correctness check: both sides of any physics equation must have the same units. If you derive a "speed" that comes out in $\text{kg}\cdot\text{s}$, you made an algebra error — no computation needed to know it's wrong.

> **Common pitfall.** Always carry units through the whole calculation and state them in your answer. In Lattice, a physics answer is graded on its units too: `6 N` is correct where `6` alone or `6 J` is not.
