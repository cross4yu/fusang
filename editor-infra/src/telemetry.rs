use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TelemetryEvent {
    pub name: String,
    pub properties: HashMap<String, String>,
    pub metrics: HashMap<String, f64>,
}

pub struct Telemetry {
    enabled: bool,
    session_id: String,
}

impl Telemetry {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            session_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    pub fn track_event(&self, event: TelemetryEvent) {
        if !self.enabled {
            return;
        }

        // In a real implementation, this would send to a telemetry service
        tracing::debug!(
            "Telemetry event: {} (session: {})",
            event.name,
            self.session_id
        );
    }

    pub fn track_editor_action(&self, action: &str, duration_ms: f64) {
        let event = TelemetryEvent {
            name: "editor_action".to_string(),
            properties: HashMap::from([("action".to_string(), action.to_string())]),
            metrics: HashMap::from([("duration_ms".to_string(), duration_ms)]),
        };
        self.track_event(event);
    }

    pub fn track_ai_completion(&self, model: &str, tokens_used: usize) {
        let event = TelemetryEvent {
            name: "ai_completion".to_string(),
            properties: HashMap::from([("model".to_string(), model.to_string())]),
            metrics: HashMap::from([("tokens_used".to_string(), tokens_used as f64)]),
        };
        self.track_event(event);
    }
}

impl Default for Telemetry {
    fn default() -> Self {
        Self::new(true)
    }
}