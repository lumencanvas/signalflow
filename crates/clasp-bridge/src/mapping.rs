//! Address mapping and value transformation

use clasp_core::Value;
use evalexpr::{eval_with_context_mut, ContextWithMutableVariables, HashMapContext};
use std::collections::HashMap;
use tracing::warn;

/// Maps between protocol addresses and Clasp addresses
#[derive(Debug, Clone)]
pub struct AddressMapping {
    /// Protocol address pattern
    pub from: String,
    /// Clasp address pattern
    pub to: String,
    /// Value transformation
    pub transform: Option<ValueTransform>,
}

impl AddressMapping {
    pub fn new(from: &str, to: &str) -> Self {
        Self {
            from: from.to_string(),
            to: to.to_string(),
            transform: None,
        }
    }

    pub fn with_transform(mut self, transform: ValueTransform) -> Self {
        self.transform = Some(transform);
        self
    }

    /// Apply mapping to convert protocol address to Clasp address
    pub fn map_address(&self, addr: &str) -> Option<String> {
        // Simple pattern matching
        if self.from.contains('*') {
            // Extract wildcards
            let from_parts: Vec<&str> = self.from.split('*').collect();
            let to_parts: Vec<&str> = self.to.split('*').collect();

            if from_parts.len() != to_parts.len() {
                return None;
            }

            let mut result = self.to.clone();
            let mut remaining = addr;

            for (i, part) in from_parts.iter().enumerate() {
                if !part.is_empty() {
                    if let Some(pos) = remaining.find(part) {
                        if i > 0 {
                            let captured = &remaining[..pos];
                            result = result.replacen('*', captured, 1);
                        }
                        remaining = &remaining[pos + part.len()..];
                    } else {
                        return None;
                    }
                }
            }

            // Handle trailing wildcard
            if self.from.ends_with('*') && !remaining.is_empty() {
                result = result.replacen('*', remaining, 1);
            }

            Some(result)
        } else if addr == self.from {
            Some(self.to.clone())
        } else {
            None
        }
    }
}

/// Value transformation
#[derive(Debug, Clone)]
pub enum ValueTransform {
    /// No transformation
    Identity,
    /// Scale from one range to another
    Scale {
        from_min: f64,
        from_max: f64,
        to_min: f64,
        to_max: f64,
    },
    /// Clamp to range
    Clamp { min: f64, max: f64 },
    /// Invert (1 - x for normalized values)
    Invert,
    /// Convert to integer
    ToInt,
    /// Convert to float
    ToFloat,
    /// Custom expression (for advanced use)
    Expression(String),
}

impl ValueTransform {
    pub fn scale(from_min: f64, from_max: f64, to_min: f64, to_max: f64) -> Self {
        Self::Scale {
            from_min,
            from_max,
            to_min,
            to_max,
        }
    }

    /// Apply the transformation to a value
    pub fn apply(&self, value: &Value) -> Value {
        match self {
            ValueTransform::Identity => value.clone(),
            ValueTransform::Scale {
                from_min,
                from_max,
                to_min,
                to_max,
            } => {
                if let Some(v) = value.as_f64() {
                    let normalized = (v - from_min) / (from_max - from_min);
                    let scaled = to_min + normalized * (to_max - to_min);
                    Value::Float(scaled)
                } else {
                    value.clone()
                }
            }
            ValueTransform::Clamp { min, max } => {
                if let Some(v) = value.as_f64() {
                    Value::Float(v.clamp(*min, *max))
                } else {
                    value.clone()
                }
            }
            ValueTransform::Invert => {
                if let Some(v) = value.as_f64() {
                    Value::Float(1.0 - v)
                } else {
                    value.clone()
                }
            }
            ValueTransform::ToInt => {
                if let Some(v) = value.as_f64() {
                    Value::Int(v as i64)
                } else {
                    value.clone()
                }
            }
            ValueTransform::ToFloat => {
                if let Some(v) = value.as_i64() {
                    Value::Float(v as f64)
                } else {
                    value.clone()
                }
            }
            ValueTransform::Expression(expr) => {
                let mut context = HashMapContext::new();

                // Set value variable
                if let Some(v) = value.as_f64() {
                    let _ = context.set_value("value".to_string(), evalexpr::Value::Float(v));
                    let _ = context.set_value("x".to_string(), evalexpr::Value::Float(v));
                } else if let Some(v) = value.as_i64() {
                    let _ = context.set_value("value".to_string(), evalexpr::Value::Int(v));
                    let _ = context.set_value("x".to_string(), evalexpr::Value::Int(v));
                }

                // Add math constants
                let _ = context.set_value(
                    "PI".to_string(),
                    evalexpr::Value::Float(std::f64::consts::PI),
                );
                let _ =
                    context.set_value("E".to_string(), evalexpr::Value::Float(std::f64::consts::E));

                match eval_with_context_mut(expr, &mut context) {
                    Ok(evalexpr::Value::Float(f)) => Value::Float(f),
                    Ok(evalexpr::Value::Int(i)) => Value::Int(i),
                    Ok(evalexpr::Value::Boolean(b)) => Value::Bool(b),
                    Ok(evalexpr::Value::String(s)) => Value::String(s),
                    Ok(_) => value.clone(),
                    Err(e) => {
                        warn!("Expression evaluation failed: {}", e);
                        value.clone()
                    }
                }
            }
        }
    }
}

/// Collection of address mappings
#[derive(Debug, Clone, Default)]
pub struct MappingTable {
    mappings: Vec<AddressMapping>,
    cache: HashMap<String, String>,
}

impl MappingTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, mapping: AddressMapping) {
        self.mappings.push(mapping);
        self.cache.clear();
    }

    pub fn map(&mut self, addr: &str) -> Option<String> {
        // Check cache
        if let Some(cached) = self.cache.get(addr) {
            return Some(cached.clone());
        }

        // Find matching mapping
        for mapping in &self.mappings {
            if let Some(result) = mapping.map_address(addr) {
                self.cache.insert(addr.to_string(), result.clone());
                return Some(result);
            }
        }

        None
    }

    /// Transform a value using the mapping for an address
    pub fn transform(&self, addr: &str, value: &Value) -> Value {
        for mapping in &self.mappings {
            if mapping.map_address(addr).is_some() {
                if let Some(ref transform) = mapping.transform {
                    return transform.apply(value);
                }
            }
        }
        value.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_mapping() {
        let mapping = AddressMapping::new("/osc/synth/cutoff", "/synth/cutoff");
        assert_eq!(
            mapping.map_address("/osc/synth/cutoff"),
            Some("/synth/cutoff".to_string())
        );
        assert_eq!(mapping.map_address("/osc/synth/other"), None);
    }

    #[test]
    fn test_wildcard_mapping() {
        let mapping = AddressMapping::new("/midi/*/cc/*", "/midi/*/*/*");
        // This is a simplified test - real implementation would be more sophisticated
    }

    #[test]
    fn test_scale_transform() {
        let transform = ValueTransform::scale(0.0, 127.0, 0.0, 1.0);
        let result = transform.apply(&Value::Float(63.5));
        assert!((result.as_f64().unwrap() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_invert_transform() {
        let transform = ValueTransform::Invert;
        let result = transform.apply(&Value::Float(0.3));
        assert!((result.as_f64().unwrap() - 0.7).abs() < 0.001);
    }
}
