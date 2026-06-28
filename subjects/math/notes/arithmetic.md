# Arithmetic

**Intuition.** Arithmetic is the grammar of every later topic: if the order in which you combine numbers is shaky, every derivative and dot product built on top of it inherits the mistake.

## Definition

Arithmetic combines numbers with four operations — addition, subtraction, multiplication, and division — under a fixed **order of operations**:

1. **P**arentheses (innermost first)
2. **E**xponents
3. **M**ultiplication and **D**ivision, left to right
4. **A**ddition and **S**ubtraction, left to right

Multiplication and division share a tier (neither beats the other — you go left to right), and so do addition and subtraction. A fraction bar acts like invisible parentheses around its top and bottom.

## Worked example

Evaluate $3 + 4 \times 2^2 - (6 - 1)$.

$$3 + 4 \times 2^2 - (6 - 1) = 3 + 4 \times 4 - 5 = 3 + 16 - 5 = 14.$$

Exponent first ($2^2 = 4$), then the multiplication ($4 \times 4 = 16$), then left-to-right addition and subtraction.

## Why it matters

Models are billions of these operations chained together. A sign error or a misordered step here doesn't announce itself — it just produces a confidently wrong number downstream.

> **Common pitfall.** $-3^2$ is $-9$, not $9$. The exponent binds tighter than the minus sign, so it reads as $-(3^2)$. You only get $9$ if the negative is inside parentheses: $(-3)^2$.
