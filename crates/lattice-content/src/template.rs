//! Parameterized problem templates (spec §2.3, §5).
//!
//! The V1 generation strategy in one sentence: **generate the answer first, then
//! build the problem around it.** A linear equation whose solution we chose is
//! solvable by construction; a dot product we computed ourselves has a known
//! result. That sidesteps the "LLM silently emits a problem with no valid
//! solution" failure mode (spec §2.3) and lets a property test assert the
//! invariant over thousands of random instances (spec open Q5).
//!
//! The *parameters* (coefficient ranges, dimensions, difficulty) are data, loaded
//! from `subjects/<id>/templates.json`. The *solver* for each kind is verified
//! Rust here. Re-using a kind for a new subject is pure data; a genuinely new
//! problem *form* needs a new [`TemplateKind`] variant plus its sampler.

use lattice_core::{ConceptId, Difficulty, Problem, ProblemId, ProblemSource, SubjectId};
// rand 0.10 moved range sampling (`random_range`) onto the `RngExt` trait.
use rand::{Rng, RngExt};
use serde::{Deserialize, Serialize};

/// One authored template: which concept it drills, at what difficulty, plus the
/// kind-specific parameter ranges (flattened in the JSON).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub id: String,
    pub concept: ConceptId,
    pub difficulty: Difficulty,
    #[serde(flatten)]
    pub kind: TemplateKind,
}

/// The supported template kinds, internally tagged by `"kind"` in JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TemplateKind {
    /// `a·x + b = c`, solve for `x` — the algebra foundation under everything.
    LinearEquation {
        a_range: [i64; 2],
        x_range: [i64; 2],
        b_range: [i64; 2],
    },
    /// Dot product of two integer vectors — the linear-algebra primitive under
    /// matrix multiply and every neural-net layer.
    DotProduct { dim: usize, value_range: [i64; 2] },
    /// Power-rule derivative of a single monomial `a·x^n`.
    PowerRuleDerivative {
        coeff_range: [i64; 2],
        exponent_range: [i64; 2],
    },
    /// Evaluate `a + b·c` (respecting precedence). Foundations arithmetic.
    ArithmeticEval {
        a_range: [i64; 2],
        b_range: [i64; 2],
        c_range: [i64; 2],
    },
    /// Simplify `b^m · b^n` into a single power. Exponent rules.
    ExponentProduct {
        base_range: [i64; 2],
        exponent_range: [i64; 2],
    },
    /// Component-wise sum of two integer vectors.
    VectorSum { dim: usize, value_range: [i64; 2] },
    /// Product of a 2×2 integer matrix and a 2-vector.
    MatrixVectorProduct { value_range: [i64; 2] },
    /// `P(red)` for a bag of red/blue marbles — a reduced fraction. Probability.
    SimpleProbability {
        red_range: [i64; 2],
        blue_range: [i64; 2],
    },
    /// `P(not red)` for a bag of red/blue marbles — the complement rule, as a
    /// reduced fraction. A second *form* for probability basics.
    ComplementProbability {
        red_range: [i64; 2],
        blue_range: [i64; 2],
    },
    /// `E[X]` for a 3-value uniform random variable. Statistics.
    ExpectationUniform {
        mean_range: [i64; 2],
        spread_range: [i64; 2],
    },
    /// Derivative of a quadratic `a·x² + b·x + c`. Calculus.
    PolynomialDerivative {
        a_range: [i64; 2],
        b_range: [i64; 2],
        c_range: [i64; 2],
    },
    /// Partial derivative `∂/∂x (a·x² + b·y²)`. Multivariable calculus.
    PartialDerivative {
        a_range: [i64; 2],
        b_range: [i64; 2],
    },
    /// `P(B | A)` from counts — a reduced fraction. Conditional probability.
    ConditionalProbability { total_range: [i64; 2] },
    /// `P(disease | +)` in natural-frequency form — a reduced fraction. Bayes.
    BayesNaturalFrequency { count_range: [i64; 2] },
    /// `Var(X)` of a symmetric two-point variable (= spread²). Statistics.
    VarianceTwoPoint {
        mean_range: [i64; 2],
        spread_range: [i64; 2],
    },
    /// Chain rule: `d/dx (a·x + b)^n`. The heart of backprop.
    ChainRule {
        a_range: [i64; 2],
        b_range: [i64; 2],
        exponent_range: [i64; 2],
    },
    /// Gradient of `a·x² + b·y²` at a point. Toward gradient descent.
    Gradient {
        coeff_range: [i64; 2],
        point_range: [i64; 2],
    },
    /// Gradient of `a·x² + b·y² + c·z²` at a point — the three-variable form, a
    /// harder tier for gradients.
    Gradient3Var {
        coeff_range: [i64; 2],
        point_range: [i64; 2],
    },
    /// Evaluate a linear function `f(x) = a·x + b` at a point. Functions.
    FunctionEval {
        a_range: [i64; 2],
        b_range: [i64; 2],
        x_range: [i64; 2],
    },
    /// Factor a difference of squares `x² − a²`. Factoring.
    DifferenceOfSquares { root_range: [i64; 2] },
    /// Removable limit `lim_{x→a} (x²−a²)/(x−a) = 2a`. Limits.
    RemovableLimit { root_range: [i64; 2] },
    /// Trace of a 2×2 matrix. Matrices.
    MatrixTrace { value_range: [i64; 2] },
    /// Product of two 2×2 matrices. Matrix multiplication.
    MatrixMultiply { value_range: [i64; 2] },
    /// `P(X = k)` heads in n fair flips — a reduced fraction. Random variables.
    BinomialHeads { flips_range: [i64; 2] },
    /// One gradient-descent step on `f(x) = x²` with η = ¼. Gradient descent.
    GradientDescentStep { value_range: [i64; 2] },
    /// Maximum-likelihood estimate of a coin's P(heads). Maximum likelihood.
    MleCoin { flips_range: [i64; 2] },
    /// Read off the i-th entry of an integer vector. Vectors (definitional).
    VectorComponent { dim: usize, value_range: [i64; 2] },

    // --- Physics (answers carry units; graded by the units-aware `answers_match`) ---
    /// Average speed = distance / time. Answer in `m/s`.
    AverageSpeed {
        speed_range: [i64; 2],
        time_range: [i64; 2],
    },
    /// Acceleration from a standing start: a = Δv / t. Answer in `m/s^2`.
    AccelerationFromSpeed {
        accel_range: [i64; 2],
        time_range: [i64; 2],
    },
    /// Final velocity under constant acceleration: v = u + a·t. Answer in `m/s`.
    FinalVelocity {
        u_range: [i64; 2],
        a_range: [i64; 2],
        t_range: [i64; 2],
    },
    /// Newton's second law: F = m·a. Answer in `N`.
    NewtonSecondLaw {
        mass_range: [i64; 2],
        accel_range: [i64; 2],
    },
    /// Weight from mass: W = m·g with g = 9.8 m/s². Answer in `N`.
    Weight { mass_range: [i64; 2] },
    /// Unit conversion: kilometres to metres. Answer in `m`.
    UnitConversion { value_range: [i64; 2] },

    // --- Calculus: extra derivative *forms* (variety fights template-memorizing) ---
    /// Derivative of a cubic `a·x³ + b·x² + c·x + d` → `3a·x² + 2b·x + c`.
    CubicDerivative {
        a_range: [i64; 2],
        b_range: [i64; 2],
        c_range: [i64; 2],
        d_range: [i64; 2],
    },
    /// Product rule: `d/dx[(a·x + b)(c·x + d)]` → `2ac·x + (ad + bc)`.
    ProductRuleDerivative {
        a_range: [i64; 2],
        b_range: [i64; 2],
        c_range: [i64; 2],
        d_range: [i64; 2],
    },
    /// Word problem: slope of the tangent to `f(x) = a·x² + b·x + c` at `x = k`,
    /// i.e. `f'(k) = 2a·k + b` — a single number, testing derivative + evaluation.
    TangentLineSlope {
        a_range: [i64; 2],
        b_range: [i64; 2],
        c_range: [i64; 2],
        x_range: [i64; 2],
    },

    // --- Rolled-out extra forms: calculus, algebra, linear algebra ---
    /// Limit by direct substitution: `lim_{x→k}(a·x² + b·x + c)` → a number.
    PolynomialLimit {
        a_range: [i64; 2],
        b_range: [i64; 2],
        c_range: [i64; 2],
        x_range: [i64; 2],
    },
    /// Chain rule evaluated at a point: `d/dx (a·x + b)^n` at `x = k` → a number.
    ChainRuleAtPoint {
        a_range: [i64; 2],
        b_range: [i64; 2],
        exponent_range: [i64; 2],
        x_range: [i64; 2],
    },
    /// Partial derivative with a cross term: `∂/∂x (a·x² + b·xy + c·y²)` → `2a·x + b·y`.
    PartialWithCrossTerm {
        a_range: [i64; 2],
        b_range: [i64; 2],
        c_range: [i64; 2],
    },
    /// Quotient of powers: `b^m / b^n` → `b^{m-n}`.
    ExponentQuotient {
        base_range: [i64; 2],
        exponent_range: [i64; 2],
    },
    /// Function composition: `f(x)=a·x+b`, `g(x)=c·x+d`, find `f(g(k))` → a number.
    FunctionComposition {
        a_range: [i64; 2],
        b_range: [i64; 2],
        c_range: [i64; 2],
        d_range: [i64; 2],
        x_range: [i64; 2],
    },
    /// Factor a monic quadratic `x² + (p+q)·x + pq` → `(x+p)(x+q)`.
    FactorQuadratic { root_range: [i64; 2] },
    /// Determinant of a 2×2 integer matrix → `ad − bc`.
    MatrixDeterminant2x2 { value_range: [i64; 2] },

    // --- #2 rollout: second forms for prob/stats + vectors ---
    /// Linearity of expectation: given `E[X] = m`, find `E[aX + b] = a·m + b`.
    LinearityOfExpectation {
        mean_range: [i64; 2],
        a_range: [i64; 2],
        b_range: [i64; 2],
    },
    /// Variance scaling: given `Var(X) = v`, find `Var(aX) = a²·v`.
    VarianceScaling {
        var_range: [i64; 2],
        a_range: [i64; 2],
    },
    /// Squared magnitude of an integer vector: `‖v‖² = Σ vᵢ²`.
    VectorMagnitudeSquared { dim: usize, value_range: [i64; 2] },
    /// PMF of a fair n-sided die: `P(X = k) = 1/n`.
    UniformPmf { sides_range: [i64; 2] },

    // --- #4 deep vertical: more calculus-for-ML concepts ---
    /// Second derivative of a cubic: `d²/dx²(a·x³ + b·x² + …)` → `6a·x + 2b`.
    SecondDerivative {
        a_range: [i64; 2],
        b_range: [i64; 2],
        c_range: [i64; 2],
        d_range: [i64; 2],
    },
    /// Critical point of `a·x² + b·x + c`: where `f'(x)=0`, i.e. `x = -b/(2a)`.
    CriticalPoint {
        a_range: [i64; 2],
        b_range: [i64; 2],
        c_range: [i64; 2],
    },
    /// Derivative of `e^{a·x}` → `a·e^{a·x}` (chain rule with the exponential).
    ExponentialDerivative { coeff_range: [i64; 2] },
    /// Derivative of `ln(a·x)` → `1/x` (independent of `a`, for `a>0`).
    LogDerivative { coeff_range: [i64; 2] },
    /// Minimum value of an upward parabola `a·x² + b·x + c` (a>0): `(4ac − b²)/(4a)`.
    QuadraticMinimum {
        a_range: [i64; 2],
        b_range: [i64; 2],
        c_range: [i64; 2],
    },
}

/// A rendered instance: the problem statement and its solution, both as LaTeX.
#[derive(Default)]
pub struct Instance {
    pub content: String,
    pub solution: String,
    /// Deterministic worked-solution steps (KaTeX / `$math$`), empty when a
    /// template hasn't authored them — the UI then falls back to the AI explainer.
    pub steps: Vec<String>,
    /// Progressive hint ladder, empty when not authored.
    pub hints: Vec<String>,
}

impl Template {
    /// Generate one concrete, guaranteed-solvable [`Problem`] from this template.
    pub fn generate(&self, subject_id: &SubjectId, rng: &mut impl Rng) -> Problem {
        let instance = self.kind.sample(rng);
        Problem {
            id: ProblemId::new(),
            subject_id: subject_id.clone(),
            concepts: vec![self.concept.clone()],
            difficulty: self.difficulty,
            content: instance.content,
            solution: instance.solution,
            generated_by: ProblemSource::Template,
            attribution: None,
            hints: instance.hints,
            steps: instance.steps,
        }
    }
}

