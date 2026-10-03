use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Grammar {
    pub axiom: String,
    pub iterations: usize,
    pub angle: f64,
    pub step: f64,

    #[serde(default = "default_branch_probability")]
    pub branch_probability: f64,

    pub rules: HashMap<String, Vec<Production>>,
}

fn default_branch_probability() -> f64 {
    1.0
}

#[derive(Debug, Deserialize, Clone)]
pub struct Production {
    pub weight: f64,
    pub replacement: String,
}

impl Grammar {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let source = fs::read_to_string(path)?;
        let grammar: Grammar = toml::from_str(&source)?;

        grammar.validate()?;

        Ok(grammar)
    }

    fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.axiom.is_empty() {
            return Err("axiom must not be empty".into());
        }

        if self.iterations == 0 {
            return Err("iterations must be greater than zero".into());
        }

        if self.step <= 0.0 {
            return Err("step must be greater than zero".into());
        }

        if !(0.0..=1.0).contains(&self.branch_probability) {
            return Err("branch_probability must be between 0 and 1".into());
        }

        for (symbol, productions) in &self.rules {
            if symbol.chars().count() != 1 {
                return Err(format!(
                    "rule key {:?} must contain exactly one character",
                    symbol
                )
                .into());
            }

            if productions.is_empty() {
                return Err(format!("rule {:?} has no productions", symbol).into());
            }

            let total_weight: f64 = productions.iter().map(|p| p.weight).sum();

            if total_weight <= 0.0 {
                return Err(format!("rule {:?} has no positive weights", symbol).into());
            }
        }

        Ok(())
    }
}
