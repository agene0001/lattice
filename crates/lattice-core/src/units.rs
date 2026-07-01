//! Units-aware answer comparison — the foundation for grading physics (and
//! quantitative chemistry) alongside math.
//!
//! A physics answer is a **physical quantity**: a number *and* a unit. Grading
//! them as bare strings or bare numbers is wrong in both directions — it rejects
//! `9.8 m/s^2` vs `9.80 m/s²` (formatting) and accepts `9.8 J` for a `9.8 m/s^2`
//! answer (dimensionally nonsense). So we parse each side into a value plus a
//! **dimension vector** over the seven SI base dimensions, canonicalizing units
//! to a common scale, and compare value *and* dimension.
//!
//! This is intentionally not a full CAS (that's the SymPy follow-up in spec
//! §10.4); it covers the compound units physics answers actually use — `m/s^2`,
//! `kg*m/s^2` = `N`, `km` = `1000 m`, `1 min` = `60 s` — and returns `None` for
//! anything without a recognizable unit, so unitless math answers fall straight
//! through to the existing numeric/substring logic with no change in behavior.

/// Exponents over the seven SI base dimensions:
/// `[mass, length, time, current, temperature, amount, luminous]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dimension(pub [i8; 7]);

impl Dimension {
    pub const ZERO: Dimension = Dimension([0; 7]);
    const MASS: Dimension = Dimension([1, 0, 0, 0, 0, 0, 0]);
    const LENGTH: Dimension = Dimension([0, 1, 0, 0, 0, 0, 0]);
    const TIME: Dimension = Dimension([0, 0, 1, 0, 0, 0, 0]);
    const CURRENT: Dimension = Dimension([0, 0, 0, 1, 0, 0, 0]);
    const TEMP: Dimension = Dimension([0, 0, 0, 0, 1, 0, 0]);
    const AMOUNT: Dimension = Dimension([0, 0, 0, 0, 0, 1, 0]);
    const LUMINOUS: Dimension = Dimension([0, 0, 0, 0, 0, 0, 1]);

    /// `self + other * exp`, componentwise — accumulate a factor raised to `exp`.
    fn add_scaled(self, other: Dimension, exp: i8) -> Dimension {
        let mut out = self.0;
        for i in 0..7 {
            out[i] += other.0[i] * exp;
        }
        Dimension(out)
    }
}

/// A parsed physical quantity: value expressed in SI base units, plus dimension.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quantity {
    pub value: f64,
    pub dim: Dimension,
}

/// Default relative tolerance for comparing quantity magnitudes (~1%), forgiving
/// of sig-fig rounding (`9.8` vs `9.81`) but not of genuinely different answers.
pub const DEFAULT_REL_TOL: f64 = 1e-2;

/// Parse a string into a [`Quantity`], or `None` if it carries no recognizable
/// unit (a bare number is *not* a quantity here — it belongs to the numeric path).
///
/// The **rightmost** number-with-a-unit wins, so prose around the answer is fine
/// (`"the speed is 20 m/s"` → `20 m/s`).
pub fn parse_quantity(s: &str) -> Option<Quantity> {
    let cleaned = clean(s);
    let b = cleaned.as_bytes();

    // Every index that begins a fresh number (a digit not preceded by an
    // alphanumeric or a dot — i.e. not mid-number and not a unit suffix like `s2`).
    let mut starts: Vec<usize> = Vec::new();
    for i in 0..b.len() {
        let prev_joined = i > 0 && (b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'.');
        if b[i].is_ascii_digit() && !prev_joined {
            starts.push(i);
        }
    }

    for &start in starts.iter().rev() {
        // Pull in a leading sign if it's right before the number.
        let from = if start > 0 && (b[start - 1] == b'-' || b[start - 1] == b'+') {
            start - 1
        } else {
            start
        };
        let Some((value, unit_expr)) = split_number(&cleaned[from..]) else {
            continue;
        };
        let Some((scale, dim)) = parse_unit_expr(&unit_expr) else {
            continue;
        };
        // Dimensionless-and-unscaled is "just a number" — keep looking / let the
        // caller fall through to the numeric comparison.
        if dim == Dimension::ZERO && (scale - 1.0).abs() < f64::EPSILON {
            continue;
        }
        return Some(Quantity {
            value: value * scale,
            dim,
        });
    }
    None
}

