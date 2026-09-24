use tracing::{info, warn};

pub const DEFAULT_FALLBACK_MODELS: &[&str] = &[
    "gemini-2.5-flash",
    "gemini-3.5-flash-lite",
    "gemini-3.1-flash-lite",
    "gemini-3.5-flash",
    "gemini-flash-latest",
    "gemini-flash-lite-latest",
];

#[derive(Debug, Clone)]
pub struct FallbackCoordinator {
    models: Vec<String>,
    current_index: usize,
}

impl FallbackCoordinator {
    pub fn new(primary_model: Option<&str>, fallback_models: Option<Vec<String>>) -> Self {
        let primary = primary_model.unwrap_or("gemini-2.5-flash");
        let mut models = vec![primary.to_string()];

        let defaults = match fallback_models {
            Some(custom) => custom,
            None => DEFAULT_FALLBACK_MODELS
                .iter()
                .map(|s| s.to_string())
                .collect(),
        };

        for model in defaults {
            if !models.contains(&model) {
                models.push(model);
            }
        }

        Self {
            models,
            current_index: 0,
        }
    }

    /// Current model index in fallback list
    pub fn current_index(&self) -> usize {
        self.current_index
    }

    /// Get current active model name
    pub fn current_model(&self) -> &str {
        &self.models[self.current_index]
    }

    /// Set a specific model if present in available list
    pub fn set_model(&mut self, model: &str) {
        if let Some(pos) = self.models.iter().position(|m| m == model) {
            self.current_index = pos;
        } else {
            self.models.insert(0, model.to_string());
            self.current_index = 0;
        }
    }

    /// Advance to the next fallback model in the list
    pub fn next_fallback(&mut self) -> Option<&str> {
        if self.current_index + 1 < self.models.len() {
            self.current_index += 1;
            let next = &self.models[self.current_index];
            warn!(
                "Switching to fallback model tier [{}/{}]: {}",
                self.current_index + 1,
                self.models.len(),
                next
            );
            Some(next)
        } else {
            None
        }
    }

    /// Reset fallback state back to primary tier
    pub fn reset(&mut self) {
        if self.current_index != 0 {
            info!("Resetting active model to primary tier: {}", self.models[0]);
            self.current_index = 0;
        }
    }

    /// All configured models in order
    pub fn all_models(&self) -> &[String] {
        &self.models
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_sequence() {
        let mut coordinator = FallbackCoordinator::new(Some("custom-model"), None);
        assert_eq!(coordinator.current_model(), "custom-model");

        let next = coordinator.next_fallback();
        assert!(next.is_some());
        assert_ne!(coordinator.current_model(), "custom-model");

        coordinator.reset();
        assert_eq!(coordinator.current_model(), "custom-model");
    }

    #[test]
    fn test_fallback_exhaustion() {
        let mut coordinator = FallbackCoordinator::new(
            Some("model-1"),
            Some(vec!["model-1".to_string(), "model-2".to_string()]),
        );
        assert_eq!(coordinator.current_model(), "model-1");
        assert_eq!(coordinator.next_fallback(), Some("model-2"));
        assert_eq!(coordinator.next_fallback(), None);
    }
}
