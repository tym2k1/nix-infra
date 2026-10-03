use crate::grammar::{Grammar, Production};
use rand::Rng;
use rand::RngExt;

pub fn expand<R: Rng + ?Sized>(
    grammar: &Grammar,
    rng: &mut R,
) -> String {
    let mut current = grammar.axiom.clone();

    for _ in 0..grammar.iterations {
        let mut next = String::new();

        for symbol in current.chars() {
            if let Some(productions) = grammar.rules.get(&symbol.to_string()) {
                let replacement = choose_production(productions, rng);
                next.push_str(&replacement);
            } else {
                next.push(symbol);
            }
        }

        current = next;
    }

    current
}

fn choose_production<R: Rng + ?Sized>(
    productions: &[Production],
    rng: &mut R,
) -> String {
    let total: f64 = productions
        .iter()
        .map(|p| p.weight.max(0.0))
        .sum();

    let mut choice = rng.random_range(0.0..total);

    for production in productions {
        let weight = production.weight.max(0.0);

        if choice < weight {
            return production.replacement.clone();
        }

        choice -= weight;
    }

    productions
        .last()
        .expect("productions must not be empty")
        .replacement
        .clone()
}