impl TemplateKind {
    fn sample(&self, rng: &mut impl Rng) -> Instance {
        match self {
            TemplateKind::LinearEquation {
                a_range,
                x_range,
                b_range,
            } => LinearEq::sample(rng, a_range, x_range, b_range).render(),
            TemplateKind::DotProduct { dim, value_range } => {
                DotProduct::sample(rng, *dim, value_range).render()
            }
            TemplateKind::PowerRuleDerivative {
                coeff_range,
                exponent_range,
            } => PowerRule::sample(rng, coeff_range, exponent_range).render(),
            TemplateKind::ArithmeticEval {
                a_range,
                b_range,
                c_range,
            } => ArithmeticEval::sample(rng, a_range, b_range, c_range).render(),
            TemplateKind::ExponentProduct {
                base_range,
                exponent_range,
            } => ExponentProduct::sample(rng, base_range, exponent_range).render(),
            TemplateKind::VectorSum { dim, value_range } => {
                VectorSum::sample(rng, *dim, value_range).render()
            }
            TemplateKind::MatrixVectorProduct { value_range } => {
                MatrixVectorProduct::sample(rng, value_range).render()
            }
            TemplateKind::SimpleProbability {
                red_range,
                blue_range,
            } => SimpleProbability::sample(rng, red_range, blue_range).render(),
            TemplateKind::ComplementProbability {
                red_range,
                blue_range,
            } => ComplementProbability::sample(rng, red_range, blue_range).render(),
            TemplateKind::ExpectationUniform {
                mean_range,
                spread_range,
            } => ExpectationUniform::sample(rng, mean_range, spread_range).render(),
            TemplateKind::PolynomialDerivative {
                a_range,
                b_range,
                c_range,
            } => PolynomialDerivative::sample(rng, a_range, b_range, c_range).render(),
            TemplateKind::PartialDerivative { a_range, b_range } => {
                PartialDerivative::sample(rng, a_range, b_range).render()
            }
            TemplateKind::ConditionalProbability { total_range } => {
                ConditionalProbability::sample(rng, total_range).render()
            }
            TemplateKind::BayesNaturalFrequency { count_range } => {
                BayesNaturalFrequency::sample(rng, count_range).render()
            }
            TemplateKind::VarianceTwoPoint {
                mean_range,
                spread_range,
            } => VarianceTwoPoint::sample(rng, mean_range, spread_range).render(),
            TemplateKind::ChainRule {
                a_range,
                b_range,
                exponent_range,
            } => ChainRule::sample(rng, a_range, b_range, exponent_range).render(),
            TemplateKind::Gradient {
                coeff_range,
                point_range,
            } => Gradient::sample(rng, coeff_range, point_range).render(),
            TemplateKind::Gradient3Var {
                coeff_range,
                point_range,
            } => Gradient3Var::sample(rng, coeff_range, point_range).render(),
            TemplateKind::FunctionEval {
                a_range,
                b_range,
                x_range,
            } => FunctionEval::sample(rng, a_range, b_range, x_range).render(),
            TemplateKind::DifferenceOfSquares { root_range } => {
                DifferenceOfSquares::sample(rng, root_range).render()
            }
            TemplateKind::RemovableLimit { root_range } => {
                RemovableLimit::sample(rng, root_range).render()
            }
            TemplateKind::MatrixTrace { value_range } => {
                MatrixTrace::sample(rng, value_range).render()
            }
            TemplateKind::MatrixMultiply { value_range } => {
                MatrixMultiply::sample(rng, value_range).render()
            }
            TemplateKind::BinomialHeads { flips_range } => {
                BinomialHeads::sample(rng, flips_range).render()
            }
            TemplateKind::GradientDescentStep { value_range } => {
                GradientDescentStep::sample(rng, value_range).render()
            }
            TemplateKind::MleCoin { flips_range } => {
                MleCoin::sample(rng, flips_range).render()
            }
            TemplateKind::VectorComponent { dim, value_range } => {
                VectorComponent::sample(rng, *dim, value_range).render()
            }
            TemplateKind::AverageSpeed {
                speed_range,
                time_range,
            } => AverageSpeed::sample(rng, speed_range, time_range).render(),
            TemplateKind::AccelerationFromSpeed {
                accel_range,
                time_range,
            } => AccelerationFromSpeed::sample(rng, accel_range, time_range).render(),
            TemplateKind::FinalVelocity {
                u_range,
                a_range,
                t_range,
            } => FinalVelocity::sample(rng, u_range, a_range, t_range).render(),
            TemplateKind::NewtonSecondLaw {
                mass_range,
                accel_range,
            } => NewtonSecondLaw::sample(rng, mass_range, accel_range).render(),
            TemplateKind::Weight { mass_range } => Weight::sample(rng, mass_range).render(),
            TemplateKind::UnitConversion { value_range } => {
                UnitConversion::sample(rng, value_range).render()
            }
            TemplateKind::CubicDerivative {
                a_range,
                b_range,
                c_range,
                d_range,
            } => CubicDerivative::sample(rng, a_range, b_range, c_range, d_range).render(),
            TemplateKind::ProductRuleDerivative {
                a_range,
                b_range,
                c_range,
                d_range,
            } => ProductRuleDerivative::sample(rng, a_range, b_range, c_range, d_range).render(),
            TemplateKind::TangentLineSlope {
                a_range,
                b_range,
                c_range,
                x_range,
            } => TangentLineSlope::sample(rng, a_range, b_range, c_range, x_range).render(),
            TemplateKind::PolynomialLimit {
                a_range,
                b_range,
                c_range,
                x_range,
            } => PolynomialLimit::sample(rng, a_range, b_range, c_range, x_range).render(),
            TemplateKind::ChainRuleAtPoint {
                a_range,
                b_range,
                exponent_range,
                x_range,
            } => ChainRuleAtPoint::sample(rng, a_range, b_range, exponent_range, x_range).render(),
            TemplateKind::PartialWithCrossTerm {
                a_range,
                b_range,
                c_range,
            } => PartialWithCrossTerm::sample(rng, a_range, b_range, c_range).render(),
            TemplateKind::ExponentQuotient {
                base_range,
                exponent_range,
            } => ExponentQuotient::sample(rng, base_range, exponent_range).render(),
            TemplateKind::FunctionComposition {
                a_range,
                b_range,
                c_range,
                d_range,
                x_range,
            } => FunctionComposition::sample(rng, a_range, b_range, c_range, d_range, x_range)
                .render(),
            TemplateKind::FactorQuadratic { root_range } => {
                FactorQuadratic::sample(rng, root_range).render()
            }
            TemplateKind::MatrixDeterminant2x2 { value_range } => {
                MatrixDeterminant2x2::sample(rng, value_range).render()
            }
            TemplateKind::LinearityOfExpectation {
                mean_range,
                a_range,
                b_range,
            } => LinearityOfExpectation::sample(rng, mean_range, a_range, b_range).render(),
            TemplateKind::VarianceScaling { var_range, a_range } => {
                VarianceScaling::sample(rng, var_range, a_range).render()
            }
            TemplateKind::VectorMagnitudeSquared { dim, value_range } => {
                VectorMagnitudeSquared::sample(rng, *dim, value_range).render()
            }
            TemplateKind::UniformPmf { sides_range } => {
                UniformPmf::sample(rng, sides_range).render()
            }
            TemplateKind::SecondDerivative {
                a_range,
                b_range,
                c_range,
                d_range,
            } => SecondDerivative::sample(rng, a_range, b_range, c_range, d_range).render(),
            TemplateKind::CriticalPoint {
                a_range,
                b_range,
                c_range,
            } => CriticalPoint::sample(rng, a_range, b_range, c_range).render(),
            TemplateKind::ExponentialDerivative { coeff_range } => {
                ExponentialDerivative::sample(rng, coeff_range).render()
            }
            TemplateKind::LogDerivative { coeff_range } => {
                LogDerivative::sample(rng, coeff_range).render()
            }
            TemplateKind::QuadraticMinimum {
                a_range,
                b_range,
                c_range,
            } => QuadraticMinimum::sample(rng, a_range, b_range, c_range).render(),
        }
    }
}

// --- Typed instances: each knows how to verify itself and render itself. ---
//
// Keeping a structured form (not just the rendered strings) is what lets the
// property tests assert the math invariant directly.

struct LinearEq {
    a: i64,
    b: i64,
    c: i64,
    x: i64,
}

impl LinearEq {
    fn sample(rng: &mut impl Rng, a_range: &[i64; 2], x_range: &[i64; 2], b_range: &[i64; 2]) -> Self {
        let a = sample_nonzero(rng, a_range);
        let x = sample_in(rng, x_range);
        let b = sample_in(rng, b_range);
        Self { a, b, c: a * x + b, x }
    }

    /// Verification: the chosen `x` actually satisfies the equation.
    #[cfg(test)]
    fn holds(&self) -> bool {
        self.a * self.x + self.b == self.c
    }

    fn render(&self) -> Instance {
        Instance {
            content: format!("{} = {}", linear_lhs(self.a, self.b), self.c),
            solution: format!("x = {}", self.x),
            steps: vec![
                format!("Start with ${} = {}$.", linear_lhs(self.a, self.b), self.c),
                format!(
                    "Isolate the $x$ term: ${} = {}$.",
                    linear_lhs(self.a, 0),
                    self.c - self.b
                ),
                format!("Divide both sides by ${}$: $x = {}$.", self.a, self.x),
            ],
            hints: vec![
                "Get the $x$ term alone on one side first.".to_string(),
                "Then divide by the coefficient of $x$.".to_string(),
            ],
        }
    }
}

struct DotProduct {
    u: Vec<i64>,
    v: Vec<i64>,
    dot: i64,
}

impl DotProduct {
    fn sample(rng: &mut impl Rng, dim: usize, value_range: &[i64; 2]) -> Self {
        let u: Vec<i64> = (0..dim).map(|_| sample_in(rng, value_range)).collect();
        let v: Vec<i64> = (0..dim).map(|_| sample_in(rng, value_range)).collect();
        let dot = u.iter().zip(&v).map(|(a, b)| a * b).sum();
        Self { u, v, dot }
    }

    /// Verification: independently recompute the dot product.
    #[cfg(test)]
    fn holds(&self) -> bool {
        let recomputed: i64 = self.u.iter().zip(&self.v).map(|(a, b)| a * b).sum();
        recomputed == self.dot
    }

    fn render(&self) -> Instance {
        Instance {
            content: format!(
                "\\mathbf{{u}} \\cdot \\mathbf{{v}} = \\;? \\qquad \\mathbf{{u}} = {},\\quad \\mathbf{{v}} = {}",
                row_vec(&self.u),
                row_vec(&self.v)
            ),
            solution: self.dot.to_string(),
            ..Default::default()
        }
    }
}

struct PowerRule {
    a: i64,
    n: i64,
}

impl PowerRule {
    fn sample(rng: &mut impl Rng, coeff_range: &[i64; 2], exponent_range: &[i64; 2]) -> Self {
        Self {
            a: sample_nonzero(rng, coeff_range),
            n: sample_in(rng, exponent_range),
        }
    }

    /// Verification: cross-check the symbolic derivative `a·n·x^(n-1)` against a
    /// central finite-difference approximation at a few sample points. (A nice
    /// reminder that the analytic and numerical derivatives must agree.)
    #[cfg(test)]
    fn holds(&self) -> bool {
        let f = |x: f64| (self.a as f64) * x.powi(self.n as i32);
        let f_prime = |x: f64| (self.a as f64) * (self.n as f64) * x.powi((self.n - 1) as i32);
        let h = 1e-4;
        [0.5_f64, 1.3, 2.1].iter().all(|&x| {
            let numeric = (f(x + h) - f(x - h)) / (2.0 * h);
            (numeric - f_prime(x)).abs() < 1e-2
        })
    }

    fn render(&self) -> Instance {
        Instance {
            content: format!("\\frac{{d}}{{dx}}\\left({}\\right)", monomial(self.a, self.n)),
            solution: monomial(self.a * self.n, self.n - 1),
            steps: vec![
                "Power rule: multiply by the exponent, then subtract 1 from it.".to_string(),
                format!(
                    "Bring down the exponent: ${} \\cdot {} = {}$.",
                    self.a,
                    self.n,
                    self.a * self.n
                ),
                format!(
                    "Reduce the exponent by one to get ${}$.",
                    monomial(self.a * self.n, self.n - 1)
                ),
            ],
            hints: vec![format!(
                "For $a x^n$, the derivative is $a n\\, x^{{n-1}}$."
            )],
        }
    }
}

struct ArithmeticEval {
    a: i64,
    b: i64,
    c: i64,
}

impl ArithmeticEval {
    fn sample(rng: &mut impl Rng, a_range: &[i64; 2], b_range: &[i64; 2], c_range: &[i64; 2]) -> Self {
        Self {
            a: sample_in(rng, a_range),
            b: sample_in(rng, b_range),
            c: sample_in(rng, c_range),
        }
    }

    fn value(&self) -> i64 {
        self.a + self.b * self.c
    }

    fn render(&self) -> Instance {
        Instance {
            content: format!("{} + {} \\times {} = \\;?", self.a, self.b, self.c),
            solution: self.value().to_string(),
            ..Default::default()
        }
    }
}