/// Two quantities match iff same dimension and magnitudes within `rel_tol`.
pub fn quantities_match(a: &Quantity, b: &Quantity, rel_tol: f64) -> bool {
    if a.dim != b.dim {
        return false;
    }
    let diff = (a.value - b.value).abs();
    let scale = a.value.abs().max(b.value.abs());
    diff <= rel_tol * scale || diff < 1e-12
}

// --- parsing ---

/// Strip LaTeX wrappers and normalize unicode so unit expressions parse:
/// `9.8\,\text{m/s}^2` and `9.8 m/s²` both become `9.8 m/s^2`.
fn clean(s: &str) -> String {
    let mut t = s.to_string();
    for (from, to) in [
        ("\\text{", ""),
        ("\\mathrm{", ""),
        ("\\rm{", ""),
        ("\\operatorname{", ""),
        ("\\,", " "),
        ("\\;", " "),
        ("\\:", " "),
        ("\\ ", " "),
        ("\\!", ""),
        ("\\cdot", "*"),
        ("\\times", "*"),
    ] {
        t = t.replace(from, to);
    }
    for (from, to) in [
        ("·", "*"),
        ("×", "*"),
        ("÷", "/"),
        ("−", "-"),
        ("μ", "u"),
        ("µ", "u"),
        ("Ω", "ohm"),
        ("⁻¹", "^-1"),
        ("⁻²", "^-2"),
        ("⁻³", "^-3"),
        ("¹", "^1"),
        ("²", "^2"),
        ("³", "^3"),
    ] {
        t = t.replace(from, to);
    }
    t.chars()
        .filter(|c| !matches!(c, '\\' | '{' | '}' | '$'))
        .collect()
}

/// Split a leading number (incl. sign and scientific notation) off the front,
/// returning `(value, rest)`. `None` if there's no leading number.
fn split_number(s: &str) -> Option<(f64, String)> {
    let s = s.trim();
    let b = s.as_bytes();
    let mut i = 0;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        i += 1;
    }
    let digits_start = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    if i < b.len() && b[i] == b'.' {
        i += 1;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
    }
    // Require at least one digit.
    if s[digits_start..i].bytes().all(|c| !c.is_ascii_digit()) {
        return None;
    }
    // Optional scientific exponent.
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        let mut j = i + 1;
        if j < b.len() && (b[j] == b'+' || b[j] == b'-') {
            j += 1;
        }
        let exp_start = j;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        if j > exp_start {
            i = j;
        }
    }
    let value: f64 = s[..i].parse().ok()?;
    Some((value, s[i..].trim().to_string()))
}

/// Parse a compound unit expression (`kg*m/s^2`, `J/(kg*K)`) into a combined
/// `(scale, dimension)`. Parentheses are flattened; `/` puts every following
/// group in the denominator.
fn parse_unit_expr(expr: &str) -> Option<(f64, Dimension)> {
    let expr: String = expr.chars().filter(|c| *c != '(' && *c != ')').collect();
    let expr = expr.trim();
    if expr.is_empty() {
        return Some((1.0, Dimension::ZERO));
    }
    let mut scale = 1.0_f64;
    let mut dim = Dimension::ZERO;
    for (group_idx, group) in expr.split('/').enumerate() {
        let sign: i8 = if group_idx == 0 { 1 } else { -1 };
        for factor in group.split(|c: char| c == '*' || c.is_whitespace()) {
            if factor.is_empty() {
                continue;
            }
            let (name, exp) = split_power(factor)?;
            let (s, d) = resolve_unit(name)?;
            let e = sign * exp;
            scale *= s.powi(e as i32);
            dim = dim.add_scaled(d, e);
        }
    }
    Some((scale, dim))
}

/// Split `s^2` → `("s", 2)`, `m` → `("m", 1)`, `m^-1` → `("m", -1)`.
fn split_power(factor: &str) -> Option<(&str, i8)> {
    match factor.split_once('^') {
        Some((name, exp)) => Some((name, exp.parse().ok()?)),
        None => Some((factor, 1)),
    }
}

/// Resolve a unit token (possibly SI-prefixed) to `(scale_to_SI, dimension)`.
fn resolve_unit(token: &str) -> Option<(f64, Dimension)> {
    if let Some(u) = base_unit(token) {
        return Some(u);
    }
    // Try SI prefixes, longest first so `da` beats `d`.
    for (prefix, factor) in PREFIXES {
        if let Some(rest) = token.strip_prefix(prefix) {
            if !rest.is_empty() {
                if let Some((s, d)) = base_unit(rest) {
                    return Some((factor * s, d));
                }
            }
        }
    }
    None
}

