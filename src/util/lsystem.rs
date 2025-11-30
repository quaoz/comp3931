use crate::util::rng::random_bool;

/// Whether a symbol is subject to rewriting
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum SymbolType {
    Terminal,
    NonTerminal,
}

pub trait Symbol: PartialEq + Copy + Clone {
    fn symbol_type(&self) -> SymbolType;
}

/// A single production rule
///
/// - `Normal(head, rhs)`
/// - `Stochastic(head, prob, rhs)`
/// - `Parametric(head, f)`
pub enum Rule<'a, S: Symbol> {
    Normal(S, &'a [S]),
    Stochastic(S, f32, &'a [S]),
    Parametric(S, &'a dyn Fn(&S, &mut Vec<S>) -> bool),
}

impl<S: Symbol> Rule<'_, S> {
    fn priority(&self) -> u8 {
        match self {
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
