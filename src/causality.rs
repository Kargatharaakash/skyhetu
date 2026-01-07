//! Causality tracking for SkyHetu
//!
//! Records all state mutations with timestamps and values,
//! enabling the `why()` introspection function.

use std::collections::HashMap;
use crate::value::Value;
use std::time::Instant;

/// A single mutation event
#[derive(Debug, Clone)]
pub struct MutationEvent {
    /// Unique event ID
    pub id: usize,
    
    /// Name of the variable that was mutated
    pub variable: String,
    
    /// Value before mutation
    pub old_value: Value,
    
    /// Value after mutation
    pub new_value: Value,
    
    /// Logical timestamp (event order)
    pub timestamp: usize,
    
    /// Source location info
    pub location: Option<String>,
}

impl std::fmt::Display for MutationEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[#{}] {} : {} -> {}",
            self.id,
            self.variable,
            self.old_value,
            self.new_value
        )
    }
}

/// The causality log - tracks all state mutations
#[derive(Debug, Default)]
pub struct CausalityLog {
    /// All events in order
    events: Vec<MutationEvent>,
    
    /// Events indexed by variable name
    by_variable: HashMap<String, Vec<usize>>,
    
    /// Logical clock for event ordering
    clock: usize,
    
    /// Next event ID
    next_id: usize,
    
    /// Start time for relative timestamps
    _start: Option<Instant>,
}

impl CausalityLog {
    /// Create a new causality log
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            by_variable: HashMap::new(),
            clock: 0,
            next_id: 0,
            _start: Some(Instant::now()),
        }
    }
    
    /// Record a state mutation
    pub fn record_mutation(
        &mut self,
        variable: &str,
        old_value: Value,
        new_value: Value,
        location: Option<String>,
    ) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.clock += 1;
        
        let event = MutationEvent {
            id,
            variable: variable.to_string(),
            old_value,
            new_value,
            timestamp: self.clock,
            location,
        };
        
        // Store event
        self.events.push(event);
        
        // Index by variable
        self.by_variable
            .entry(variable.to_string())
            .or_default()
            .push(id);
        
        id
    }
    
    /// Get all mutation history for a variable
    pub fn history(&self, variable: &str) -> Vec<&MutationEvent> {
        self.by_variable
            .get(variable)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.events.get(*id))
                    .collect()
            })
            .unwrap_or_default()
    }
    
    /// Get all events in order
    pub fn all_events(&self) -> &[MutationEvent] {
        &self.events
    }
    
    /// Format the causality chain for a variable (for `why()` function)
    pub fn why(&self, variable: &str) -> String {
        let history = self.history(variable);
        
        if history.is_empty() {
            return format!("No state history for '{}'", variable);
        }
        
        let mut result = format!("Causality chain for '{}':\n", variable);
        
        for (i, event) in history.iter().enumerate() {
            result.push_str(&format!(
                "  {}. [t={}] {} -> {}\n",
                i + 1,
                event.timestamp,
                event.old_value,
                event.new_value
            ));
        }
        
        result
    }
    
    /// Get the current logical time
    pub fn current_time(&self) -> usize {
        self.clock
    }
    
    /// Clear all history
    pub fn clear(&mut self) {
        self.events.clear();
        self.by_variable.clear();
        self.clock = 0;
        self.next_id = 0;
    }
    
    /// Export causality chain for a variable as DOT format (Graphviz)
    pub fn to_dot(&self, variable: &str) -> String {
        let history = self.history(variable);
        
        if history.is_empty() {
            return format!("digraph {} {{\n  \"no_history\" [label=\"No history\"];\n}}\n", variable);
        }
        
        let mut dot = format!("digraph {} {{\n", variable);
        dot.push_str("  rankdir=LR;\n");
        dot.push_str("  node [shape=box];\n");
        
        // Create nodes for each state
        for (i, event) in history.iter().enumerate() {
            let value_str = format!("{}", event.new_value).replace("\"", "\\\"");
            if i == 0 {
                let old_str = format!("{}", event.old_value).replace("\"", "\\\"");
                dot.push_str(&format!("  s{} [label=\"{}\"];\n", i, old_str));
            }
            dot.push_str(&format!("  s{} [label=\"{}\"];\n", i + 1, value_str));
        }
        
        // Create edges
        for (i, event) in history.iter().enumerate() {
            dot.push_str(&format!("  s{} -> s{} [label=\"t={}\"];\n", i, i + 1, event.timestamp));
        }
        
        dot.push_str("}\n");
        dot
    }
    
    /// Export causality chain for a variable as JSON
    pub fn to_json(&self, variable: &str) -> String {
        let history = self.history(variable);
        
        if history.is_empty() {
            return format!("{{\"variable\":\"{}\",\"events\":[]}}", variable);
        }
        
        let mut json = format!("{{\"variable\":\"{}\",\"events\":[", variable);
        
        for (i, event) in history.iter().enumerate() {
            if i > 0 {
                json.push(',');
            }
            let old_str = format!("{}", event.old_value).replace("\"", "\\\"");
            let new_str = format!("{}", event.new_value).replace("\"", "\\\"");
            json.push_str(&format!(
                "{{\"id\":{},\"timestamp\":{},\"old\":\"{}\",\"new\":\"{}\"}}",
                event.id, event.timestamp, old_str, new_str
            ));
        }
        
        json.push_str("]}");
        json
    }
    
    /// Get state value at a specific timestamp (for replay)
    pub fn value_at(&self, variable: &str, timestamp: usize) -> Option<Value> {
        let history = self.history(variable);
        
        if history.is_empty() {
            return None;
        }
        
        // Find the last event at or before the timestamp
        let mut result = None;
        for event in &history {
            if event.timestamp <= timestamp {
                result = Some(event.new_value.clone());
            } else {
                break;
            }
        }
        
        // If no event found, return initial value
        if result.is_none() {
            if let Some(first) = history.first() {
                if first.timestamp > timestamp {
                    return Some(first.old_value.clone());
                }
            }
        }
        
        result
    }
    
    /// Get number of transitions for a variable
    pub fn transition_count(&self, variable: &str) -> usize {
        self.history(variable).len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_record_mutation() {
        let mut log = CausalityLog::new();
        
        log.record_mutation("x", Value::Number(0.0), Value::Number(1.0), None);
        log.record_mutation("x", Value::Number(1.0), Value::Number(2.0), None);
        
        let history = log.history("x");
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].old_value, Value::Number(0.0));
        assert_eq!(history[0].new_value, Value::Number(1.0));
        assert_eq!(history[1].old_value, Value::Number(1.0));
        assert_eq!(history[1].new_value, Value::Number(2.0));
    }
    
    #[test]
    fn test_why() {
        let mut log = CausalityLog::new();
        
        log.record_mutation("counter", Value::Number(0.0), Value::Number(1.0), None);
        log.record_mutation("counter", Value::Number(1.0), Value::Number(2.0), None);
        
        let why = log.why("counter");
        assert!(why.contains("Causality chain"));
        assert!(why.contains("0 -> 1"));
        assert!(why.contains("1 -> 2"));
    }
}