const PREFIXES: &[(&str, f64)] = &[
    ("da", 1e1),
    ("Y", 1e24),
    ("Z", 1e21),
    ("E", 1e18),
    ("P", 1e15),
    ("T", 1e12),
    ("G", 1e9),
    ("M", 1e6),
    ("k", 1e3),
    ("h", 1e2),
    ("d", 1e-1),
    ("c", 1e-2),
    ("m", 1e-3),
    ("u", 1e-6),
    ("n", 1e-9),
    ("p", 1e-12),
    ("f", 1e-15),
];

/// Known units → `(scale_to_SI, dimension)`. Exact matches win over prefixing,
/// so `min` is minutes (not milli-inches) and `kg` is a kilogram directly.
fn base_unit(name: &str) -> Option<(f64, Dimension)> {
    use Dimension as D;
    let q = |scale: f64, dim: Dimension| Some((scale, dim));
    match name {
        // mass — gram is the prefixable base; kg is given explicitly.
        "g" => q(1e-3, D::MASS),
        "kg" => q(1.0, D::MASS),
        // length
        "m" => q(1.0, D::LENGTH),
        // time
        "s" | "sec" => q(1.0, D::TIME),
        "min" => q(60.0, D::TIME),
        "h" | "hr" => q(3600.0, D::TIME),
        // electric current / temperature / amount / luminous
        "A" => q(1.0, D::CURRENT),
        "K" => q(1.0, D::TEMP),
        "mol" => q(1.0, D::AMOUNT),
        "cd" => q(1.0, D::LUMINOUS),
        // derived
        "N" => q(1.0, D([1, 1, -2, 0, 0, 0, 0])),      // kg·m/s²
        "J" => q(1.0, D([1, 2, -2, 0, 0, 0, 0])),      // kg·m²/s²
        "W" => q(1.0, D([1, 2, -3, 0, 0, 0, 0])),      // J/s
        "Pa" => q(1.0, D([1, -1, -2, 0, 0, 0, 0])),    // N/m²
        "Hz" => q(1.0, D([0, 0, -1, 0, 0, 0, 0])),     // 1/s
        "V" => q(1.0, D([1, 2, -3, -1, 0, 0, 0])),     // kg·m²/(s³·A)
        "ohm" => q(1.0, D([1, 2, -3, -2, 0, 0, 0])),   // V/A
        "L" => q(1e-3, D([0, 3, 0, 0, 0, 0, 0])),      // litre = 1e-3 m³
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matches(a: &str, b: &str) -> bool {
        match (parse_quantity(a), parse_quantity(b)) {
            (Some(x), Some(y)) => quantities_match(&x, &y, DEFAULT_REL_TOL),
            _ => false,
        }
    }

    #[test]
    fn formatting_and_sigfigs_are_ignored() {
        assert!(matches("9.8 m/s^2", "9.80 m/s^2"));
        assert!(matches("9.8 m/s^2", "9.8 m/s²")); // unicode superscript
        assert!(matches("9.8 m/s^2", "9.81 m/s^2")); // within 1%
    }

    #[test]
    fn unit_conversions_hold() {
        assert!(matches("1 m", "100 cm"));
        assert!(matches("1 km", "1000 m"));
        assert!(matches("1 min", "60 s"));
        assert!(matches("5 kg", "5000 g"));
    }

    #[test]
    fn derived_units_canonicalize() {
        // 1 N is exactly 1 kg·m/s².
        assert!(matches("1 N", "1 kg*m/s^2"));
        assert!(matches("1 J", "1 N*m"));
    }

    #[test]
    fn wrong_dimensions_do_not_match() {
        assert!(!matches("5 J", "5 N")); // energy vs force
        assert!(!matches("5 kg", "5 g")); // 5 kg is not 5 g
        assert!(!matches("1 m", "1 s"));
    }

    #[test]
    fn bare_numbers_are_not_quantities() {
        assert!(parse_quantity("9.8").is_none());
        assert!(parse_quantity("3").is_none());
        assert!(parse_quantity("1/2").is_none());
        // ...but a number with a unit is.
        assert!(parse_quantity("9.8 m/s^2").is_some());
    }

    #[test]
    fn latex_wrapped_units_parse() {
        assert!(matches("9.8\\,\\text{m/s}^2", "9.80 m/s^2"));
    }
}
