use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EvalScenario {
    pub scenario_name: String,
    pub query: String,
    pub expected_output: String,
    pub expected_tools: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EvalResult {
    pub scenario_name: String,
    pub passed: bool,
    pub similarity_score: f64,
    pub tools_matched: bool,
}

#[derive(Clone)]
pub struct EvaluationHarness {
    results: Arc<RwLock<Vec<EvalResult>>>,
}

impl EvaluationHarness {
    pub fn new() -> Self {
        Self {
            results: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn run_evaluation(&self, scenarios: Vec<EvalScenario>) -> Vec<EvalResult> {
        let mut suite_results = Vec::new();
        
        for sc in scenarios {
            // Mocking execution of the query and trace capture
            let actual_output = format!("Mock output for query: {}", sc.query);
            let actual_tools = sc.expected_tools.clone(); // mock tool match

            // Calculate similarity score (mock string matching heuristics)
            let mut score = 0.5;
            if actual_output.contains(&sc.expected_output) || sc.expected_output.contains(&actual_output) {
                score = 1.0;
            } else if sc.query.contains("test") {
                score = 0.9;
            }

            let tools_matched = actual_tools == sc.expected_tools;
            let passed = score >= 0.8 && tools_matched;

            let res = EvalResult {
                scenario_name: sc.scenario_name.clone(),
                passed,
                similarity_score: score,
                tools_matched,
            };
            suite_results.push(res);
        }

        let mut write = self.results.write().unwrap();
        *write = suite_results.clone();
        suite_results
    }

    pub fn check_regression_gate(&self, threshold: f64) -> Result<String, String> {
        let read = self.results.read().unwrap();
        if read.is_empty() {
            return Err("No evaluation results available. Run evaluation first.".to_string());
        }

        let total_score: f64 = read.iter().map(|r| r.similarity_score).sum();
        let avg_score = total_score / read.len() as f64;

        if avg_score < threshold {
            Err(format!(
                "Regression Gate Failure: Average similarity score '{:.2}' is below target threshold '{:.2}'", 
                avg_score, threshold
            ))
        } else {
            Ok(format!("Regression Gate Passed: Average similarity score is {:.2}", avg_score))
        }
    }
}
