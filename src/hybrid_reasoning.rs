use std::collections::{HashMap, HashSet};

pub struct HybridReasoningEngine {
    // Symbolic knowledge base
    rules: HashMap<String, String>,
}

impl HybridReasoningEngine {
    pub fn new() -> Self {
        let mut rules = HashMap::new();
        rules.insert("cargo build".to_string(), "compile dependency first".to_string());
        rules.insert("cargo test".to_string(), "cargo build first".to_string());
        Self { rules }
    }

    /// Solves task dependency resolution symbolically using topological sort (deterministic)
    pub fn resolve_dependencies(&self, dependencies: HashMap<String, Vec<String>>) -> Result<Vec<String>, String> {
        let mut order = Vec::new();
        let mut visited = HashSet::new();
        let mut temp = HashSet::new();

        fn visit(
            node: &str,
            dependencies: &HashMap<String, Vec<String>>,
            visited: &mut HashSet<String>,
            temp: &mut HashSet<String>,
            order: &mut Vec<String>,
        ) -> Result<(), String> {
            if temp.contains(node) {
                return Err(format!("Cyclic dependency detected at: {}", node));
            }
            if !visited.contains(node) {
                temp.insert(node.to_string());
                if let Some(deps) = dependencies.get(node) {
                    for dep in deps {
                        visit(dep, dependencies, visited, temp, order)?;
                    }
                }
                temp.remove(node);
                visited.insert(node.to_string());
                order.push(node.to_string());
            }
            Ok(())
        }

        for node in dependencies.keys() {
            visit(node, &dependencies, &mut visited, &mut temp, &mut order)?;
        }

        Ok(order)
    }

    /// Evaluates if a query can be solved symbolically (deterministic) before falling back to LLM
    pub fn evaluate_hybrid(&self, query: &str) -> Option<String> {
        self.rules.get(query).cloned()
    }
}