struct ExponentProduct {
    base: i64,
    m: i64,
    n: i64,
}

impl ExponentProduct {
    fn sample(rng: &mut impl Rng, base_range: &[i64; 2], exponent_range: &[i64; 2]) -> Self {
        Self {
            base: sample_in(rng, base_range),
            m: sample_in(rng, exponent_range),
            n: sample_in(rng, exponent_range),
        }
    }

    /// Verification: `b^m · b^n == b^(m+n)`.
    #[cfg(test)]
    fn holds(&self) -> bool {
        self.base.pow(self.m as u32) * self.base.pow(self.n as u32)
            == self.base.pow((self.m + self.n) as u32)
    }

    fn render(&self) -> Instance {
        Instance {
            content: format!(
                "{b}^{{{m}}} \\cdot {b}^{{{n}}}",
                b = self.base,
                m = self.m,
                n = self.n
            ),
            solution: format!("{}^{{{}}}", self.base, self.m + self.n),
            ..Default::default()
        }
    }
}

struct VectorSum {
    u: Vec<i64>,
    v: Vec<i64>,
}

impl VectorSum {
    fn sample(rng: &mut impl Rng, dim: usize, value_range: &[i64; 2]) -> Self {
        Self {
            u: (0..dim).map(|_| sample_in(rng, value_range)).collect(),
            v: (0..dim).map(|_| sample_in(rng, value_range)).collect(),
        }
    }

    fn sum(&self) -> Vec<i64> {
        self.u.iter().zip(&self.v).map(|(a, b)| a + b).collect()
    }

    fn render(&self) -> Instance {
        Instance {
            content: format!("{} + {}", col_vec(&self.u), col_vec(&self.v)),
            solution: components(&self.sum()),
            ..Default::default()
        }
    }
}

struct MatrixVectorProduct {
    m: [[i64; 2]; 2],
    v: [i64; 2],
}

impl MatrixVectorProduct {
    fn sample(rng: &mut impl Rng, value_range: &[i64; 2]) -> Self {
        Self {
            m: [
                [sample_in(rng, value_range), sample_in(rng, value_range)],
                [sample_in(rng, value_range), sample_in(rng, value_range)],
            ],
            v: [sample_in(rng, value_range), sample_in(rng, value_range)],
        }
    }

    fn result(&self) -> [i64; 2] {
        [
            self.m[0][0] * self.v[0] + self.m[0][1] * self.v[1],
            self.m[1][0] * self.v[0] + self.m[1][1] * self.v[1],
        ]
    }

    fn render(&self) -> Instance {
        Instance {
            content: format!("{} \\, {}", mat2(&self.m), col_vec(&self.v)),
            solution: components(&self.result()),
            ..Default::default()
        }
    }
}

struct SimpleProbability {
    red: i64,
    blue: i64,
}

impl SimpleProbability {
    fn sample(rng: &mut impl Rng, red_range: &[i64; 2], blue_range: &[i64; 2]) -> Self {
        Self {
            red: sample_in(rng, red_range).max(1),
            blue: sample_in(rng, blue_range).max(1),
        }
    }

    /// `P(red)` as a reduced fraction `(numerator, denominator)`.
    fn fraction(&self) -> (i64, i64) {
        let total = self.red + self.blue;
        let g = gcd(self.red, total);
        (self.red / g, total / g)
    }

    fn render(&self) -> Instance {
        let (p, q) = self.fraction();
        Instance {
            content: format!(
                "\\text{{A bag holds }} {r} \\text{{ red and }} {b} \\text{{ blue marbles. }} P(\\text{{red}}) = \\;?",
                r = self.red,
                b = self.blue
            ),
            solution: format!("{p}/{q}"),
            ..Default::default()
        }
    }
}

struct ComplementProbability {
    red: i64,
    blue: i64,
}

impl ComplementProbability {
    fn sample(rng: &mut impl Rng, red_range: &[i64; 2], blue_range: &[i64; 2]) -> Self {
        Self {
            red: sample_in(rng, red_range).max(1),
            blue: sample_in(rng, blue_range).max(1),
        }
    }

    /// `P(not red) = blue / total`, reduced.
    fn fraction(&self) -> (i64, i64) {
        let total = self.red + self.blue;
        let g = gcd(self.blue, total);
        (self.blue / g, total / g)
    }

    fn render(&self) -> Instance {
        let (p, q) = self.fraction();
        Instance {
            content: format!(
                "\\text{{A bag holds }} {r} \\text{{ red and }} {b} \\text{{ blue marbles. }} P(\\text{{not red}}) = \\;?",
                r = self.red,
                b = self.blue
            ),
            solution: format!("{p}/{q}"),
            ..Default::default()
        }
    }
}

struct ExpectationUniform {
    values: [i64; 3],
    mean: i64,
}

impl ExpectationUniform {
    fn sample(rng: &mut impl Rng, mean_range: &[i64; 2], spread_range: &[i64; 2]) -> Self {
        // Choose the mean first, then values that sum to 3·mean, so the uniform
        // expectation is exactly an integer by construction.
        let m = sample_in(rng, mean_range);
        let d1 = sample_in(rng, spread_range);
        let d2 = sample_in(rng, spread_range);
        Self {
            values: [m + d1, m + d2, m - d1 - d2],
            mean: m,
        }
    }

    fn render(&self) -> Instance {
        let [a, b, c] = self.values;
        Instance {
            content: format!(
                "X \\in \\{{ {a}, {b}, {c} \\}} \\text{{, each with probability }} \\tfrac{{1}}{{3}}. \\quad E[X] = \\;?"
            ),
            solution: self.mean.to_string(),
            ..Default::default()
        }
    }
}

struct PolynomialDerivative {
    a: i64,
    b: i64,
    c: i64,
}

impl PolynomialDerivative {
    fn sample(
        rng: &mut impl Rng,
        a_range: &[i64; 2],
        b_range: &[i64; 2],
        c_range: &[i64; 2],
    ) -> Self {
        Self {
            a: sample_nonzero(rng, a_range),
            b: sample_in(rng, b_range),
            c: sample_in(rng, c_range),
        }
    }

    /// Verification: the analytic derivative `2a·x + b` matches a central finite
    /// difference of `a·x² + b·x + c`.
    #[cfg(test)]
    fn holds(&self) -> bool {
        let f = |x: f64| (self.a as f64) * x * x + (self.b as f64) * x + (self.c as f64);
        let f_prime = |x: f64| 2.0 * (self.a as f64) * x + (self.b as f64);
        let h = 1e-4;
        [0.5_f64, 1.3, 2.1].iter().all(|&x| {
            let numeric = (f(x + h) - f(x - h)) / (2.0 * h);
            (numeric - f_prime(x)).abs() < 1e-3
        })
    }

    fn render(&self) -> Instance {
        Instance {
            content: format!(
                "\\frac{{d}}{{dx}}\\left( {} \\right)",
                quadratic(self.a, self.b, self.c)
            ),
            solution: linear_lhs(2 * self.a, self.b),
            ..Default::default()
        }
    }
}

struct PartialDerivative {
    a: i64,
    b: i64,
}

impl PartialDerivative {
    fn sample(rng: &mut impl Rng, a_range: &[i64; 2], b_range: &[i64; 2]) -> Self {
        Self {
            a: sample_in(rng, a_range),
            b: sample_in(rng, b_range),
        }
    }

    fn render(&self) -> Instance {
        Instance {
            // ∂/∂x (a·x² + b·y²) = 2a·x, treating y as a constant.
            content: format!(
                "\\frac{{\\partial}}{{\\partial x}}\\left( {a}x^2 + {b}y^2 \\right)",
                a = self.a,
                b = self.b
            ),
            solution: format!("{}x", 2 * self.a),
            ..Default::default()
        }
    }
}

struct ConditionalProbability {
    total: i64,
    subset: i64,
}

impl ConditionalProbability {
    fn sample(rng: &mut impl Rng, total_range: &[i64; 2]) -> Self {
        let total = sample_in(rng, total_range).max(2);
        let subset = rng.random_range(1..=total);
        Self { total, subset }
    }

    /// `P(science | math) = subset / total`, reduced.
    fn fraction(&self) -> (i64, i64) {
        let g = gcd(self.subset, self.total);
        (self.subset / g, self.total / g)
    }

    fn render(&self) -> Instance {
        let (p, q) = self.fraction();
        Instance {
            content: format!(
                "\\text{{Of }} {t} \\text{{ students who like math, }} {s} \\text{{ also like science. }} P(\\text{{science}} \\mid \\text{{math}}) = \\;?",
                t = self.total,
                s = self.subset
            ),
            solution: format!("{p}/{q}"),
            ..Default::default()
        }
    }
}

struct BayesNaturalFrequency {
    diseased: i64,
    healthy: i64,
    true_pos: i64,
    false_pos: i64,
}

impl BayesNaturalFrequency {
    fn sample(rng: &mut impl Rng, count_range: &[i64; 2]) -> Self {
        let true_pos = sample_in(rng, count_range).max(1);
        let false_pos = sample_in(rng, count_range).max(1);
        Self {
            true_pos,
            false_pos,
            diseased: true_pos + rng.random_range(0..=4),
            healthy: false_pos + rng.random_range(0..=6),
        }
    }

    /// `P(disease | +) = true_pos / (true_pos + false_pos)`, reduced.
    fn fraction(&self) -> (i64, i64) {
        let positives = self.true_pos + self.false_pos;
        let g = gcd(self.true_pos, positives);
        (self.true_pos / g, positives / g)
    }

    fn render(&self) -> Instance {
        let (p, q) = self.fraction();
        Instance {
            content: format!(
                "\\text{{Among }} {n} \\text{{ people, }} {d} \\text{{ have a disease; }} {tp} \\text{{ of them test positive, as do }} {fp} \\text{{ of the }} {h} \\text{{ healthy. }} P(\\text{{disease}} \\mid +) = \\;?",
                n = self.diseased + self.healthy,
                d = self.diseased,
                tp = self.true_pos,
                fp = self.false_pos,
                h = self.healthy
            ),
            solution: format!("{p}/{q}"),
            ..Default::default()
        }
    }
}

struct VarianceTwoPoint {
    lo: i64,
    hi: i64,
}

impl VarianceTwoPoint {
    fn sample(rng: &mut impl Rng, mean_range: &[i64; 2], spread_range: &[i64; 2]) -> Self {
        let m = sample_in(rng, mean_range);
        let k = sample_in(rng, spread_range).abs().max(1);
        // Symmetric two-point variable: Var = k² exactly.
        Self {
            lo: m - k,
            hi: m + k,
        }
    }

    fn variance(&self) -> i64 {
        let k = (self.hi - self.lo) / 2;
        k * k
    }

    fn render(&self) -> Instance {
        Instance {
            content: format!(
                "X \\in \\{{ {lo}, {hi} \\}} \\text{{, each with probability }} \\tfrac{{1}}{{2}}. \\quad \\mathrm{{Var}}(X) = \\;?",
                lo = self.lo,
                hi = self.hi
            ),
            solution: self.variance().to_string(),
            ..Default::default()
        }
    }
}

struct ChainRule {
    a: i64,
    b: i64,
    n: i64,
}

impl ChainRule {
    fn sample(
        rng: &mut impl Rng,
        a_range: &[i64; 2],
        b_range: &[i64; 2],
        exponent_range: &[i64; 2],
    ) -> Self {
        Self {
            a: sample_nonzero(rng, a_range),
            b: sample_in(rng, b_range),
            n: sample_in(rng, exponent_range),
        }
    }

    /// Verification: `n·a·(ax+b)^(n-1)` matches a finite difference of `(ax+b)^n`.
    #[cfg(test)]
    fn holds(&self) -> bool {
        let inner = |x: f64| (self.a as f64) * x + self.b as f64;
        let f = |x: f64| inner(x).powi(self.n as i32);
        let f_prime =
            |x: f64| (self.n as f64) * (self.a as f64) * inner(x).powi((self.n - 1) as i32);
        let h = 1e-4;
        [0.3_f64, 0.7, 1.1].iter().all(|&x| {
            let numeric = (f(x + h) - f(x - h)) / (2.0 * h);
            (numeric - f_prime(x)).abs() < 1e-2
        })
    }

    fn render(&self) -> Instance {
        let coeff = self.n * self.a;
        let inner = format!("({})", linear_lhs(self.a, self.b));
        let exp = self.n - 1;
        let solution = if exp == 1 {
            format!("{coeff}{inner}")
        } else {
            format!("{coeff}{inner}^{{{exp}}}")
        };
        Instance {
            content: format!(
                "\\frac{{d}}{{dx}}\\left( ({})^{{{}}} \\right)",
                linear_lhs(self.a, self.b),
                self.n
            ),
            solution,
            ..Default::default()
        }
    }
}

struct Gradient {
    a: i64,
    b: i64,
    x0: i64,
    y0: i64,
}

