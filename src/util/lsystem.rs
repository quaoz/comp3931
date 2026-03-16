use std::fmt::Display;

use crate::util::rng::random_bool;

/// Whether a symbol is subject to rewriting
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum SymbolType {
    Terminal,
    NonTerminal,
}

/// How a symbol participates in context-sensitive rule matching
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum ContextAction {
    Ignore,
    Consider,
    BranchStart,
    BranchEnd,
}

pub trait Symbol: PartialEq + Copy + Clone {
    fn symbol_type(&self) -> SymbolType;

    fn context(&self) -> ContextAction {
        ContextAction::Consider
    }
}

// Returns `true` if the `target` symbol is the nearest `Consider` symbol to the left of `index`
fn path_left<S: Symbol>(input: &[S], index: usize, target: S) -> bool {
    let mut skip = 0;

    for i in (0..index).rev() {
        let ctx = input[i].context();

        if ctx == ContextAction::BranchEnd {
            skip += 1
        } else if ctx == ContextAction::BranchStart && skip > 0 {
            skip -= 1;
        } else if ctx == ContextAction::Consider && skip == 0 {
            return input[i] == target;
        }
    }

    false
}

/// Returns `true` if the `target` symbol appears as a reachable `Consider` symbol to the right of `index`
fn tree_right<S: Symbol>(input: &[S], index: usize, target: S) -> bool {
    let mut skip: Option<usize> = None;
    let mut depth = 0;

    for s in input.iter().skip(index + 1) {
        let ctx = s.context();

        if ctx == ContextAction::BranchStart {
            depth += 1;
        } else if ctx == ContextAction::BranchEnd {
            depth -= 1;

            if let Some(skip_depth) = skip
                && skip_depth == depth
            {
                skip = None;
            }
        } else if ctx == ContextAction::Consider && skip.is_none() {
            if *s == target {
                return true;
            }
            if depth == 0 {
                return false;
            }
            // skip remainder of branch + any subbranches
            skip = Some(depth - 1);
        }
    }

    false
}

