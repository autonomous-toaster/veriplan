//! Structured LTL formula types.
//!
//! Replaces string-based LTL representation with enums that Kani can verify
//! by structural induction. The serialization to string is a thin leaf function.

/// Top-level LTL formula.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LtlFormula {
    /// `[] condition` — condition holds in every state (always/globally)
    Always(LtlCondition),
}

/// Boolean condition within an LTL formula.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LtlCondition {
    /// A single boolean variable (e.g., `active_t1_1`, `done_t1_1`, `failed_t1_1`)
    Atom(String),
    /// Negation: `!condition`
    Not(Box<LtlCondition>),
    /// Conjunction: `condition1 && condition2 && ...`
    And(Vec<LtlCondition>),
    /// Disjunction: `condition1 || condition2 || ...`
    Or(Vec<LtlCondition>),
    /// Implication: `antecedent -> consequent`
    Implies(Box<LtlCondition>, Box<LtlCondition>),
    /// Bidirectional implication: `left <-> right`
    Iff(Box<LtlCondition>, Box<LtlCondition>),
    /// Eventually: `<> condition` — condition holds in some future state
    Eventually(Box<LtlCondition>),
}

impl LtlFormula {
    /// Every atom name this formula mentions, in declaration order.
    ///
    /// A caller deciding what a formula can observe needs the atoms, not the rendering. Matching
    /// them against the rendered string is wrong in both directions: `failed_t1_1` is a prefix of
    /// `failed_t1_10`, so a substring test names a task the formula never mentioned.
    pub fn atoms(&self) -> Vec<&str> {
        match self {
            LtlFormula::Always(condition) => condition.atoms(),
        }
    }
}

impl LtlCondition {
    /// Every atom name this condition mentions, in declaration order.
    pub fn atoms(&self) -> Vec<&str> {
        match self {
            LtlCondition::Atom(name) => vec![name.as_str()],
            LtlCondition::Not(inner) | LtlCondition::Eventually(inner) => inner.atoms(),
            LtlCondition::And(terms) | LtlCondition::Or(terms) => {
                terms.iter().flat_map(LtlCondition::atoms).collect()
            }
            LtlCondition::Implies(left, right) | LtlCondition::Iff(left, right) => {
                left.atoms().into_iter().chain(right.atoms()).collect()
            }
        }
    }
}

/// Serialize an LtlFormula to the string format expected by SPIN and the BFS checker.
pub fn ltl_to_string(formula: &LtlFormula) -> String {
    match formula {
        LtlFormula::Always(cond) => format!("[] ( {} )", condition_to_string(cond)),
    }
}

pub fn condition_to_string(cond: &LtlCondition) -> String {
    match cond {
        LtlCondition::Atom(name) => name.clone(),
        LtlCondition::Not(inner) => format!("!({})", condition_to_string(inner)),
        LtlCondition::And(terms) => terms
            .iter()
            .map(condition_to_string)
            .collect::<Vec<_>>()
            .join(" && "),
        LtlCondition::Or(terms) => terms
            .iter()
            .map(condition_to_string)
            .collect::<Vec<_>>()
            .join(" || "),
        LtlCondition::Implies(a, b) => {
            format!("{} -> {}", condition_to_string(a), condition_to_string(b))
        }
        LtlCondition::Iff(a, b) => {
            format!("{} <-> {}", condition_to_string(a), condition_to_string(b))
        }
        LtlCondition::Eventually(inner) => {
            format!("<> {}", condition_to_string(inner))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ltl_to_string_always_atom() {
        let f = LtlFormula::Always(LtlCondition::Atom("active_t1_1".into()));
        assert_eq!(ltl_to_string(&f), "[] ( active_t1_1 )");
    }

    #[test]
    fn test_ltl_to_string_always_implies() {
        let f = LtlFormula::Always(LtlCondition::Implies(
            Box::new(LtlCondition::Atom("active_t1_2".into())),
            Box::new(LtlCondition::Atom("done_t1_1".into())),
        ));
        assert_eq!(ltl_to_string(&f), "[] ( active_t1_2 -> done_t1_1 )");
    }

    #[test]
    fn test_ltl_to_string_always_iff() {
        let f = LtlFormula::Always(LtlCondition::Iff(
            Box::new(LtlCondition::Atom("active_t1_1".into())),
            Box::new(LtlCondition::Atom("active_t1_2".into())),
        ));
        assert_eq!(ltl_to_string(&f), "[] ( active_t1_1 <-> active_t1_2 )");
    }

    #[test]
    fn test_ltl_to_string_always_not_and() {
        let f = LtlFormula::Always(LtlCondition::Not(Box::new(LtlCondition::And(vec![
            LtlCondition::Atom("active_t1_1".into()),
            LtlCondition::Atom("active_t1_2".into()),
        ]))));
        assert_eq!(ltl_to_string(&f), "[] ( !(active_t1_1 && active_t1_2) )");
    }

    #[test]
    fn test_ltl_to_string_eventually() {
        let f = LtlFormula::Always(LtlCondition::Eventually(Box::new(LtlCondition::Atom(
            "active_t2_1".into(),
        ))));
        assert_eq!(ltl_to_string(&f), "[] ( <> active_t2_1 )");
    }

    /// Every variant is reached, including one nested two levels deep: a variant added later
    /// fails to compile rather than being silently skipped.
    #[test]
    fn the_atom_walk_reaches_every_variant() {
        let f = LtlFormula::Always(LtlCondition::And(vec![
            LtlCondition::Atom("a".into()),
            LtlCondition::Not(Box::new(LtlCondition::Or(vec![
                LtlCondition::Atom("b".into()),
                LtlCondition::Implies(
                    Box::new(LtlCondition::Atom("c".into())),
                    Box::new(LtlCondition::Eventually(Box::new(LtlCondition::Atom(
                        "d".into(),
                    )))),
                ),
            ]))),
            LtlCondition::Iff(
                Box::new(LtlCondition::Atom("e".into())),
                Box::new(LtlCondition::Atom("f".into())),
            ),
        ]));
        assert_eq!(f.atoms(), vec!["a", "b", "c", "d", "e", "f"]);
    }

    /// A rendering is not a set of names: this is the prefix case that a substring test gets wrong.
    #[test]
    fn the_atom_walk_does_not_report_a_prefix_of_another_atom() {
        let f = LtlFormula::Always(LtlCondition::Atom("failed_t1_10".into()));
        assert_eq!(f.atoms(), vec!["failed_t1_10"]);
        assert!(!f.atoms().contains(&"failed_t1_1"));
    }

    #[test]
    fn test_ltl_to_string_always_and() {
        let f = LtlFormula::Always(LtlCondition::And(vec![
            LtlCondition::Atom("x".into()),
            LtlCondition::Atom("y".into()),
        ]));
        assert_eq!(ltl_to_string(&f), "[] ( x && y )");
    }
}