impl Gradient {
    fn sample(rng: &mut impl Rng, coeff_range: &[i64; 2], point_range: &[i64; 2]) -> Self {
        Self {
            a: sample_nonzero(rng, coeff_range),
            b: sample_nonzero(rng, coeff_range),
            x0: sample_in(rng, point_range),
            y0: sample_in(rng, point_range),
        }
    }

    /// ∇(a·x² + b·y²) = (2a·x, 2b·y), evaluated at the point.
    fn grad(&self) -> [i64; 2] {
        [2 * self.a * self.x0, 2 * self.b * self.y0]
    }

    fn render(&self) -> Instance {
        Instance {
            content: format!(
                "f(x,y) = {a}x^2 + {b}y^2. \\quad \\nabla f \\text{{ at }} ({x0}, {y0}) = \\;?",
                a = self.a,
                b = self.b,
                x0 = self.x0,
                y0 = self.y0
            ),
            solution: components(&self.grad()),
            ..Default::default()
        }
    }
}

struct Gradient3Var {
    a: i64,
    b: i64,
    c: i64,
    x0: i64,
    y0: i64,
    z0: i64,
}

impl Gradient3Var {
    fn sample(rng: &mut impl Rng, coeff_range: &[i64; 2], point_range: &[i64; 2]) -> Self {
        Self {
            a: sample_nonzero(rng, coeff_range),
            b: sample_nonzero(rng, coeff_range),
            c: sample_nonzero(rng, coeff_range),
            x0: sample_in(rng, point_range),
            y0: sample_in(rng, point_range),
            z0: sample_in(rng, point_range),
        }
    }

    /// ∇(a·x² + b·y² + c·z²) = (2a·x, 2b·y, 2c·z), evaluated at the point.
    fn grad(&self) -> [i64; 3] {
        [2 * self.a * self.x0, 2 * self.b * self.y0, 2 * self.c * self.z0]
    }

    fn render(&self) -> Instance {
        Instance {
            content: format!(
                "f(x,y,z) = {a}x^2 + {b}y^2 + {c}z^2. \\quad \\nabla f \\text{{ at }} ({x0}, {y0}, {z0}) = \\;?",
                a = self.a,
                b = self.b,
                c = self.c,
                x0 = self.x0,
                y0 = self.y0,
                z0 = self.z0
            ),
            solution: components(&self.grad()),
            ..Default::default()
        }
    }
}

struct FunctionEval {
    a: i64,
    b: i64,
    x: i64,
}

impl FunctionEval {
    fn sample(rng: &mut impl Rng, a_range: &[i64; 2], b_range: &[i64; 2], x_range: &[i64; 2]) -> Self {
        Self {
            a: sample_nonzero(rng, a_range),
            b: sample_in(rng, b_range),
            x: sample_in(rng, x_range),
        }
    }

    fn value(&self) -> i64 {
        self.a * self.x + self.b
    }

    fn render(&self) -> Instance {
        Instance {
            content: format!(
                "f(x) = {}, \\quad f({}) = \\;?",
                linear_lhs(self.a, self.b),
                self.x
            ),
            solution: self.value().to_string(),
            ..Default::default()
        }
    }
}

struct DifferenceOfSquares {
    root: i64,
}

impl DifferenceOfSquares {
    fn sample(rng: &mut impl Rng, root_range: &[i64; 2]) -> Self {
        Self {
            root: sample_in(rng, root_range).abs().max(1),
        }
    }

    fn render(&self) -> Instance {
        Instance {
            content: format!("\\text{{Factor }} x^2 - {}", self.root * self.root),
            // Canonical order; the V1 answer check is order-sensitive (open Q6).
            solution: format!("(x - {r})(x + {r})", r = self.root),
            ..Default::default()
        }
    }
}

struct RemovableLimit {
    root: i64,
}

impl RemovableLimit {
    fn sample(rng: &mut impl Rng, root_range: &[i64; 2]) -> Self {
        Self {
            root: sample_in(rng, root_range).abs().max(1),
        }
    }

    fn limit(&self) -> i64 {
        2 * self.root
    }

    fn render(&self) -> Instance {
        Instance {
            content: format!(
                "\\lim_{{x \\to {r}}} \\frac{{x^2 - {r2}}}{{x - {r}}} = \\;?",
                r = self.root,
                r2 = self.root * self.root
            ),
            solution: self.limit().to_string(),
            ..Default::default()
        }
    }
}

struct MatrixTrace {
    m: [[i64; 2]; 2],
}

impl MatrixTrace {
    fn sample(rng: &mut impl Rng, value_range: &[i64; 2]) -> Self {
        Self {
            m: [
                [sample_in(rng, value_range), sample_in(rng, value_range)],
                [sample_in(rng, value_range), sample_in(rng, value_range)],
            ],
        }
    }

    fn trace(&self) -> i64 {
        self.m[0][0] + self.m[1][1]
    }

    fn render(&self) -> Instance {
        Instance {
            content: format!("\\mathrm{{tr}} \\, {} = \\;?", mat2(&self.m)),
            solution: self.trace().to_string(),
            ..Default::default()
        }
    }
}

struct MatrixMultiply {
    a: [[i64; 2]; 2],
    b: [[i64; 2]; 2],
}

impl MatrixMultiply {
    fn sample(rng: &mut impl Rng, value_range: &[i64; 2]) -> Self {
        let mut s = |range: &[i64; 2]| sample_in(rng, range);
        Self {
            a: [[s(value_range), s(value_range)], [s(value_range), s(value_range)]],
            b: [[s(value_range), s(value_range)], [s(value_range), s(value_range)]],
        }
    }

    /// Row-major product entries [c00, c01, c10, c11].
    fn product(&self) -> [i64; 4] {
        let (a, b) = (&self.a, &self.b);
        [
            a[0][0] * b[0][0] + a[0][1] * b[1][0],
            a[0][0] * b[0][1] + a[0][1] * b[1][1],
            a[1][0] * b[0][0] + a[1][1] * b[1][0],
            a[1][0] * b[0][1] + a[1][1] * b[1][1],
        ]
    }

    fn render(&self) -> Instance {
        Instance {
            content: format!("{} \\, {} = \\;?", mat2(&self.a), mat2(&self.b)),
            solution: components(&self.product()),
            ..Default::default()
        }
    }
}

struct BinomialHeads {
    n: i64,
    k: i64,
}

impl BinomialHeads {
    fn sample(rng: &mut impl Rng, flips_range: &[i64; 2]) -> Self {
        let n = sample_in(rng, flips_range).max(2);
        Self {
            n,
            k: rng.random_range(0..=n),
        }
    }

    /// `P(X = k) = C(n, k) / 2^n`, reduced.
    fn fraction(&self) -> (i64, i64) {
        let num = binomial(self.n, self.k);
        let den = 1_i64 << self.n;
        let g = gcd(num, den);
        (num / g, den / g)
    }

    fn render(&self) -> Instance {
        let (p, q) = self.fraction();
        Instance {
            content: format!(
                "X = \\text{{ heads in }} {n} \\text{{ fair coin flips. }} P(X = {k}) = \\;?",
                n = self.n,
                k = self.k
            ),
            solution: fraction_str(p, q),
            ..Default::default()
        }
    }
}

struct GradientDescentStep {
    x0: i64,
}

impl GradientDescentStep {
    fn sample(rng: &mut impl Rng, value_range: &[i64; 2]) -> Self {
        // x0 even so the η=¼ step `x0 - ¼·2x0 = x0/2` stays an integer.
        let j = sample_in(rng, value_range).abs().max(1);
        Self { x0: 2 * j }
    }

    fn next_x(&self) -> i64 {
        self.x0 / 2
    }

    fn render(&self) -> Instance {
        Instance {
            content: format!(
                "f(x) = x^2,\\; \\eta = \\tfrac{{1}}{{4}}. \\quad x_0 = {}, \\quad x_1 = x_0 - \\eta \\cdot 2x_0 = \\;?",
                self.x0
            ),
            solution: self.next_x().to_string(),
            ..Default::default()
        }
    }
}

struct MleCoin {
    n: i64,
    h: i64,
}

impl MleCoin {
    fn sample(rng: &mut impl Rng, flips_range: &[i64; 2]) -> Self {
        let n = sample_in(rng, flips_range).max(2);
        Self {
            n,
            h: rng.random_range(1..=n),
        }
    }

    /// MLE of P(heads) = h / n, reduced.
    fn fraction(&self) -> (i64, i64) {
        let g = gcd(self.h, self.n);
        (self.h / g, self.n / g)
    }

    fn render(&self) -> Instance {
        let (p, q) = self.fraction();
        Instance {
            content: format!(
                "\\text{{In }} {n} \\text{{ coin flips you see }} {h} \\text{{ heads. The MLE of }} P(\\text{{heads}}) = \\;?",
                n = self.n,
                h = self.h
            ),
            solution: fraction_str(p, q),
            ..Default::default()
        }
    }
}

struct VectorComponent {
    v: Vec<i64>,
    index: usize, // 1-based
}

impl VectorComponent {
    fn sample(rng: &mut impl Rng, dim: usize, value_range: &[i64; 2]) -> Self {
        let dim = dim.max(1);
        Self {
            v: (0..dim).map(|_| sample_in(rng, value_range)).collect(),
            index: rng.random_range(1..=dim),
        }
    }

    fn value(&self) -> i64 {
        self.v[self.index - 1]
    }

    fn render(&self) -> Instance {
        Instance {
            content: format!(
                "\\text{{Entry }} {i} \\text{{ of }} {vec} = \\;?",
                i = self.index,
                vec = row_vec(&self.v)
            ),
            solution: self.value().to_string(),
            ..Default::default()
        }
    }
}

// --- Calculus: extra derivative forms ---

struct CubicDerivative {
    a: i64,
    b: i64,
    c: i64,
    d: i64,
}

impl CubicDerivative {
    fn sample(
        rng: &mut impl Rng,
        a_range: &[i64; 2],
        b_range: &[i64; 2],
        c_range: &[i64; 2],
        d_range: &[i64; 2],
    ) -> Self {
        Self {
            a: sample_nonzero(rng, a_range),
            b: sample_in(rng, b_range),
            c: sample_in(rng, c_range),
            d: sample_in(rng, d_range),
        }
    }

    /// Verification: analytic `3a·x² + 2b·x + c` matches a finite difference of
    /// the cubic.
    #[cfg(test)]
    fn holds(&self) -> bool {
        let f = |x: f64| {
            (self.a as f64) * x * x * x + (self.b as f64) * x * x + (self.c as f64) * x
                + (self.d as f64)
        };
        let f_prime = |x: f64| {
            3.0 * (self.a as f64) * x * x + 2.0 * (self.b as f64) * x + (self.c as f64)
        };
        let h = 1e-4;
        [0.4_f64, 1.2, 2.3].iter().all(|&x| {
            let numeric = (f(x + h) - f(x - h)) / (2.0 * h);
            (numeric - f_prime(x)).abs() < 1e-2
        })
    }

    fn render(&self) -> Instance {
        Instance {
            content: format!(
                "\\frac{{d}}{{dx}}\\left( {} \\right)",
                cubic(self.a, self.b, self.c, self.d)
            ),
            // f'(x) = 3a x² + 2b x + c.
            solution: quadratic(3 * self.a, 2 * self.b, self.c),
            ..Default::default()
        }
    }
}

struct ProductRuleDerivative {
    a: i64,
    b: i64,
    c: i64,
    d: i64,
}

impl ProductRuleDerivative {
    fn sample(
        rng: &mut impl Rng,
        a_range: &[i64; 2],
        b_range: &[i64; 2],
        c_range: &[i64; 2],
        d_range: &[i64; 2],
    ) -> Self {
        Self {
            a: sample_nonzero(rng, a_range),
            b: sample_in(rng, b_range),
            c: sample_nonzero(rng, c_range),
            d: sample_in(rng, d_range),
        }
    }

    /// `d/dx[(ax+b)(cx+d)] = 2ac·x + (ad + bc)`.
    fn deriv_coeffs(&self) -> (i64, i64) {
        (2 * self.a * self.c, self.a * self.d + self.b * self.c)
    }

    /// Verification: finite difference of the product matches `2ac·x + (ad+bc)`.
    #[cfg(test)]
    fn holds(&self) -> bool {
        let f = |x: f64| ((self.a as f64) * x + self.b as f64) * ((self.c as f64) * x + self.d as f64);
        let (m, k) = self.deriv_coeffs();
        let f_prime = |x: f64| (m as f64) * x + (k as f64);
        let h = 1e-4;
        [0.3_f64, 1.1, 2.2].iter().all(|&x| {
            let numeric = (f(x + h) - f(x - h)) / (2.0 * h);
            (numeric - f_prime(x)).abs() < 1e-2
        })
    }

    fn render(&self) -> Instance {
        let (m, k) = self.deriv_coeffs();
        Instance {
            content: format!(
                "\\frac{{d}}{{dx}}\\left( ({})({}) \\right)",
                linear_lhs(self.a, self.b),
                linear_lhs(self.c, self.d)
            ),
            solution: linear_lhs(m, k),
            ..Default::default()
        }
    }
}