/// A single production rule
///
/// - `Normal(head, rhs)`
/// - `Stochastic(head, prob, rhs)`
/// - `Parametric(head, f)`
/// - `ContextSensitive(head, left, right, rhs)`
pub enum Rule<'a, S: Symbol> {
    Normal(S, &'a [S]),
    Stochastic(S, f32, &'a [S]),
    Parametric(S, &'a dyn Fn(&S, &mut Vec<S>) -> bool),
    ContextSensitive(S, Option<S>, Option<S>, &'a [S]),
}

impl<S: Symbol> Rule<'_, S> {
    fn priority(&self) -> u8 {
        match self {
            Rule::ContextSensitive(..) => 0,
            Rule::Parametric(..) => 1,
            Rule::Stochastic(..) => 2,
            Rule::Normal(..) => 3,
        }
    }

    /// Attempts to apply this rule to the symbol at `index` in `input`. On match,
    /// appends the production to `out` and returns `true`
    pub fn apply(&self, index: usize, input: &[S], out: &mut Vec<S>) -> bool {
        let symbol = input[index];

        if symbol.symbol_type() != SymbolType::NonTerminal {
            return false;
        }

        match self {
            Rule::ContextSensitive(head, left, right, res)
                if symbol == *head
                    && (left.is_some() || right.is_some())
                    && left.is_none_or(|l| path_left(input, index, l))
                    && right.is_none_or(|r| tree_right(input, index, r)) =>
            {
                out.extend(*res);
                true
            }
            Rule::Parametric(head, func) if symbol == *head => func(&symbol, out),
            Rule::Stochastic(head, prob, res) if symbol == *head && random_bool(*prob as f64) => {
                out.extend(*res);
                true
            }
            Rule::Normal(head, res) if symbol == *head => {
                out.extend(*res);
                true
            }
            _ => false,
        }
    }
}

pub struct LSystem<'a, S: Symbol> {
    state: Vec<S>,
    rules: Vec<Rule<'a, S>>,
}

impl<'a, S: Symbol> LSystem<'a, S> {
    /// Construct an L-system from an axiom and set of rules
    pub fn new(axiom: &[S], mut rules: Vec<Rule<'a, S>>) -> Self {
        rules.sort_by_key(|r| r.priority());
        Self {
            state: axiom.to_vec(),
            rules,
        }
    }

    /// Return the current string as an immutable slice
    pub fn current(&self) -> &[S] {
        &self.state
    }

    /// Apply one rewriting pass
    pub fn step(&mut self) {
        let current = std::mem::take(&mut self.state);
        let mut next = Vec::with_capacity(current.len() * 2);

        for (i, &sym) in current.iter().enumerate() {
            // apply() writes directly into `next`, avoiding per-symbol allocations
            if !self.rules.iter().any(|r| r.apply(i, &current, &mut next)) {
                next.push(sym);
            }
        }
        self.state = next;
    }

    // Apply `iterations` rewriting passes
    pub fn evolve(&mut self, iterations: usize) {
        for _ in 0..iterations {
            self.step();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Symbol properties:
    //   A          NonTerminal  Consider     — visible as context, can be rewritten
    //   B, C       NonTerminal  Ignore       — can be rewritten, invisible to context search
    //   D          Terminal     Consider     — visible as context, passes through unrewritten
    //   E          Terminal     Ignore       — passes through unrewritten, invisible to context
    //   F(u32)     NonTerminal  Consider     — parametric, visible as context (discriminant match)
    //   G(u32)     NonTerminal  Ignore       — parametric, invisible to context search
    //   Push / Pop Terminal     BranchStart / BranchEnd — delimit branches

    #[derive(Debug, Copy, Clone)]
    enum TestSymbols {
        A,
        B,
        C,
        D,
        E,
        F(u32),
        G(u32),
        Push,
        Pop,
    }

    impl Display for TestSymbols {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::A => write!(f, "A"),
                Self::B => write!(f, "B"),
                Self::C => write!(f, "C"),
                Self::D => write!(f, "D"),
                Self::E => write!(f, "E"),
                Self::F(x) => write!(f, "F({x})"),
                Self::G(x) => write!(f, "G({x})"),
                Self::Push => write!(f, "["),
                Self::Pop => write!(f, "]"),
            }
        }
    }

    impl PartialEq for TestSymbols {
        fn eq(&self, other: &Self) -> bool {
            std::mem::discriminant(self) == std::mem::discriminant(other)
        }
    }

    impl Symbol for TestSymbols {
        fn symbol_type(&self) -> SymbolType {
            match self {
                Self::D | Self::E | Self::Pop | Self::Push => SymbolType::Terminal,
                _ => SymbolType::NonTerminal,
            }
        }

        fn context(&self) -> ContextAction {
            match self {
                Self::A | Self::D | Self::F(_) => ContextAction::Consider,
                Self::Push => ContextAction::BranchStart,
                Self::Pop => ContextAction::BranchEnd,
                _ => ContextAction::Ignore,
            }
        }
    }

    use TestSymbols::*;

    fn ts_str(syms: &[TestSymbols]) -> String {
        syms.iter()
            .map(|s| format!("{s}"))
            .collect::<Vec<_>>()
            .join("")
    }

    // ── Normal rules ──

    /// Classic algae growth
    #[test]
    fn algae() {
        let mut ls = LSystem::new(&[A], vec![Rule::Normal(A, &[A, B]), Rule::Normal(B, &[A])]);
        let expected = ["A", "AB", "ABA", "ABAAB", "ABAABABA"];
        for exp in expected {
            assert_eq!(ts_str(ls.current()), exp);
            ls.step();
        }
    }

    /// Terminal symbols are never rewritten and pass through unchanged
    #[test]
    fn terminal_passes_through() {
        let mut ls = LSystem::new(&[A, D, B], vec![
            Rule::Normal(A, &[A, B]),
            Rule::Normal(B, &[A]),
        ]);
        ls.step();
        assert_eq!(ts_str(ls.current()), "ABDA");
    }

    /// A NonTerminal with no matching rule passes through unrewritten
    #[test]
    fn nonterminal_without_rule_passes_through() {
        let mut ls = LSystem::new(&[A, C], vec![Rule::Normal(A, &[B])]);
        ls.step();
        assert_eq!(ts_str(ls.current()), "BC");
    }

    // ── Stochastic rules ──

    #[test]
    fn stochastic_always_fires_at_prob_1() {
        let mut ls = LSystem::new(&[A], vec![Rule::Stochastic(A, 1.0, &[B])]);
        ls.step();
        assert_eq!(ls.current(), &[B]);
    }

    #[test]
    fn stochastic_never_fires_at_prob_0() {
        let mut ls = LSystem::new(&[A], vec![Rule::Stochastic(A, 0.0, &[B])]);
        ls.step();
        assert_eq!(ls.current(), &[A]);
    }

    /// When the stochastic rule misses a lower-priority Normal rule fires instead
    #[test]
    fn stochastic_falls_back_to_normal() {
        let mut ls = LSystem::new(&[A], vec![
            Rule::Stochastic(A, 0.0, &[C]),
            Rule::Normal(A, &[B]),
        ]);
        ls.step();
        assert_eq!(ls.current(), &[B]);
    }

    // ── Context-sensitive rules: basic left / right matching ──

    /// Both left and right contexts present and matching
    #[test]
    fn context_left_and_right_match() {
        let mut ls = LSystem::new(&[A, B, A], vec![Rule::ContextSensitive(
            B,
            Some(A),
            Some(A),
            &[C],
        )]);
        ls.step();
        assert_eq!(ts_str(ls.current()), "ACA");
    }

    /// Left context only: fires when left neighbour matches
    #[test]
    fn context_left_only() {
        let mut ls = LSystem::new(&[A, B], vec![Rule::ContextSensitive(B, Some(A), None, &[
            C,
        ])]);
        ls.step();
        assert_eq!(ts_str(ls.current()), "AC");
    }

    /// Right context only: fires when right neighbour matches
    #[test]
    fn context_right_only() {
        let mut ls = LSystem::new(&[B, A], vec![Rule::ContextSensitive(B, None, Some(A), &[
            C,
        ])]);
        ls.step();
        assert_eq!(ts_str(ls.current()), "CA");
    }

    /// A different Consider symbol on the left prevents the rule from firing
    #[test]
    fn context_no_match_wrong_left() {
        let mut ls = LSystem::new(&[D, B, A], vec![Rule::ContextSensitive(
            B,
            Some(A),
            None,
            &[C],
        )]);
        ls.step();
        assert_eq!(ts_str(ls.current()), "DBA");
    }

    /// A non-matching Consider symbol at depth 0 on the right terminates the search
    #[test]
    fn context_no_match_wrong_right() {
        let mut ls = LSystem::new(&[A, B, D], vec![Rule::ContextSensitive(
            B,
            None,
            Some(A),
            &[C],
        )]);
        ls.step();
        assert_eq!(ts_str(ls.current()), "ABD");
    }

    #[test]
    fn context_no_match_at_left_edge() {
        let mut ls = LSystem::new(&[B, A], vec![Rule::ContextSensitive(B, Some(A), None, &[
            C,
        ])]);
        ls.step();
        assert_eq!(ts_str(ls.current()), "BA");
    }

    // ── Context-sensitive rules: Ignore symbols and branch traversal ──

    /// Symbols with ContextAction::Ignore are transparent to path_left
    #[test]
    fn context_left_skips_ignored_symbols() {
        let mut ls = LSystem::new(&[A, E, B, A], vec![Rule::ContextSensitive(
            B,
            Some(A),
            Some(A),
            &[C],
        )]);
        ls.step();
        assert_eq!(ts_str(ls.current()), "AECA");
    }

    /// path_left skips over entire bracketed sub-expressions when walking towards the root
    #[test]
    fn context_left_skips_branches() {
        let mut ls = LSystem::new(&[A, Push, D, Pop, B, A], vec![Rule::ContextSensitive(
            B,
            Some(A),
            Some(A),
            &[C],
        )]);
        ls.step();
        assert_eq!(ts_str(ls.current()), "A[D]CA");
    }

    /// tree_right finds a Consider symbol even when it is inside a bracketed branch
    #[test]
    fn context_right_finds_symbol_in_branch() {
        // A B [A]: A inside the branch satisfies the right context.
        let mut ls = LSystem::new(&[A, B, Push, A, Pop], vec![Rule::ContextSensitive(
            B,
            Some(A),
            Some(A),
            &[C],
        )]);
        ls.step();
        assert_eq!(ts_str(ls.current()), "AC[A]");
    }

    /// tree_right searches all sibling branches
    #[test]
    fn context_right_searches_multiple_branches() {
        let mut ls = LSystem::new(&[B, Push, D, Pop, Push, A, Pop], vec![
            Rule::ContextSensitive(B, None, Some(A), &[C]),
        ]);
        ls.step();
        assert_eq!(ts_str(ls.current()), "C[D][A]");
    }

    #[test]
    fn context_right_nested_branch_skipped_after_mismatch() {
        let mut ls = LSystem::new(&[B, Push, D, Push, A, Pop, Pop], vec![
            Rule::ContextSensitive(B, None, Some(A), &[C]),
        ]);
        ls.step();
        assert_eq!(ts_str(ls.current()), "B[D[A]]");
    }

    // ── Parametric rules ──

    /// Parametric rule receives the symbol's inline value and can base its production on it
    #[test]
    fn parametric_reads_inline_value() {
        let mut ls = LSystem::new(&[F(0)], vec![Rule::Parametric(F(0), &|s: &TestSymbols,
                                                                         out: &mut Vec<
            TestSymbols,
        >| {
            if let &F(n) = s {
                out.push(F(n + 1));
                true
            } else {
                false
            }
        })]);
        ls.step();
        assert_eq!(ls.current(), &[F(1)]);
        ls.step();
        assert_eq!(ls.current(), &[F(2)]);
    }

    /// When no condition in a Parametric rule matches, the symbol passes through
    #[test]
    fn parametric_conditional_countdown() {
        let mut ls = LSystem::new(&[F(3)], vec![Rule::Parametric(F(0), &|s: &TestSymbols,
                                                                         out: &mut Vec<
            TestSymbols,
        >| {
            if let &F(n) = s
                && n > 0
            {
                out.push(F(n - 1));
                true
            } else {
                false
            }
        })]);
        ls.step();
        assert_eq!(ls.current(), &[F(2)]);
        ls.step();
        assert_eq!(ls.current(), &[F(1)]);
        ls.step();
        assert_eq!(ls.current(), &[F(0)]);
        ls.step();
        assert_eq!(ls.current(), &[F(0)]);
    }

    #[test]
    fn parametric_context_matches_by_discriminant() {
        let mut ls = LSystem::new(&[F(5), A], vec![Rule::ContextSensitive(
            A,
            Some(F(0)),
            None,
            &[B],
        )]);
        ls.step();
        assert_eq!(ts_str(ls.current()), "F(5)B");
    }

    #[test]
    fn parametric_ignored_symbol_does_not_satisfy_context() {
        let mut ls = LSystem::new(&[G(3), A], vec![Rule::ContextSensitive(
            A,
            Some(F(0)),
            None,
            &[B],
        )]);
        ls.step();
        assert_eq!(ts_str(ls.current()), "G(3)A");
    }

    // ── Rule priority ──

    /// ContextSensitive beats Normal when the context is satisfied
    #[test]
    fn context_sensitive_has_priority_over_normal() {
        let mut ls = LSystem::new(&[A, B, A], vec![
            Rule::Normal(B, &[E]),
            Rule::ContextSensitive(B, Some(A), Some(A), &[C]),
        ]);
        ls.step();
        assert_eq!(ts_str(ls.current()), "ACA");
    }

    /// ContextSensitive beats Parametric when the context is satisfied
    #[test]
    fn context_sensitive_has_priority_over_parametric() {
        let mut ls = LSystem::new(&[A, B], vec![
            Rule::Parametric(B, &|_: &TestSymbols, out: &mut Vec<TestSymbols>| {
                out.push(E);
                true
            }),
            Rule::ContextSensitive(B, Some(A), None, &[C]),
        ]);
        ls.step();
        assert_eq!(ts_str(ls.current()), "AC");
    }

    /// ContextSensitive beats Stochastic when the context is satisfied
    #[test]
    fn context_sensitive_has_priority_over_stochastic() {
        let mut ls = LSystem::new(&[A, B, A], vec![
            Rule::Stochastic(B, 1.0, &[E]),
            Rule::ContextSensitive(B, Some(A), Some(A), &[C]),
        ]);
        ls.step();
        assert_eq!(ts_str(ls.current()), "ACA");
    }

    /// When context is not satisfied, a lower-priority Normal rule triggers
    #[test]
    fn normal_fallback_when_context_unmatched() {
        let mut ls = LSystem::new(&[D, B], vec![
            Rule::Normal(B, &[E]),
            Rule::ContextSensitive(B, Some(A), None, &[C]),
        ]);
        ls.step();
        assert_eq!(ts_str(ls.current()), "DE");
    }

    // ── evolve / step ──

    /// evolve(0) leaves the axiom state unchanged
    #[test]
    fn evolve_zero_steps_unchanged() {
        let mut ls = LSystem::new(&[A, B, A], vec![
            Rule::Normal(A, &[B]),
            Rule::Normal(B, &[A]),
        ]);
        ls.evolve(0);
        assert_eq!(ts_str(ls.current()), "ABA");
    }

    /// evolve(n) produces the same state as n sequential calls to step()
    #[test]
    fn evolve_matches_repeated_step() {
        let make = || LSystem::new(&[A], vec![Rule::Normal(A, &[A, B]), Rule::Normal(B, &[A])]);
        let mut by_step = make();
        for _ in 0..5 {
            by_step.step();
        }
        let mut by_evolve = make();
        by_evolve.evolve(5);
        assert_eq!(by_step.current(), by_evolve.current());
    }
}
