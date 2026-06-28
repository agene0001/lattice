# Matrices

**Intuition.** A matrix is a grid of numbers — read it as a stack of vectors, a table of data, or (most powerfully) a machine that transforms one vector into another.

## Definition

A matrix is a rectangular array with $m$ rows and $n$ columns, its **shape** written $m \times n$:

$$A = \begin{bmatrix} a_{11} & a_{12} \\ a_{21} & a_{22} \\ a_{31} & a_{32} \end{bmatrix} \quad (3 \times 2).$$

The entry $a_{ij}$ sits in row $i$, column $j$ — **row first, then column**. You can view the matrix as a column of row-vectors or a row of column-vectors; both readings matter later.

## Worked example

In

$$A = \begin{bmatrix} 5 & 8 & 1 \\ 0 & 2 & 7 \end{bmatrix},$$

the shape is $2 \times 3$. The entry $a_{23}$ (row 2, column 3) is $7$, and $a_{12}$ is $8$.

## Why it matters

A layer of a network is a matrix of weights; an image is a matrix of pixels; a dataset is a matrix with one row per example. Almost every computation in ML is "apply a matrix to some vectors," so getting comfortable with shape and indexing pays off constantly.

> **Common pitfall.** Indexing is row-then-column, and shape is rows-then-columns. A $2 \times 3$ matrix has 2 rows and 3 columns — mixing up the order is the single most common source of shape-mismatch errors downstream.