struct TangentLineSlope {
    a: i64,
    b: i64,
    c: i64,
    k: i64,
}

impl TangentLineSlope {
    fn sample(
        rng: &mut impl Rng,
        a_range: &[i64; 2],
        b_range: &[i64; 2],
        c_range: &[i64; 2],
        x_range: &[i64; 2],
    ) -> Self {
        Self {
            a: sample_nonzero(rng, a_range),
            b: sample_in(rng, b_range),
            c: sample_in(rng, c_range),
            k: sample_in(rng, x_range),
        }
    }

    /// Slope of the tangent at `x = k` is `f'(k) = 2a·k + b`.
    fn slope(&self) -> i64 {
        2 * self.a * self.k + self.b
    }

    fn render(&self) -> Instance {
        Instance {
            content: format!(
                "\\text{{What is the slope of the tangent line to }} f(x) = {} \\text{{ at }} x = {}?",
                quadratic(self.a, self.b, self.c),
                self.k
            ),
            solution: self.slope().to_string(),
            ..Default::default()
        }
    }
}

// --- Rolled-out extra forms (calculus / algebra / linear algebra) ---

struct PolynomialLimit {
    a: i64,
    b: i64,
    c: i64,
    k: i64,
}

impl PolynomialLimit {
    fn sample(rng: &mut impl Rng, a_range: &[i64; 2], b_range: &[i64; 2], c_range: &[i64; 2], x_range: &[i64; 2]) -> Self {
        Self {
            a: sample_nonzero(rng, a_range),
            b: sample_in(rng, b_range),
            c: sample_in(rng, c_range),
            k: sample_in(rng, x_range),
        }
    }

    fn value(&self) -> i64 {
        self.a * self.k * self.k + self.b * self.k + self.c
    }

    fn render(&self) -> Instance {
        Instance {
            content: format!(
                "\\lim_{{x \\to {}}} \\left( {} \\right)",
                self.k,
                quadratic(self.a, self.b, self.c)
            ),
            solution: self.value().to_string(),
            ..Default::default()
        }
    }
}

struct ChainRuleAtPoint {
    a: i64,
    b: i64,
    n: i64,
    k: i64,
}

impl ChainRuleAtPoint {
    fn sample(rng: &mut impl Rng, a_range: &[i64; 2], b_range: &[i64; 2], exponent_range: &[i64; 2], x_range: &[i64; 2]) -> Self {
        Self {
            a: sample_nonzero(rng, a_range),
            b: sample_in(rng, b_range),
            n: sample_in(rng, exponent_range).max(2),
            k: sample_in(rng, x_range),
        }
    }

    /// `d/dx (ax+b)^n` at `x=k` is `n·a·(ak+b)^(n-1)`.
    fn slope(&self) -> i64 {
        let inner = self.a * self.k + self.b;
        self.n * self.a * inner.pow((self.n - 1) as u32)
    }

    /// Verification: finite difference of `(ax+b)^n` at `k` matches the formula.
    #[cfg(test)]
    fn holds(&self) -> bool {
        let inner = |x: f64| self.a as f64 * x + self.b as f64;
        let f = |x: f64| inner(x).powi(self.n as i32);
        let k = self.k as f64;
        let h = 1e-4;
        let numeric = (f(k + h) - f(k - h)) / (2.0 * h);
        let analytic = self.slope() as f64;
        (numeric - analytic).abs() <= 1e-3 * analytic.abs().max(1.0)
    }

    fn render(&self) -> Instance {
        Instance {
            content: format!(
                "\\text{{Let }} f(x) = ({})^{{{}}}. \\text{{ Find }} f'({}).",
                linear_lhs(self.a, self.b),
                self.n,
                self.k
            ),
            solution: self.slope().to_string(),
            ..Default::default()
        }
    }
}

struct PartialWithCrossTerm {
    a: i64,
    b: i64,
    c: i64,
}

impl PartialWithCrossTerm {
    fn sample(rng: &mut impl Rng, a_range: &[i64; 2], b_range: &[i64; 2], c_range: &[i64; 2]) -> Self {
        Self {
            a: sample_nonzero(rng, a_range),
            b: sample_nonzero(rng, b_range),
            c: sample_nonzero(rng, c_range),
        }
    }

    /// Verification: `∂/∂x` at a fixed `(x0, y0)` matches `2a·x0 + b·y0`.
    #[cfg(test)]
    fn holds(&self) -> bool {
        let (x0, y0) = (1.3_f64, 0.7_f64);
        let f = |x: f64| self.a as f64 * x * x + self.b as f64 * x * y0 + self.c as f64 * y0 * y0;
        let h = 1e-4;
        let numeric = (f(x0 + h) - f(x0 - h)) / (2.0 * h);
        let analytic = 2.0 * self.a as f64 * x0 + self.b as f64 * y0;
        (numeric - analytic).abs() < 1e-3
    }

    fn render(&self) -> Instance {
        let mut solution = mono_coeff(2 * self.a, "x");
        // b ≠ 0 by construction, so there's always a y-term.
        solution.push_str(if self.b > 0 { " + " } else { " - " });
        solution.push_str(&mono_coeff(self.b.abs(), "y"));
        Instance {
            content: format!(
                "\\frac{{\\partial}}{{\\partial x}}\\left( {} \\right)",
                xy_quadratic(self.a, self.b, self.c)
            ),
            solution,
            ..Default::default()
        }
    }
}

struct ExponentQuotient {
    base: i64,
    m: i64,
    n: i64,
}

impl ExponentQuotient {
    fn sample(rng: &mut impl Rng, base_range: &[i64; 2], exponent_range: &[i64; 2]) -> Self {
        let base = sample_in(rng, base_range).max(2);
        let n = sample_in(rng, exponent_range).abs().max(1);
        let extra = sample_in(rng, exponent_range).abs().max(1);
        Self { base, m: n + extra, n }
    }

    /// Verification: `b^m == b^(m-n) · b^n`.
    #[cfg(test)]
    fn holds(&self) -> bool {
        self.base.pow(self.m as u32)
            == self.base.pow((self.m - self.n) as u32) * self.base.pow(self.n as u32)
    }

    fn render(&self) -> Instance {
        Instance {
            content: format!("\\frac{{{b}^{{{m}}}}}{{{b}^{{{n}}}}}", b = self.base, m = self.m, n = self.n),
            solution: format!("{}^{{{}}}", self.base, self.m - self.n),
            ..Default::default()
        }
    }
}

struct FunctionComposition {
    a: i64,
    b: i64,
    c: i64,
    d: i64,
    k: i64,
}

impl FunctionComposition {
    fn sample(rng: &mut impl Rng, a_range: &[i64; 2], b_range: &[i64; 2], c_range: &[i64; 2], d_range: &[i64; 2], x_range: &[i64; 2]) -> Self {
        Self {
            a: sample_nonzero(rng, a_range),
            b: sample_in(rng, b_range),
            c: sample_nonzero(rng, c_range),
            d: sample_in(rng, d_range),
            k: sample_in(rng, x_range),
        }
    }

    /// `f(g(k)) = a·(c·k + d) + b`.
    fn value(&self) -> i64 {
        self.a * (self.c * self.k + self.d) + self.b
    }

    fn render(&self) -> Instance {
        Instance {
            content: format!(
                "f(x) = {}, \\quad g(x) = {}. \\quad f(g({})) = \\;?",
                linear_lhs(self.a, self.b),
                linear_lhs(self.c, self.d),
                self.k
            ),
            solution: self.value().to_string(),
            ..Default::default()
        }
    }
}

struct FactorQuadratic {
    p: i64,
    q: i64,
}

impl FactorQuadratic {
    fn sample(rng: &mut impl Rng, root_range: &[i64; 2]) -> Self {
        let mut p = sample_nonzero(rng, root_range);
        let mut q = sample_nonzero(rng, root_range);
        if p > q {
            std::mem::swap(&mut p, &mut q);
        }
        Self { p, q }
    }

    /// Verification: `-p` and `-q` are roots of `x² + (p+q)x + pq`.
    #[cfg(test)]
    fn holds(&self) -> bool {
        let (s, prod) = (self.p + self.q, self.p * self.q);
        let eval = |x: i64| x * x + s * x + prod;
        eval(-self.p) == 0 && eval(-self.q) == 0
    }

    fn factor(root: i64) -> String {
        if root >= 0 {
            format!("(x + {root})")
        } else {
            format!("(x - {})", root.abs())
        }
    }

    fn render(&self) -> Instance {
        Instance {
            content: format!(
                "\\text{{Factor }} {}",
                quadratic(1, self.p + self.q, self.p * self.q)
            ),
            solution: format!("{}{}", Self::factor(self.p), Self::factor(self.q)),
            ..Default::default()
        }
    }
}

struct MatrixDeterminant2x2 {
    m: [[i64; 2]; 2],
}

impl MatrixDeterminant2x2 {
    fn sample(rng: &mut impl Rng, value_range: &[i64; 2]) -> Self {
        Self {
            m: [
                [sample_in(rng, value_range), sample_in(rng, value_range)],
                [sample_in(rng, value_range), sample_in(rng, value_range)],
            ],
        }
    }

    fn det(&self) -> i64 {
        self.m[0][0] * self.m[1][1] - self.m[0][1] * self.m[1][0]
    }

    fn render(&self) -> Instance {
        Instance {
            content: format!("\\det {} = \\;?", mat2(&self.m)),
            solution: self.det().to_string(),
            steps: vec![
                "For a 2×2 matrix, the determinant is (top-left × bottom-right) − (top-right × bottom-left).".to_string(),
                format!(
                    "$= ({})({}) - ({})({})$.",
                    self.m[0][0], self.m[1][1], self.m[0][1], self.m[1][0]
                ),
                format!(
                    "$= {} - {} = {}$.",
                    self.m[0][0] * self.m[1][1],
                    self.m[0][1] * self.m[1][0],
                    self.det()
                ),
            ],
            hints: vec!["The determinant of $\\begin{bmatrix} a & b \\\\ c & d \\end{bmatrix}$ is $ad - bc$.".to_string()],
        }
    }
}

struct LinearityOfExpectation {
    m: i64,
    a: i64,
    b: i64,
}

impl LinearityOfExpectation {
    fn sample(rng: &mut impl Rng, mean_range: &[i64; 2], a_range: &[i64; 2], b_range: &[i64; 2]) -> Self {
        Self {
            m: sample_in(rng, mean_range),
            a: sample_nonzero(rng, a_range),
            b: sample_in(rng, b_range),
        }
    }

    fn value(&self) -> i64 {
        self.a * self.m + self.b
    }

    fn render(&self) -> Instance {
        Instance {
            content: format!(
                "\\text{{If }} E[X] = {}, \\text{{ what is }} E[{}]?",
                self.m,
                linear_lhs(self.a, self.b)
            ),
            solution: self.value().to_string(),
            steps: vec![
                "Expectation is linear: $E[aX + b] = a\\,E[X] + b$.".to_string(),
                format!(
                    "$= ({})({}) + ({}) = {}$.",
                    self.a,
                    self.m,
                    self.b,
                    self.value()
                ),
            ],
            hints: vec!["Pull the constant out: $E[aX+b] = a\\,E[X] + b$.".to_string()],
        }
    }
}

struct VarianceScaling {
    v: i64,
    a: i64,
}

impl VarianceScaling {
    fn sample(rng: &mut impl Rng, var_range: &[i64; 2], a_range: &[i64; 2]) -> Self {
        Self {
            v: sample_in(rng, var_range).abs().max(1),
            a: sample_nonzero(rng, a_range),
        }
    }

    fn value(&self) -> i64 {
        self.a * self.a * self.v
    }

    fn render(&self) -> Instance {
        Instance {
            content: format!(
                "\\text{{If }} \\mathrm{{Var}}(X) = {}, \\text{{ what is }} \\mathrm{{Var}}({}X)?",
                self.v, self.a
            ),
            solution: self.value().to_string(),
            steps: vec![
                "Scaling by $a$ scales variance by $a^2$: $\\mathrm{Var}(aX) = a^2\\,\\mathrm{Var}(X)$.".to_string(),
                format!("$= ({})^2 \\cdot {} = {}$.", self.a, self.v, self.value()),
            ],
            hints: vec!["Variance scales by $a^2$ — the sign of $a$ doesn't matter.".to_string()],
        }
    }
}

struct VectorMagnitudeSquared {
    v: Vec<i64>,
}

impl VectorMagnitudeSquared {
    fn sample(rng: &mut impl Rng, dim: usize, value_range: &[i64; 2]) -> Self {
        let dim = dim.max(1);
        Self {
            v: (0..dim).map(|_| sample_in(rng, value_range)).collect(),
        }
    }

    fn value(&self) -> i64 {
        self.v.iter().map(|x| x * x).sum()
    }

