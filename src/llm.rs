use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum RoutingPolicy {
    CostAware,
    LatencyAware,
    TaskAware,
}

pub trait LlmProvider {
    fn name(&self) -> &str;
    fn get_cost_per_token(&self) -> f64;
    fn get_estimated_latency(&self) -> u32; // in ms
    fn generate_response(&self, prompt: &str) -> Result<String, String>;
}

pub struct GPT4Provider;
impl LlmProvider for GPT4Provider {
    fn name(&self) -> &str { "GPT-4" }
    fn get_cost_per_token(&self) -> f64 { 0.00003 }
    fn get_estimated_latency(&self) -> u32 { 1200 }
    fn generate_response(&self, prompt: &str) -> Result<String, String> {
        Ok(format!("GPT-4 response for prompt: '{}'", prompt))
    }
}

pub struct ClaudeProvider;
impl LlmProvider for ClaudeProvider {
    fn name(&self) -> &str { "Claude-3" }
    fn get_cost_per_token(&self) -> f64 { 0.000015 }
    fn get_estimated_latency(&self) -> u32 { 950 }
    fn generate_response(&self, prompt: &str) -> Result<String, String> {
        Ok(format!("Claude response for prompt: '{}'", prompt))
    }
}

pub struct GeminiProvider;
impl LlmProvider for GeminiProvider {
    fn name(&self) -> &str { "Gemini-Pro" }
    fn get_cost_per_token(&self) -> f64 { 0.000007 }
    fn get_estimated_latency(&self) -> u32 { 700 }
    fn generate_response(&self, prompt: &str) -> Result<String, String> {
        Ok(format!("Gemini response for prompt: '{}'", prompt))
    }
}

pub struct OllamaProvider;
impl LlmProvider for OllamaProvider {
    fn name(&self) -> &str { "Ollama-Local" }
    fn get_cost_per_token(&self) -> f64 { 0.0 }
    fn get_estimated_latency(&self) -> u32 { 300 }
    fn generate_response(&self, prompt: &str) -> Result<String, String> {
        Ok(format!("Ollama response for prompt: '{}'", prompt))
    }
}

pub struct LlmRouter {
    providers: Vec<Box<dyn LlmProvider + Send + Sync>>,
}

impl LlmRouter {
    pub fn new() -> Self {
        Self {
            providers: vec![
                Box::new(GPT4Provider),
                Box::new(ClaudeProvider),
                Box::new(GeminiProvider),
                Box::new(OllamaProvider),
            ],
        }
    }

    pub fn route(&self, prompt: &str, policy: RoutingPolicy) -> Result<String, String> {
        let mut ordered_providers = self.providers.iter().collect::<Vec<_>>();
        
        match policy {
            RoutingPolicy::CostAware => {
                ordered_providers.sort_by(|a, b| a.get_cost_per_token().partial_cmp(&b.get_cost_per_token()).unwrap());
            }
            RoutingPolicy::LatencyAware => {
                ordered_providers.sort_by_key(|p| p.get_estimated_latency());
            }
            RoutingPolicy::TaskAware => {
                // For complex tasks, prioritize high intelligence (GPT-4), otherwise local models
                if prompt.contains("compile") || prompt.contains("refactor") {
                    ordered_providers.sort_by(|a, b| b.get_cost_per_token().partial_cmp(&a.get_cost_per_token()).unwrap());
                } else {
                    ordered_providers.sort_by(|a, b| a.get_cost_per_token().partial_cmp(&b.get_cost_per_token()).unwrap());
                }
            }
        }

        // Try route with fallbacks
        let mut errors = Vec::new();
        for provider in ordered_providers {
            match provider.generate_response(prompt) {
                Ok(res) => {
                    return Ok(format!("[Routed via {}] {}", provider.name(), res));
                }
                Err(e) => {
                    errors.push(format!("Provider {}: {}", provider.name(), e));
                }
            }
        }

        Err(format!("All router LLM providers failed: {:?}", errors))
    }
}
