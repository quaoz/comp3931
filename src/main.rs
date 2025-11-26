use std::fmt::Display;

use crate::util::lsystem::{LSystem, Rule, Symbol, SymbolType};

mod util;

#[derive(Debug, Copy, Clone)]
enum TestSymbols {
    A,
    B,
}

impl Display for TestSymbols {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::A => write!(f, "A"),
            Self::B => write!(f, "B"),
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
        SymbolType::NonTerminal
    }
}

fn ts_str(syms: &[TestSymbols]) -> String {
    syms.iter()
        .map(|s| format!("{s}"))
        .collect::<Vec<_>>()
        .join("")
}

fn main() -> anyhow::Result<()> {
    let mut ls = LSystem::new(&[TestSymbols::A], vec![
        Rule::Normal(TestSymbols::A, &[TestSymbols::A, TestSymbols::B]),
        Rule::Normal(TestSymbols::B, &[TestSymbols::A]),
    ]);
    let expected = [
        "A",
        "AB",
        "ABA",
        "ABAAB",
        "ABAABABA",
        "ABAABABAABAAB",
        "ABAABABAABAABABAABABA",
        "ABAABABAABAABABAABABAABAABABAABAAB",
    ];

    for exp in expected {
        let s = ts_str(ls.current());
        println!("{s}");
        assert_eq!(s, exp);
        ls.step();
    }

    Ok(())
}