    fn render(&self) -> Instance {
        Instance {
            content: format!(
                "v = {}, \\quad \\lVert v \\rVert^2 = \\;?",
                row_vec(&self.v)
            ),
            solution: self.value().to_string(),
            steps: vec![
                "The squared magnitude is the sum of the squared components.".to_string(),
                format!(
                    "$= {} = {}$.",
                    self.v.iter().map(|x| format!("({})^2", x)).collect::<Vec<_>>().join(" + "),
                    self.value()
                ),
            ],
            hints: vec!["$\\lVert v \\rVert^2 = v_1^2 + v_2^2 + \\cdots$ — no square root here.".to_string()],
        }
    }
}

struct UniformPmf {
    n: i64,
    k: i64,
}

impl UniformPmf {
    fn sample(rng: &mut impl Rng, sides_range: &[i64; 2]) -> Self {
        let n = sample_in(rng, sides_range).max(2);
        Self {
            n,
            k: rng.random_range(1..=n),
        }
    }

    fn render(&self) -> Instance {
        Instance {
            content: format!(
                "\\text{{A fair }} {n}\\text{{-sided die is rolled. }} P(X = {k}) = \\;?",
                n = self.n,
                k = self.k
            ),
            solution: format!("1/{}", self.n),
            steps: vec![
                "Every face of a fair die is equally likely.".to_string(),
                format!(
                    "With {n} equally likely outcomes, $P(X = {k}) = \\dfrac{{1}}{{{n}}}$.",
                    n = self.n,
                    k = self.k
                ),
            ],
            hints: vec!["A fair $n$-sided die gives each face probability $1/n$.".to_string()],
        }
    }
}

// --- Deep-vertical calculus instances ---

struct SecondDerivative {
    a: i64,
    b: i64,
    c: i64,
    d: i64,
}

impl SecondDerivative {
    fn sample(rng: &mut impl Rng, a_range: &[i64; 2], b_range: &[i64; 2], c_range: &[i64; 2], d_range: &[i64; 2]) -> Self {
        Self {
            a: sample_nonzero(rng, a_range),
            b: sample_in(rng, b_range),
            c: sample_in(rng, c_range),
            d: sample_in(rng, d_range),
        }
    }

    /// Verification: `d²/dx²` matches a second central difference of the cubic.
    #[cfg(test)]
    fn holds(&self) -> bool {
        let f = |x: f64| {
            self.a as f64 * x * x * x + self.b as f64 * x * x + self.c as f64 * x + self.d as f64
        };
        let f2 = |x: f64| 6.0 * self.a as f64 * x + 2.0 * self.b as f64;
        let h = 1e-3;
        [0.5_f64, 1.4, 2.1].iter().all(|&x| {
            let numeric = (f(x + h) - 2.0 * f(x) + f(x - h)) / (h * h);
            (numeric - f2(x)).abs() < 1e-1
        })
    }

    fn render(&self) -> Instance {
        Instance {
            content: format!(
                "\\frac{{d^2}}{{dx^2}}\\left( {} \\right)",
                cubic(self.a, self.b, self.c, self.d)
            ),
            solution: linear_lhs(6 * self.a, 2 * self.b),
            steps: vec![
                "Differentiate once (power rule term by term), then differentiate again.".to_string(),
                format!(
                    "First derivative: ${}$.",
                    // f'(x) = 3a x² + 2b x + c
                    quadratic(3 * self.a, 2 * self.b, self.c)
                ),
                format!("Differentiate again: ${}$.", linear_lhs(6 * self.a, 2 * self.b)),
            ],
            hints: vec!["Apply the power rule twice.".to_string()],
        }
    }
}

struct CriticalPoint {
    a: i64,
    b: i64,
    c: i64,
}

impl CriticalPoint {
    fn sample(rng: &mut impl Rng, a_range: &[i64; 2], b_range: &[i64; 2], c_range: &[i64; 2]) -> Self {
        Self {
            a: sample_nonzero(rng, a_range),
            b: sample_in(rng, b_range),
            c: sample_in(rng, c_range),
        }
    }

    /// `x = -b / (2a)`, reduced.
    fn point(&self) -> (i64, i64) {
        let (mut num, mut den) = (-self.b, 2 * self.a);
        if den < 0 {
            num = -num;
            den = -den;
        }
        let g = gcd(num, den).max(1);
        (num / g, den / g)
    }

    fn render(&self) -> Instance {
        let (num, den) = self.point();
        Instance {
            content: format!(
                "\\text{{Find the critical point of }} f(x) = {}. \\quad x = \\;?",
                quadratic(self.a, self.b, self.c)
            ),
            solution: fraction_str(num, den),
            steps: vec![
                "A critical point is where the derivative is zero.".to_string(),
                format!("$f'(x) = {} = 0$.", linear_lhs(2 * self.a, self.b)),
                format!("Solve for $x$: $x = {}$.", fraction_str(num, den)),
            ],
            hints: vec!["Set $f'(x) = 0$ and solve.".to_string()],
        }
    }
}

struct ExponentialDerivative {
    a: i64,
}

impl ExponentialDerivative {
    fn sample(rng: &mut impl Rng, coeff_range: &[i64; 2]) -> Self {
        Self {
            a: sample_nonzero(rng, coeff_range),
        }
    }

    /// `d/dx e^{ax} = a e^{ax}`.
    #[cfg(test)]
    fn holds(&self) -> bool {
        let f = |x: f64| (self.a as f64 * x).exp();
        let f_prime = |x: f64| self.a as f64 * (self.a as f64 * x).exp();
        let h = 1e-5;
        [0.1_f64, 0.4].iter().all(|&x| {
            let numeric = (f(x + h) - f(x - h)) / (2.0 * h);
            (numeric - f_prime(x)).abs() <= 1e-3 * f_prime(x).abs().max(1.0)
        })
    }

    fn render(&self) -> Instance {
        let exp = linear_lhs(self.a, 0); // "ax" / "x" / "-x"
        let coeff = match self.a {
            1 => String::new(),
            -1 => "-".to_string(),
            n => n.to_string(),
        };
        Instance {
            content: format!("\\frac{{d}}{{dx}}\\left( e^{{{}}} \\right)", exp),
            solution: format!("{coeff}e^{{{exp}}}"),
            steps: vec![
                "The derivative of $e^{u}$ is $e^{u}$ times the derivative of $u$ (chain rule)."
                    .to_string(),
                format!(
                    "Here $u = {}$, so $u' = {}$, giving ${}e^{{{}}}$.",
                    exp, self.a, coeff, exp
                ),
            ],
            hints: vec!["$\\frac{d}{dx}e^{ax} = a\\,e^{ax}$.".to_string()],
        }
    }
}

struct LogDerivative {
    a: i64,
}

impl LogDerivative {
    fn sample(rng: &mut impl Rng, coeff_range: &[i64; 2]) -> Self {
        Self {
            a: sample_in(rng, coeff_range).abs().max(1),
        }
    }

    /// `d/dx ln(ax) = 1/x` (for a>0).
    #[cfg(test)]
    fn holds(&self) -> bool {
        let f = |x: f64| (self.a as f64 * x).ln();
        let h = 1e-5;
        [0.5_f64, 1.7, 3.0].iter().all(|&x| {
            let numeric = (f(x + h) - f(x - h)) / (2.0 * h);
            (numeric - 1.0 / x).abs() < 1e-3
        })
    }

    fn render(&self) -> Instance {
        Instance {
            content: format!("\\frac{{d}}{{dx}}\\left( \\ln({}x) \\right)", self.a),
            solution: "1/x".to_string(),
            steps: vec![
                format!("Rewrite: $\\ln({}x) = \\ln {} + \\ln x$.", self.a, self.a),
                "The constant $\\ln a$ differentiates to 0, and $\\frac{d}{dx}\\ln x = \\frac{1}{x}$.".to_string(),
                "So the derivative is $\\dfrac{1}{x}$ — the coefficient $a$ doesn't matter.".to_string(),
            ],
            hints: vec!["$\\frac{d}{dx}\\ln(ax) = \\frac{a}{ax} = \\frac{1}{x}$.".to_string()],
        }
    }
}

struct QuadraticMinimum {
    a: i64,
    b: i64,
    c: i64,
}

impl QuadraticMinimum {
    fn sample(rng: &mut impl Rng, a_range: &[i64; 2], b_range: &[i64; 2], c_range: &[i64; 2]) -> Self {
        Self {
            // Upward parabola so a minimum exists.
            a: sample_in(rng, a_range).abs().max(1),
            b: sample_in(rng, b_range),
            c: sample_in(rng, c_range),
        }
    }

    /// Minimum value `(4ac − b²)/(4a)`, reduced.
    fn min_value(&self) -> (i64, i64) {
        let (num, den) = (4 * self.a * self.c - self.b * self.b, 4 * self.a);
        let g = gcd(num, den).max(1);
        (num / g, den / g)
    }

    fn render(&self) -> Instance {
        let (num, den) = self.min_value();
        Instance {
            content: format!(
                "\\text{{What is the minimum value of }} f(x) = {}?",
                quadratic(self.a, self.b, self.c)
            ),
            solution: fraction_str(num, den),
            steps: vec![
                "The minimum is at the vertex, where $f'(x) = 0$: $x = -\\dfrac{b}{2a}$.".to_string(),
                format!("Substitute back: the minimum value is $\\dfrac{{4ac - b^2}}{{4a}} = {}$.", fraction_str(num, den)),
            ],
            hints: vec!["Find where $f'(x)=0$, then evaluate $f$ there.".to_string()],
        }
    }
}

// --- Physics instances (answers include units) ---

struct AverageSpeed {
    speed: i64,
    time: i64,
}

impl AverageSpeed {
    fn sample(rng: &mut impl Rng, speed_range: &[i64; 2], time_range: &[i64; 2]) -> Self {
        Self {
            speed: sample_in(rng, speed_range).abs().max(1),
            time: sample_in(rng, time_range).abs().max(1),
        }
    }

    /// Distance is chosen as speed·time so the answer is exactly `speed`.
    fn distance(&self) -> i64 {
        self.speed * self.time
    }

    fn render(&self) -> Instance {
        Instance {
            content: format!(
                "\\text{{A car travels }} {d}\\,\\text{{m in }} {t}\\,\\text{{s. What is its average speed?}}",
                d = self.distance(),
                t = self.time
            ),
            solution: format!("{} m/s", self.speed),
            steps: vec![
                "Average speed is distance divided by time.".to_string(),
                format!(
                    "$\\dfrac{{{}\\,\\text{{m}}}}{{{}\\,\\text{{s}}}} = {}\\,\\text{{m/s}}$.",
                    self.distance(),
                    self.time,
                    self.speed
                ),
            ],
            hints: vec!["Use speed = distance ÷ time; keep the units.".to_string()],
        }
    }
}

struct AccelerationFromSpeed {
    accel: i64,
    time: i64,
}

impl AccelerationFromSpeed {
    fn sample(rng: &mut impl Rng, accel_range: &[i64; 2], time_range: &[i64; 2]) -> Self {
        Self {
            accel: sample_in(rng, accel_range).abs().max(1),
            time: sample_in(rng, time_range).abs().max(1),
        }
    }

    fn delta_v(&self) -> i64 {
        self.accel * self.time
    }

    fn render(&self) -> Instance {
        Instance {
            content: format!(
                "\\text{{A runner speeds up from rest to }} {dv}\\,\\text{{m/s in }} {t}\\,\\text{{s. What is the acceleration?}}",
                dv = self.delta_v(),
                t = self.time
            ),
            solution: format!("{} m/s^2", self.accel),
            ..Default::default()
        }
    }
}

struct FinalVelocity {
    u: i64,
    a: i64,
    t: i64,
}

impl FinalVelocity {
    fn sample(rng: &mut impl Rng, u_range: &[i64; 2], a_range: &[i64; 2], t_range: &[i64; 2]) -> Self {
        Self {
            u: sample_in(rng, u_range).abs(),
            a: sample_in(rng, a_range).abs().max(1),
            t: sample_in(rng, t_range).abs().max(1),
        }
    }

    fn final_v(&self) -> i64 {
        self.u + self.a * self.t
    }

    fn render(&self) -> Instance {
        Instance {
            content: format!(
                "\\text{{An object moving at }} {u}\\,\\text{{m/s accelerates at }} {a}\\,\\text{{m/s}}^2 \\text{{ for }} {t}\\,\\text{{s. What is its final velocity?}}",
                u = self.u,
                a = self.a,
                t = self.t
            ),
            solution: format!("{} m/s", self.final_v()),
            ..Default::default()
        }
    }
}

struct NewtonSecondLaw {
    mass: i64,
    accel: i64,
}

impl NewtonSecondLaw {
    fn sample(rng: &mut impl Rng, mass_range: &[i64; 2], accel_range: &[i64; 2]) -> Self {
        Self {
            mass: sample_in(rng, mass_range).abs().max(1),
            accel: sample_in(rng, accel_range).abs().max(1),
        }
    }

    fn force(&self) -> i64 {
        self.mass * self.accel
    }

    fn render(&self) -> Instance {
        Instance {
            content: format!(
                "\\text{{A net force accelerates a }} {m}\\,\\text{{kg mass at }} {a}\\,\\text{{m/s}}^2. \\text{{ What is the force?}}",
                m = self.mass,
                a = self.accel
            ),
            solution: format!("{} N", self.force()),
            ..Default::default()
        }
    }
}

struct Weight {
    mass: i64,
}

impl Weight {
    const G: f64 = 9.8;

    fn sample(rng: &mut impl Rng, mass_range: &[i64; 2]) -> Self {
        Self {
            mass: sample_in(rng, mass_range).abs().max(1),
        }
    }

    fn weight(&self) -> f64 {
        self.mass as f64 * Self::G
    }

    fn render(&self) -> Instance {
        Instance {
            content: format!(
                "\\text{{What is the weight of a }} {m}\\,\\text{{kg object }} (g = 9.8\\,\\text{{m/s}}^2)?",
                m = self.mass
            ),
            solution: format!("{} N", fmt_num(self.weight())),
            ..Default::default()
        }
    }
}

struct UnitConversion {
    km: i64,
}

impl UnitConversion {
    fn sample(rng: &mut impl Rng, value_range: &[i64; 2]) -> Self {
        Self {
            km: sample_in(rng, value_range).abs().max(1),
        }
    }

    fn metres(&self) -> i64 {
        self.km * 1000
    }

    fn render(&self) -> Instance {
        Instance {
            content: format!(
                "\\text{{Convert }} {km}\\,\\text{{km to metres.}}",
                km = self.km
            ),
            solution: format!("{} m", self.metres()),
            ..Default::default()
        }
    }
}

/// Format a float as a tidy answer: `98` not `98.0`, `4.9` not `4.9000`.
fn fmt_num(x: f64) -> String {
    if (x.fract()).abs() < 1e-9 {
        format!("{}", x.round() as i64)
    } else {
        let s = format!("{x:.4}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

// --- Sampling helpers ---

fn sample_in(rng: &mut impl Rng, range: &[i64; 2]) -> i64 {
    let (lo, hi) = (range[0].min(range[1]), range[0].max(range[1]));
    rng.random_range(lo..=hi)
}

/// Like [`sample_in`] but never returns zero — for coefficients that must not
/// collapse the problem (a leading `0` in `a·x + b = c` would be degenerate).
fn sample_nonzero(rng: &mut impl Rng, range: &[i64; 2]) -> i64 {
    let (lo, hi) = (range[0].min(range[1]), range[0].max(range[1]));
    if lo == 0 && hi == 0 {
        return 1; // guard against a degenerate authored range
    }
    loop {
        let v = rng.random_range(lo..=hi);
        if v != 0 {
            return v;
        }
    }
}

// --- LaTeX rendering helpers ---

/// `a·x + b`, with tidy signs and elided `1` coefficients.
fn linear_lhs(a: i64, b: i64) -> String {
    let ax = match a {
        1 => "x".to_string(),
        -1 => "-x".to_string(),
        _ => format!("{a}x"),
    };
    match b.cmp(&0) {
        std::cmp::Ordering::Equal => ax,
        std::cmp::Ordering::Greater => format!("{ax} + {b}"),
        std::cmp::Ordering::Less => format!("{ax} - {}", b.abs()),
    }
}

/// `coeff·x^exp`, with `^1` and unit coefficients elided.
fn monomial(coeff: i64, exp: i64) -> String {
    if coeff == 0 {
        return "0".to_string();
    }
    if exp == 0 {
        return coeff.to_string();
    }
    let body = if exp == 1 {
        "x".to_string()
    } else {
        format!("x^{{{exp}}}")
    };
    match coeff {
        1 => body,
        -1 => format!("-{body}"),
        _ => format!("{coeff}{body}"),
    }
}

fn row_vec(v: &[i64]) -> String {
    let entries = v.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(" & ");
    format!("\\begin{{bmatrix}} {entries} \\end{{bmatrix}}")
}

/// A column vector as a `bmatrix` (entries stacked with `\\`).
fn col_vec(v: &[i64]) -> String {
    let entries = v.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(" \\\\ ");
    format!("\\begin{{bmatrix}} {entries} \\end{{bmatrix}}")
}

/// A 2×2 matrix as a `bmatrix`.
fn mat2(m: &[[i64; 2]; 2]) -> String {
    format!(
        "\\begin{{bmatrix}} {} & {} \\\\ {} & {} \\end{{bmatrix}}",
        m[0][0], m[0][1], m[1][0], m[1][1]
    )
}

/// Plain comma-separated components, e.g. `3, -5` — the answer format for
/// vector-valued problems (kept paren-free so a lenient match accepts `[3,-5]`,
/// `(3, -5)`, etc.; see the V1 answer check in `lattice-service`).
fn components(v: &[i64]) -> String {
    v.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(", ")
}

/// `a·x² + b·x + c` with tidy signs and elided unit/zero terms (assumes a ≠ 0).
fn quadratic(a: i64, b: i64, c: i64) -> String {
    let mut out = monomial(a, 2);
    if b != 0 {
        out.push_str(if b > 0 { " + " } else { " - " });
        out.push_str(&monomial(b.abs(), 1));
    }
    if c != 0 {
        out.push_str(&format!(" {} {}", if c > 0 { "+" } else { "-" }, c.abs()));
    }
    out
}

/// `a·x³ + b·x² + c·x + d` with tidy signs and elided zero terms (assumes a ≠ 0).
fn cubic(a: i64, b: i64, c: i64, d: i64) -> String {
    let mut out = monomial(a, 3);
    for (coeff, exp) in [(b, 2), (c, 1), (d, 0)] {
        if coeff != 0 {
            out.push_str(if coeff > 0 { " + " } else { " - " });
            out.push_str(&monomial(coeff.abs(), exp));
        }
    }
    out
}

/// A coefficient glued to a body like `x` or `xy`, eliding unit coefficients:
/// `mono_coeff(3, "xy")` → `3xy`, `mono_coeff(1, "x")` → `x`, `-1` → `-x`.
fn mono_coeff(coeff: i64, body: &str) -> String {
    match coeff {
        1 => body.to_string(),
        -1 => format!("-{body}"),
        n => format!("{n}{body}"),
    }
}

/// `a·x² + b·xy + c·y²` with tidy signs and elided unit coefficients (a ≠ 0).
fn xy_quadratic(a: i64, b: i64, c: i64) -> String {
    let mut out = mono_coeff(a, "x^{2}");
    for (coeff, body) in [(b, "xy"), (c, "y^{2}")] {
        if coeff != 0 {
            out.push_str(if coeff > 0 { " + " } else { " - " });
            out.push_str(&mono_coeff(coeff.abs(), body));
        }
    }
    out
}

fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 {
        a.abs()
    } else {
        gcd(b, a % b)
    }
}

/// Binomial coefficient C(n, k) by exact integer arithmetic (small n only).
fn binomial(n: i64, k: i64) -> i64 {
    if k < 0 || k > n {
        return 0;
    }
    let k = k.min(n - k);
    let mut result = 1_i64;
    for i in 0..k {
        result = result * (n - i) / (i + 1);
    }
    result
}

/// Render a reduced fraction, collapsing `p/1` to just `p`.
fn fraction_str(p: i64, q: i64) -> String {
    if q == 1 {
        p.to_string()
    } else {
        format!("{p}/{q}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{rngs::StdRng, SeedableRng};

    fn rng() -> StdRng {
        StdRng::seed_from_u64(0xC0FFEE)
    }

    #[test]
    fn linear_equations_are_solvable_by_construction() {
        let mut r = rng();
        for _ in 0..2000 {
            let inst = LinearEq::sample(&mut r, &[-9, 9], &[-12, 12], &[-20, 20]);
            assert!(inst.a != 0, "leading coefficient must be nonzero");
            assert!(
                inst.holds(),
                "a={}, x={}, b={}, c={}",
                inst.a,
                inst.x,
                inst.b,
                inst.c
            );
        }
    }

    #[test]
    fn dot_products_recompute_consistently() {
        let mut r = rng();
        for _ in 0..2000 {
            let inst = DotProduct::sample(&mut r, 3, &[-6, 6]);
            assert!(inst.holds());
        }
    }

    #[test]
    fn power_rule_agrees_with_numeric_derivative() {
        let mut r = rng();
        for _ in 0..2000 {
            let inst = PowerRule::sample(&mut r, &[1, 6], &[2, 5]);
            assert!(inst.holds(), "a={}, n={}", inst.a, inst.n);
        }
    }

    #[test]
    fn renders_a_known_instance_exactly() {
        // a=2, x=3, b=1  =>  c = 7
        let t = Template {
            id: "t".into(),
            concept: ConceptId::new("algebraic_manipulation"),
            difficulty: Difficulty::Easy,
            kind: TemplateKind::LinearEquation {
                a_range: [2, 2],
                x_range: [3, 3],
                b_range: [1, 1],
            },
        };
        let p = t.generate(&SubjectId::new("math"), &mut rng());
        assert_eq!(p.content, "2x + 1 = 7");
        assert_eq!(p.solution, "x = 3");
        assert_eq!(p.generated_by, ProblemSource::Template);
        assert_eq!(p.concepts, vec![ConceptId::new("algebraic_manipulation")]);
        // Authored templates carry deterministic worked steps + hints through to
        // the Problem (the "teach when stuck" refactor), ending at the answer.
        assert!(!p.steps.is_empty() && !p.hints.is_empty());
        assert!(p.steps.last().unwrap().contains("x = 3"));
    }

    #[test]
    fn latex_helpers_elide_trivial_terms() {
        assert_eq!(linear_lhs(1, 0), "x");
        assert_eq!(linear_lhs(-1, -5), "-x - 5");
        assert_eq!(linear_lhs(3, 4), "3x + 4");
        assert_eq!(monomial(1, 2), "x^{2}");
        assert_eq!(monomial(-1, 1), "-x");
        assert_eq!(monomial(5, 0), "5");
    }

    #[test]
    fn arithmetic_eval_respects_precedence() {
        let mut r = rng();
        for _ in 0..1000 {
            let inst = ArithmeticEval::sample(&mut r, &[1, 20], &[2, 9], &[2, 9]);
            assert_eq!(inst.value(), inst.a + inst.b * inst.c);
        }
    }

    #[test]
    fn exponent_product_adds_exponents() {
        let mut r = rng();
        for _ in 0..1000 {
            let inst = ExponentProduct::sample(&mut r, &[2, 4], &[1, 4]);
            assert!(inst.holds());
        }
    }

    #[test]
    fn vector_sum_is_componentwise() {
        let mut r = rng();
        for _ in 0..1000 {
            let inst = VectorSum::sample(&mut r, 3, &[-6, 6]);
            let s = inst.sum();
            for i in 0..3 {
                assert_eq!(s[i], inst.u[i] + inst.v[i]);
            }
        }
    }

    #[test]
    fn matrix_vector_product_is_correct() {
        let mut r = rng();
        for _ in 0..1000 {
            let inst = MatrixVectorProduct::sample(&mut r, &[-5, 5]);
            let res = inst.result();
            assert_eq!(res[0], inst.m[0][0] * inst.v[0] + inst.m[0][1] * inst.v[1]);
            assert_eq!(res[1], inst.m[1][0] * inst.v[0] + inst.m[1][1] * inst.v[1]);
        }
    }

    #[test]
    fn simple_probability_is_a_reduced_proper_fraction() {
        let mut r = rng();
        for _ in 0..1000 {
            let inst = SimpleProbability::sample(&mut r, &[1, 9], &[1, 9]);
            let (p, q) = inst.fraction();
            assert!(p >= 1 && q > p, "{p}/{q}");
            assert_eq!(gcd(p, q), 1, "{p}/{q} not reduced");
            assert_eq!(p * (inst.red + inst.blue), q * inst.red);
        }
    }

    #[test]
    fn complement_probability_is_reduced_and_consistent() {
        let mut r = rng();
        for _ in 0..1000 {
            let inst = ComplementProbability::sample(&mut r, &[1, 9], &[1, 9]);
            let (p, q) = inst.fraction();
            // P(not red) = blue/total, reduced, and P(red) + P(not red) = 1.
            assert_eq!(gcd(p, q), 1, "{p}/{q} not reduced");
            assert_eq!(p * (inst.red + inst.blue), q * inst.blue);
            assert!(p <= q, "a probability must be ≤ 1");
        }
    }

    #[test]
    fn gradient3var_doubles_each_coefficient() {
        let mut r = rng();
        for _ in 0..1000 {
            let inst = Gradient3Var::sample(&mut r, &[2, 5], &[-3, 3]);
            assert!(inst.a != 0 && inst.b != 0 && inst.c != 0);
            assert_eq!(
                inst.grad(),
                [2 * inst.a * inst.x0, 2 * inst.b * inst.y0, 2 * inst.c * inst.z0]
            );
        }
    }

    #[test]
    fn expectation_uniform_mean_is_exact() {
        let mut r = rng();
        for _ in 0..1000 {
            let inst = ExpectationUniform::sample(&mut r, &[2, 12], &[-4, 4]);
            assert_eq!(inst.values.iter().sum::<i64>(), 3 * inst.mean);
        }
    }

    #[test]
    fn polynomial_derivative_matches_numeric() {
        let mut r = rng();
        for _ in 0..1000 {
            let inst = PolynomialDerivative::sample(&mut r, &[1, 5], &[-9, 9], &[-9, 9]);
            assert!(inst.holds(), "a={}, b={}, c={}", inst.a, inst.b, inst.c);
        }
    }

    #[test]
    fn partial_derivative_doubles_the_x_coefficient() {
        let mut r = rng();
        for _ in 0..1000 {
            let inst = PartialDerivative::sample(&mut r, &[2, 6], &[2, 6]);
            assert_eq!(inst.render().solution, format!("{}x", 2 * inst.a));
        }
    }

    #[test]
    fn conditional_probability_is_reduced() {
        let mut r = rng();
        for _ in 0..1000 {
            let inst = ConditionalProbability::sample(&mut r, &[2, 9]);
            let (p, q) = inst.fraction();
            assert!(inst.subset <= inst.total);
            assert_eq!(gcd(p, q), 1);
            assert_eq!(p * inst.total, q * inst.subset);
        }
    }

    #[test]
    fn bayes_posterior_is_consistent_and_reduced() {
        let mut r = rng();
        for _ in 0..1000 {
            let inst = BayesNaturalFrequency::sample(&mut r, &[1, 8]);
            assert!(inst.true_pos <= inst.diseased);
            assert!(inst.false_pos <= inst.healthy);
            let (p, q) = inst.fraction();
            assert_eq!(gcd(p, q), 1);
            assert_eq!(p * (inst.true_pos + inst.false_pos), q * inst.true_pos);
        }
    }

    #[test]
    fn variance_two_point_is_spread_squared() {
        let mut r = rng();
        for _ in 0..1000 {
            let inst = VarianceTwoPoint::sample(&mut r, &[0, 10], &[1, 6]);
            assert_eq!((inst.hi - inst.lo) % 2, 0);
            let k = (inst.hi - inst.lo) / 2;
            assert_eq!(inst.variance(), k * k);
        }
    }

    #[test]
    fn chain_rule_matches_numeric() {
        let mut r = rng();
        for _ in 0..1000 {
            let inst = ChainRule::sample(&mut r, &[1, 4], &[1, 6], &[2, 3]);
            assert!(inst.holds(), "a={}, b={}, n={}", inst.a, inst.b, inst.n);
        }
    }

    #[test]
    fn gradient_doubles_the_coefficients() {
        let mut r = rng();
        for _ in 0..1000 {
            let inst = Gradient::sample(&mut r, &[2, 5], &[-3, 3]);
            assert_eq!(inst.grad(), [2 * inst.a * inst.x0, 2 * inst.b * inst.y0]);
        }
    }

    #[test]
    fn function_eval_is_linear() {
        let mut r = rng();
        for _ in 0..1000 {
            let inst = FunctionEval::sample(&mut r, &[1, 6], &[-9, 9], &[-6, 6]);
            assert_eq!(inst.value(), inst.a * inst.x + inst.b);
        }
    }

    #[test]
    fn difference_of_squares_root_is_positive() {
        let mut r = rng();
        for _ in 0..1000 {
            let inst = DifferenceOfSquares::sample(&mut r, &[1, 9]);
            assert!(inst.root >= 1);
        }
    }

    #[test]
    fn removable_limit_is_twice_the_root() {
        let mut r = rng();
        for _ in 0..1000 {
            let inst = RemovableLimit::sample(&mut r, &[1, 9]);
            assert_eq!(inst.limit(), 2 * inst.root);
        }
    }

    #[test]
    fn matrix_trace_sums_the_diagonal() {
        let mut r = rng();
        for _ in 0..1000 {
            let inst = MatrixTrace::sample(&mut r, &[-6, 6]);
            assert_eq!(inst.trace(), inst.m[0][0] + inst.m[1][1]);
        }
    }

    #[test]
    fn matrix_multiply_is_correct() {
        let mut r = rng();
        for _ in 0..1000 {
            let inst = MatrixMultiply::sample(&mut r, &[-4, 4]);
            let p = inst.product();
            let (a, b) = (&inst.a, &inst.b);
            assert_eq!(p[0], a[0][0] * b[0][0] + a[0][1] * b[1][0]);
            assert_eq!(p[3], a[1][0] * b[0][1] + a[1][1] * b[1][1]);
        }
    }

    #[test]
    fn binomial_pmf_is_reduced() {
        let mut r = rng();
        for _ in 0..1000 {
            let inst = BinomialHeads::sample(&mut r, &[2, 4]);
            let (p, q) = inst.fraction();
            assert!(inst.k >= 0 && inst.k <= inst.n);
            assert_eq!(gcd(p, q), 1);
        }
    }

    #[test]
    fn gradient_descent_step_halves_x0() {
        let mut r = rng();
        for _ in 0..1000 {
            let inst = GradientDescentStep::sample(&mut r, &[1, 8]);
            assert_eq!(inst.x0 % 2, 0);
            assert_eq!(inst.next_x(), inst.x0 / 2);
        }
    }

    #[test]
    fn mle_coin_is_reduced() {
        let mut r = rng();
        for _ in 0..1000 {
            let inst = MleCoin::sample(&mut r, &[2, 9]);
            let (p, q) = inst.fraction();
            assert!(inst.h >= 1 && inst.h <= inst.n);
            assert_eq!(gcd(p, q), 1);
        }
    }

    #[test]
    fn derivative_forms_agree_with_numeric_derivatives() {
        let mut r = rng();
        for _ in 0..1000 {
            let cubic = CubicDerivative::sample(&mut r, &[1, 5], &[-6, 6], &[-9, 9], &[-9, 9]);
            assert!(cubic.holds(), "a={}, b={}, c={}", cubic.a, cubic.b, cubic.c);

            let prod = ProductRuleDerivative::sample(&mut r, &[1, 5], &[-6, 6], &[1, 5], &[-6, 6]);
            assert!(prod.holds(), "product rule mismatch");

            let tan = TangentLineSlope::sample(&mut r, &[1, 5], &[-9, 9], &[-9, 9], &[-4, 4]);
            assert_eq!(tan.slope(), 2 * tan.a * tan.k + tan.b);
        }
    }

    #[test]
    fn rolled_out_forms_are_correct() {
        let mut r = rng();
        for _ in 0..1000 {
            let lim = PolynomialLimit::sample(&mut r, &[1, 5], &[-9, 9], &[-9, 9], &[-4, 4]);
            assert_eq!(lim.value(), lim.a * lim.k * lim.k + lim.b * lim.k + lim.c);

            let chain = ChainRuleAtPoint::sample(&mut r, &[1, 3], &[-3, 3], &[2, 3], &[-2, 2]);
            assert!(chain.holds(), "chain@point a={} b={} n={} k={}", chain.a, chain.b, chain.n, chain.k);

            let part = PartialWithCrossTerm::sample(&mut r, &[1, 5], &[-6, 6], &[-6, 6]);
            assert!(part.holds(), "partial cross-term mismatch");

            let quot = ExponentQuotient::sample(&mut r, &[2, 4], &[1, 4]);
            assert!(quot.holds() && quot.m > quot.n);

            let comp = FunctionComposition::sample(&mut r, &[1, 5], &[-6, 6], &[1, 5], &[-6, 6], &[-4, 4]);
            assert_eq!(comp.value(), comp.a * (comp.c * comp.k + comp.d) + comp.b);

            let fac = FactorQuadratic::sample(&mut r, &[-7, 7]);
            assert!(fac.holds() && fac.p <= fac.q, "roots must be actual roots, sorted");

            let det = MatrixDeterminant2x2::sample(&mut r, &[-6, 6]);
            assert_eq!(det.det(), det.m[0][0] * det.m[1][1] - det.m[0][1] * det.m[1][0]);
        }
    }

    #[test]
    fn prob_and_vector_second_forms_are_correct() {
        let mut r = rng();
        for _ in 0..1000 {
            let ex = LinearityOfExpectation::sample(&mut r, &[-9, 9], &[1, 5], &[-9, 9]);
            assert_eq!(ex.value(), ex.a * ex.m + ex.b);

            let var = VarianceScaling::sample(&mut r, &[1, 12], &[-5, 5]);
            assert_eq!(var.value(), var.a * var.a * var.v);
            assert!(var.value() >= 0, "variance is never negative");

            let mag = VectorMagnitudeSquared::sample(&mut r, 3, &[-6, 6]);
            assert_eq!(mag.value(), mag.v.iter().map(|x| x * x).sum::<i64>());

            let pmf = UniformPmf::sample(&mut r, &[2, 12]);
            assert!(pmf.k >= 1 && pmf.k <= pmf.n);
            assert_eq!(pmf.render().solution, format!("1/{}", pmf.n));
        }
    }

    #[test]
    fn deep_vertical_forms_are_correct() {
        let mut r = rng();
        for _ in 0..1000 {
            let sd = SecondDerivative::sample(&mut r, &[1, 4], &[-6, 6], &[-9, 9], &[-9, 9]);
            assert!(sd.holds(), "second derivative mismatch");

            let cp = CriticalPoint::sample(&mut r, &[1, 5], &[-9, 9], &[-9, 9]);
            let (num, den) = cp.point();
            assert!(den > 0 && gcd(num, den) == 1, "critical point must be reduced");
            // -b/(2a) means 2a·x + b = 0  →  2a·num + b·den == 0.
            assert_eq!(2 * cp.a * num + cp.b * den, 0);

            let ed = ExponentialDerivative::sample(&mut r, &[-4, 4]);
            assert!(ed.holds(), "exp derivative mismatch");

            let ld = LogDerivative::sample(&mut r, &[1, 9]);
            assert!(ld.holds() && ld.render().solution == "1/x");

            let qm = QuadraticMinimum::sample(&mut r, &[1, 5], &[-9, 9], &[-9, 9]);
            let (num, den) = qm.min_value();
            assert!(den > 0 && gcd(num, den) == 1);
            // (4ac - b²)/(4a) reduced equals num/den.
            assert_eq!(num * (4 * qm.a), den * (4 * qm.a * qm.c - qm.b * qm.b));
        }
    }

    #[test]
    fn cubic_renders_tidily() {
        assert_eq!(cubic(1, 0, -3, 2), "x^{3} - 3x + 2");
        assert_eq!(cubic(2, -1, 0, 0), "2x^{3} - x^{2}");
    }

    #[test]
    fn physics_templates_are_correct_and_unit_graded() {
        use lattice_core::answers_match;
        let mut r = rng();
        for _ in 0..500 {
            let s = AverageSpeed::sample(&mut r, &[2, 30], &[2, 12]);
            assert_eq!(s.distance(), s.speed * s.time);
            let sol = s.render().solution;
            assert!(answers_match(&sol, &format!("{} m/s", s.speed)));
            // Right number, wrong unit must NOT grade as correct.
            assert!(!answers_match(&sol, &format!("{} m/s^2", s.speed)));

            let a = AccelerationFromSpeed::sample(&mut r, &[1, 8], &[2, 10]);
            assert_eq!(a.delta_v(), a.accel * a.time);
            assert!(answers_match(&a.render().solution, &format!("{} m/s^2", a.accel)));

            let fv = FinalVelocity::sample(&mut r, &[0, 20], &[1, 8], &[2, 8]);
            assert_eq!(fv.final_v(), fv.u + fv.a * fv.t);

            let n = NewtonSecondLaw::sample(&mut r, &[1, 20], &[1, 10]);
            assert_eq!(n.force(), n.mass * n.accel);
            assert!(answers_match(&n.render().solution, &format!("{} N", n.force())));

            let w = Weight::sample(&mut r, &[1, 50]);
            assert!((w.weight() - w.mass as f64 * 9.8).abs() < 1e-9);
            // The formatted answer grades against itself (unit round-trip).
            let wsol = w.render().solution;
            assert!(answers_match(&wsol, &wsol));

            let uc = UnitConversion::sample(&mut r, &[1, 20]);
            assert_eq!(uc.metres(), uc.km * 1000);
            assert!(answers_match(&uc.render().solution, &format!("{} m", uc.km * 1000)));
        }
    }

    #[test]
    fn fmt_num_is_tidy() {
        assert_eq!(fmt_num(98.0), "98");
        assert_eq!(fmt_num(29.4), "29.4");
        assert_eq!(fmt_num(49.0), "49");
    }

    #[test]
    fn vector_component_reads_the_right_entry() {
        let mut r = rng();
        for _ in 0..1000 {
            let inst = VectorComponent::sample(&mut r, 4, &[-9, 9]);
            assert!(inst.index >= 1 && inst.index <= inst.v.len());
            assert_eq!(inst.value(), inst.v[inst.index - 1]);
        }
    }
}
