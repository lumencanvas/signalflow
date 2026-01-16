//! Enhanced value transformation system
//!
//! This module provides comprehensive value transformation capabilities:
//! - Expression engine for JavaScript-like math expressions
//! - Lookup tables for discrete value mapping
//! - Curve functions (easing, exponential, logarithmic, bezier)
//! - Aggregation functions (average, sum, min, max, moving average)
//! - Conditional transforms based on value or metadata
//! - JSON path extraction and injection

use clasp_core::Value;
use evalexpr::{eval_with_context_mut, HashMapContext, ContextWithMutableVariables};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::f64::consts::PI;
use tracing::warn;

/// Enhanced transform that can be applied to values
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Transform {
    /// No transformation (passthrough)
    Identity,

    /// Mathematical expression evaluation
    /// Variables: `value`, `index`, `time`
    Expression {
        expr: String,
    },

    /// Scale from one range to another
    Scale {
        from_min: f64,
        from_max: f64,
        to_min: f64,
        to_max: f64,
    },

    /// Clamp to range
    Clamp {
        min: f64,
        max: f64,
    },

    /// Invert (1 - x for normalized values)
    Invert,

    /// Convert to integer
    ToInt,

    /// Convert to float
    ToFloat,

    /// Lookup table for discrete value mapping
    Lookup {
        table: HashMap<String, Value>,
        default: Option<Value>,
    },

    /// Easing curve functions
    Curve {
        curve_type: CurveType,
    },

    /// Quantize to discrete steps
    Quantize {
        steps: u32,
    },

    /// Dead zone (values below threshold become 0)
    DeadZone {
        threshold: f64,
    },

    /// Smoothing with exponential moving average
    Smooth {
        factor: f64,
    },

    /// Rate limiter (max change per update)
    RateLimit {
        max_delta: f64,
    },

    /// Threshold trigger (output 0 or 1 based on threshold)
    Threshold {
        value: f64,
        mode: ThresholdMode,
    },

    /// Modulo operation
    Modulo {
        divisor: f64,
    },

    /// Absolute value
    Abs,

    /// Negate
    Negate,

    /// Power function
    Power {
        exponent: f64,
    },

    /// Logarithm
    Log {
        base: Option<f64>,
    },

    /// Round to decimal places
    Round {
        decimals: u32,
    },

    /// Chain multiple transforms
    Chain {
        transforms: Vec<Transform>,
    },

    /// Conditional transform based on value
    Conditional {
        condition: Condition,
        if_true: Box<Transform>,
        if_false: Box<Transform>,
    },

    /// JSON path extraction (for complex values)
    JsonPath {
        path: String,
    },

    /// Map to different value type
    MapType {
        from_type: ValueType,
        to_type: ValueType,
    },

    /// Bitwise operations
    Bitwise {
        operation: BitwiseOp,
        operand: Option<i64>,
    },
}

/// Curve types for non-linear transforms
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CurveType {
    /// Linear (no curve)
    Linear,
    /// Ease in (slow start)
    EaseIn,
    /// Ease out (slow end)
    EaseOut,
    /// Ease in and out
    EaseInOut,
    /// Quadratic ease in
    QuadIn,
    /// Quadratic ease out
    QuadOut,
    /// Quadratic ease in-out
    QuadInOut,
    /// Cubic ease in
    CubicIn,
    /// Cubic ease out
    CubicOut,
    /// Cubic ease in-out
    CubicInOut,
    /// Exponential ease in
    ExpoIn,
    /// Exponential ease out
    ExpoOut,
    /// Exponential ease in-out
    ExpoInOut,
    /// Sine ease in
    SineIn,
    /// Sine ease out
    SineOut,
    /// Sine ease in-out
    SineInOut,
    /// Circular ease in
    CircIn,
    /// Circular ease out
    CircOut,
    /// Circular ease in-out
    CircInOut,
    /// Elastic ease in
    ElasticIn,
    /// Elastic ease out
    ElasticOut,
    /// Bounce ease out
    BounceOut,
    /// Custom bezier curve
    Bezier {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
    },
}

/// Threshold mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThresholdMode {
    /// Output 1 if above, 0 if below
    Above,
    /// Output 1 if below, 0 if above
    Below,
    /// Output 1 if equal (within tolerance)
    Equal,
}

/// Condition for conditional transforms
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Condition {
    /// Value is greater than threshold
    GreaterThan { value: f64 },
    /// Value is less than threshold
    LessThan { value: f64 },
    /// Value equals (within tolerance)
    Equals { value: f64, tolerance: Option<f64> },
    /// Value is in range
    InRange { min: f64, max: f64 },
    /// Expression evaluates to true
    Expression { expr: String },
    /// Logical AND of conditions
    And { conditions: Vec<Condition> },
    /// Logical OR of conditions
    Or { conditions: Vec<Condition> },
    /// Logical NOT
    Not { condition: Box<Condition> },
}

/// Value types for type conversion
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueType {
    Int,
    Float,
    Bool,
    String,
}

/// Bitwise operations
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BitwiseOp {
    And,
    Or,
    Xor,
    Not,
    ShiftLeft,
    ShiftRight,
    GetBit,
    SetBit,
    ClearBit,
}

/// Transform state for stateful transforms (smoothing, rate limiting, etc.)
#[derive(Debug, Clone, Default)]
pub struct TransformState {
    /// Last value for smoothing/rate limiting
    pub last_value: Option<f64>,
    /// Moving average window
    pub window: VecDeque<f64>,
    /// Timestamp of last update
    pub last_time: Option<std::time::Instant>,
}

impl Transform {
    /// Apply the transform to a value
    pub fn apply(&self, value: &Value, state: &mut TransformState) -> Value {
        match self {
            Transform::Identity => value.clone(),

            Transform::Expression { expr } => {
                self.eval_expression(expr, value)
            }

            Transform::Scale { from_min, from_max, to_min, to_max } => {
                if let Some(v) = value.as_f64() {
                    let normalized = (v - from_min) / (from_max - from_min);
                    let scaled = to_min + normalized * (to_max - to_min);
                    Value::Float(scaled)
                } else {
                    value.clone()
                }
            }

            Transform::Clamp { min, max } => {
                if let Some(v) = value.as_f64() {
                    Value::Float(v.clamp(*min, *max))
                } else {
                    value.clone()
                }
            }

            Transform::Invert => {
                if let Some(v) = value.as_f64() {
                    Value::Float(1.0 - v)
                } else {
                    value.clone()
                }
            }

            Transform::ToInt => {
                if let Some(v) = value.as_f64() {
                    Value::Int(v as i64)
                } else {
                    value.clone()
                }
            }

            Transform::ToFloat => {
                if let Some(v) = value.as_i64() {
                    Value::Float(v as f64)
                } else {
                    value.clone()
                }
            }

            Transform::Lookup { table, default } => {
                let key = match value {
                    Value::Int(i) => i.to_string(),
                    Value::Float(f) => f.to_string(),
                    Value::String(s) => s.clone(),
                    Value::Bool(b) => b.to_string(),
                    _ => return default.clone().unwrap_or_else(|| value.clone()),
                };
                table.get(&key).cloned().unwrap_or_else(|| {
                    default.clone().unwrap_or_else(|| value.clone())
                })
            }

            Transform::Curve { curve_type } => {
                if let Some(t) = value.as_f64() {
                    let t = t.clamp(0.0, 1.0);
                    Value::Float(curve_type.apply(t))
                } else {
                    value.clone()
                }
            }

            Transform::Quantize { steps } => {
                if let Some(v) = value.as_f64() {
                    let steps = *steps as f64;
                    let quantized = (v * steps).round() / steps;
                    Value::Float(quantized)
                } else {
                    value.clone()
                }
            }

            Transform::DeadZone { threshold } => {
                if let Some(v) = value.as_f64() {
                    if v.abs() < *threshold {
                        Value::Float(0.0)
                    } else {
                        value.clone()
                    }
                } else {
                    value.clone()
                }
            }

            Transform::Smooth { factor } => {
                if let Some(v) = value.as_f64() {
                    let smoothed = if let Some(last) = state.last_value {
                        last + factor * (v - last)
                    } else {
                        v
                    };
                    state.last_value = Some(smoothed);
                    Value::Float(smoothed)
                } else {
                    value.clone()
                }
            }

            Transform::RateLimit { max_delta } => {
                if let Some(v) = value.as_f64() {
                    let limited = if let Some(last) = state.last_value {
                        let delta = v - last;
                        if delta.abs() > *max_delta {
                            last + max_delta * delta.signum()
                        } else {
                            v
                        }
                    } else {
                        v
                    };
                    state.last_value = Some(limited);
                    Value::Float(limited)
                } else {
                    value.clone()
                }
            }

            Transform::Threshold { value: thresh, mode } => {
                if let Some(v) = value.as_f64() {
                    let result = match mode {
                        ThresholdMode::Above => if v > *thresh { 1.0 } else { 0.0 },
                        ThresholdMode::Below => if v < *thresh { 1.0 } else { 0.0 },
                        ThresholdMode::Equal => if (v - thresh).abs() < 0.001 { 1.0 } else { 0.0 },
                    };
                    Value::Float(result)
                } else {
                    value.clone()
                }
            }

            Transform::Modulo { divisor } => {
                if let Some(v) = value.as_f64() {
                    Value::Float(v % divisor)
                } else {
                    value.clone()
                }
            }

            Transform::Abs => {
                if let Some(v) = value.as_f64() {
                    Value::Float(v.abs())
                } else if let Some(v) = value.as_i64() {
                    Value::Int(v.abs())
                } else {
                    value.clone()
                }
            }

            Transform::Negate => {
                if let Some(v) = value.as_f64() {
                    Value::Float(-v)
                } else if let Some(v) = value.as_i64() {
                    Value::Int(-v)
                } else {
                    value.clone()
                }
            }

            Transform::Power { exponent } => {
                if let Some(v) = value.as_f64() {
                    Value::Float(v.powf(*exponent))
                } else {
                    value.clone()
                }
            }

            Transform::Log { base } => {
                if let Some(v) = value.as_f64() {
                    if v > 0.0 {
                        let result = match base {
                            Some(b) => v.log(*b),
                            None => v.ln(),
                        };
                        Value::Float(result)
                    } else {
                        value.clone()
                    }
                } else {
                    value.clone()
                }
            }

            Transform::Round { decimals } => {
                if let Some(v) = value.as_f64() {
                    let factor = 10_f64.powi(*decimals as i32);
                    Value::Float((v * factor).round() / factor)
                } else {
                    value.clone()
                }
            }

            Transform::Chain { transforms } => {
                let mut result = value.clone();
                for transform in transforms {
                    result = transform.apply(&result, state);
                }
                result
            }

            Transform::Conditional { condition, if_true, if_false } => {
                if condition.evaluate(value) {
                    if_true.apply(value, state)
                } else {
                    if_false.apply(value, state)
                }
            }

            Transform::JsonPath { path } => {
                self.extract_json_path(value, path)
            }

            Transform::MapType { to_type, .. } => {
                match to_type {
                    ValueType::Int => {
                        if let Some(v) = value.as_f64() {
                            Value::Int(v as i64)
                        } else if let Some(s) = value.as_str().map(|s| s.to_string()) {
                            s.parse::<i64>().map(Value::Int).unwrap_or_else(|_| value.clone())
                        } else {
                            value.clone()
                        }
                    }
                    ValueType::Float => {
                        if let Some(v) = value.as_i64() {
                            Value::Float(v as f64)
                        } else if let Some(s) = value.as_str().map(|s| s.to_string()) {
                            s.parse::<f64>().map(Value::Float).unwrap_or_else(|_| value.clone())
                        } else {
                            value.clone()
                        }
                    }
                    ValueType::Bool => {
                        if let Some(v) = value.as_f64() {
                            Value::Bool(v != 0.0)
                        } else if let Some(v) = value.as_i64() {
                            Value::Bool(v != 0)
                        } else {
                            value.clone()
                        }
                    }
                    ValueType::String => {
                        match value {
                            Value::Int(i) => Value::String(i.to_string()),
                            Value::Float(f) => Value::String(f.to_string()),
                            Value::Bool(b) => Value::String(b.to_string()),
                            _ => value.clone(),
                        }
                    }
                }
            }

            Transform::Bitwise { operation, operand } => {
                if let Some(v) = value.as_i64() {
                    let result = match operation {
                        BitwiseOp::And => v & operand.unwrap_or(0),
                        BitwiseOp::Or => v | operand.unwrap_or(0),
                        BitwiseOp::Xor => v ^ operand.unwrap_or(0),
                        BitwiseOp::Not => !v,
                        BitwiseOp::ShiftLeft => v << operand.unwrap_or(1) as u32,
                        BitwiseOp::ShiftRight => v >> operand.unwrap_or(1) as u32,
                        BitwiseOp::GetBit => (v >> operand.unwrap_or(0) as u32) & 1,
                        BitwiseOp::SetBit => v | (1 << operand.unwrap_or(0) as u32),
                        BitwiseOp::ClearBit => v & !(1 << operand.unwrap_or(0) as u32),
                    };
                    Value::Int(result)
                } else {
                    value.clone()
                }
            }
        }
    }

    fn eval_expression(&self, expr: &str, value: &Value) -> Value {
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
        let _ = context.set_value("PI".to_string(), evalexpr::Value::Float(PI));
        let _ = context.set_value("E".to_string(), evalexpr::Value::Float(std::f64::consts::E));

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

    fn extract_json_path(&self, value: &Value, path: &str) -> Value {
        // Convert Value to serde_json::Value for jsonpath
        let json_value = Self::value_to_json(value);

        match jsonpath_lib::select(&json_value, path) {
            Ok(results) => {
                if results.len() == 1 {
                    Self::json_to_value(results[0])
                } else if !results.is_empty() {
                    let arr: Vec<Value> = results.iter().map(|v| Self::json_to_value(v)).collect();
                    Value::Array(arr)
                } else {
                    Value::Null
                }
            }
            Err(e) => {
                warn!("JSON path extraction failed: {}", e);
                value.clone()
            }
        }
    }

    fn value_to_json(value: &Value) -> serde_json::Value {
        match value {
            Value::Int(i) => serde_json::Value::Number((*i).into()),
            Value::Float(f) => serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            Value::String(s) => serde_json::Value::String(s.clone()),
            Value::Bool(b) => serde_json::Value::Bool(*b),
            Value::Bytes(b) => serde_json::Value::String(base64::encode(b)),
            Value::Array(arr) => serde_json::Value::Array(
                arr.iter().map(Self::value_to_json).collect()
            ),
            Value::Map(m) => {
                let obj: serde_json::Map<String, serde_json::Value> = m.iter()
                    .map(|(k, v)| (k.clone(), Self::value_to_json(v)))
                    .collect();
                serde_json::Value::Object(obj)
            }
            Value::Null => serde_json::Value::Null,
        }
    }

    fn json_to_value(json: &serde_json::Value) -> Value {
        match json {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(b) => Value::Bool(*b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::Int(i)
                } else if let Some(f) = n.as_f64() {
                    Value::Float(f)
                } else {
                    Value::Null
                }
            }
            serde_json::Value::String(s) => Value::String(s.clone()),
            serde_json::Value::Array(arr) => {
                Value::Array(arr.iter().map(Self::json_to_value).collect())
            }
            serde_json::Value::Object(_) => {
                // Convert object to JSON string
                Value::String(json.to_string())
            }
        }
    }
}

impl CurveType {
    /// Apply the easing curve to a normalized value (0-1)
    pub fn apply(&self, t: f64) -> f64 {
        match self {
            CurveType::Linear => t,
            CurveType::EaseIn => t * t,
            CurveType::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
            CurveType::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }
            CurveType::QuadIn => t * t,
            CurveType::QuadOut => 1.0 - (1.0 - t).powi(2),
            CurveType::QuadInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }
            CurveType::CubicIn => t * t * t,
            CurveType::CubicOut => 1.0 - (1.0 - t).powi(3),
            CurveType::CubicInOut => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
            CurveType::ExpoIn => {
                if t == 0.0 { 0.0 } else { 2_f64.powf(10.0 * t - 10.0) }
            }
            CurveType::ExpoOut => {
                if t == 1.0 { 1.0 } else { 1.0 - 2_f64.powf(-10.0 * t) }
            }
            CurveType::ExpoInOut => {
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else if t < 0.5 {
                    2_f64.powf(20.0 * t - 10.0) / 2.0
                } else {
                    (2.0 - 2_f64.powf(-20.0 * t + 10.0)) / 2.0
                }
            }
            CurveType::SineIn => 1.0 - (t * PI / 2.0).cos(),
            CurveType::SineOut => (t * PI / 2.0).sin(),
            CurveType::SineInOut => -((PI * t).cos() - 1.0) / 2.0,
            CurveType::CircIn => 1.0 - (1.0 - t * t).sqrt(),
            CurveType::CircOut => (1.0 - (t - 1.0).powi(2)).sqrt(),
            CurveType::CircInOut => {
                if t < 0.5 {
                    (1.0 - (1.0 - (2.0 * t).powi(2)).sqrt()) / 2.0
                } else {
                    ((1.0 - (-2.0 * t + 2.0).powi(2)).sqrt() + 1.0) / 2.0
                }
            }
            CurveType::ElasticIn => {
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else {
                    let c4 = (2.0 * PI) / 3.0;
                    -2_f64.powf(10.0 * t - 10.0) * ((t * 10.0 - 10.75) * c4).sin()
                }
            }
            CurveType::ElasticOut => {
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else {
                    let c4 = (2.0 * PI) / 3.0;
                    2_f64.powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0
                }
            }
            CurveType::BounceOut => {
                let n1 = 7.5625;
                let d1 = 2.75;
                if t < 1.0 / d1 {
                    n1 * t * t
                } else if t < 2.0 / d1 {
                    let t = t - 1.5 / d1;
                    n1 * t * t + 0.75
                } else if t < 2.5 / d1 {
                    let t = t - 2.25 / d1;
                    n1 * t * t + 0.9375
                } else {
                    let t = t - 2.625 / d1;
                    n1 * t * t + 0.984375
                }
            }
            CurveType::Bezier { x1, y1, x2, y2 } => {
                // Approximate cubic bezier curve
                Self::cubic_bezier(t, *x1, *y1, *x2, *y2)
            }
        }
    }

    fn cubic_bezier(t: f64, x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
        // Newton-Raphson iteration to find t for given x
        let mut guess = t;
        for _ in 0..8 {
            let x = Self::bezier_x(guess, x1, x2) - t;
            if x.abs() < 1e-6 {
                break;
            }
            let dx = Self::bezier_dx(guess, x1, x2);
            if dx.abs() < 1e-6 {
                break;
            }
            guess -= x / dx;
        }
        Self::bezier_y(guess, y1, y2)
    }

    fn bezier_x(t: f64, x1: f64, x2: f64) -> f64 {
        let t2 = t * t;
        let t3 = t2 * t;
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        3.0 * mt2 * t * x1 + 3.0 * mt * t2 * x2 + t3
    }

    fn bezier_y(t: f64, y1: f64, y2: f64) -> f64 {
        let t2 = t * t;
        let t3 = t2 * t;
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        3.0 * mt2 * t * y1 + 3.0 * mt * t2 * y2 + t3
    }

    fn bezier_dx(t: f64, x1: f64, x2: f64) -> f64 {
        let t2 = t * t;
        let mt = 1.0 - t;
        3.0 * mt * mt * x1 + 6.0 * mt * t * (x2 - x1) + 3.0 * t2 * (1.0 - x2)
    }
}

impl Condition {
    /// Evaluate the condition against a value
    pub fn evaluate(&self, value: &Value) -> bool {
        match self {
            Condition::GreaterThan { value: threshold } => {
                value.as_f64().map(|v| v > *threshold).unwrap_or(false)
            }
            Condition::LessThan { value: threshold } => {
                value.as_f64().map(|v| v < *threshold).unwrap_or(false)
            }
            Condition::Equals { value: target, tolerance } => {
                if let Some(v) = value.as_f64() {
                    let tol = tolerance.unwrap_or(0.001);
                    (v - target).abs() < tol
                } else {
                    false
                }
            }
            Condition::InRange { min, max } => {
                value.as_f64().map(|v| v >= *min && v <= *max).unwrap_or(false)
            }
            Condition::Expression { expr } => {
                let mut context = HashMapContext::new();
                if let Some(v) = value.as_f64() {
                    let _ = context.set_value("value".to_string(), evalexpr::Value::Float(v));
                }
                eval_with_context_mut(expr, &mut context)
                    .map(|v| v.as_boolean().unwrap_or(false))
                    .unwrap_or(false)
            }
            Condition::And { conditions } => {
                conditions.iter().all(|c| c.evaluate(value))
            }
            Condition::Or { conditions } => {
                conditions.iter().any(|c| c.evaluate(value))
            }
            Condition::Not { condition } => {
                !condition.evaluate(value)
            }
        }
    }
}

/// Aggregator for combining multiple values
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Aggregator {
    /// Average of all values
    Average,
    /// Sum of all values
    Sum,
    /// Minimum value
    Min,
    /// Maximum value
    Max,
    /// Most recent value
    Latest,
    /// First value
    First,
    /// Count of values
    Count,
    /// Moving average over window
    MovingAverage { window_size: usize },
    /// Rate of change (delta per second)
    RateOfChange,
    /// Standard deviation
    StdDev,
}

/// Aggregator state
#[derive(Debug, Clone, Default)]
pub struct AggregatorState {
    values: VecDeque<f64>,
    last_value: Option<f64>,
    last_time: Option<std::time::Instant>,
    window_size: usize,
}

impl Aggregator {
    /// Create a new aggregator state
    pub fn new_state(&self) -> AggregatorState {
        let window_size = match self {
            Aggregator::MovingAverage { window_size } => *window_size,
            _ => 100,
        };
        AggregatorState {
            window_size,
            ..Default::default()
        }
    }

    /// Add a value and get the aggregated result
    pub fn add(&self, value: f64, state: &mut AggregatorState) -> f64 {
        // Add to window
        state.values.push_back(value);
        while state.values.len() > state.window_size {
            state.values.pop_front();
        }

        let result = match self {
            Aggregator::Average => {
                state.values.iter().sum::<f64>() / state.values.len() as f64
            }
            Aggregator::Sum => {
                state.values.iter().sum()
            }
            Aggregator::Min => {
                state.values.iter().cloned().fold(f64::INFINITY, f64::min)
            }
            Aggregator::Max => {
                state.values.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
            }
            Aggregator::Latest => value,
            Aggregator::First => {
                state.values.front().copied().unwrap_or(value)
            }
            Aggregator::Count => state.values.len() as f64,
            Aggregator::MovingAverage { .. } => {
                state.values.iter().sum::<f64>() / state.values.len() as f64
            }
            Aggregator::RateOfChange => {
                let now = std::time::Instant::now();
                let rate = if let (Some(last), Some(time)) = (state.last_value, state.last_time) {
                    let dt = now.duration_since(time).as_secs_f64();
                    if dt > 0.0 {
                        (value - last) / dt
                    } else {
                        0.0
                    }
                } else {
                    0.0
                };
                state.last_time = Some(now);
                rate
            }
            Aggregator::StdDev => {
                if state.values.is_empty() {
                    0.0
                } else {
                    let mean = state.values.iter().sum::<f64>() / state.values.len() as f64;
                    let variance = state.values.iter()
                        .map(|v| (v - mean).powi(2))
                        .sum::<f64>() / state.values.len() as f64;
                    variance.sqrt()
                }
            }
        };

        state.last_value = Some(value);
        result
    }
}

/// Helper for base64 encoding (used in JSON path)
mod base64 {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    pub fn encode(data: &[u8]) -> String {
        let mut result = String::new();
        for chunk in data.chunks(3) {
            let mut n = (chunk[0] as u32) << 16;
            if chunk.len() > 1 {
                n |= (chunk[1] as u32) << 8;
            }
            if chunk.len() > 2 {
                n |= chunk[2] as u32;
            }

            result.push(CHARS[(n >> 18 & 0x3F) as usize] as char);
            result.push(CHARS[(n >> 12 & 0x3F) as usize] as char);
            if chunk.len() > 1 {
                result.push(CHARS[(n >> 6 & 0x3F) as usize] as char);
            } else {
                result.push('=');
            }
            if chunk.len() > 2 {
                result.push(CHARS[(n & 0x3F) as usize] as char);
            } else {
                result.push('=');
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expression_transform() {
        let transform = Transform::Expression {
            expr: "value * 2 + 10".to_string(),
        };
        let mut state = TransformState::default();
        let result = transform.apply(&Value::Float(5.0), &mut state);
        assert_eq!(result.as_f64(), Some(20.0));
    }

    #[test]
    fn test_lookup_transform() {
        let mut table = HashMap::new();
        table.insert("0".to_string(), Value::String("off".to_string()));
        table.insert("1".to_string(), Value::String("on".to_string()));

        let transform = Transform::Lookup {
            table,
            default: Some(Value::String("unknown".to_string())),
        };
        let mut state = TransformState::default();

        assert_eq!(
            transform.apply(&Value::Int(0), &mut state).as_str(),
            Some("off")
        );
        assert_eq!(
            transform.apply(&Value::Int(1), &mut state).as_str(),
            Some("on")
        );
        assert_eq!(
            transform.apply(&Value::Int(99), &mut state).as_str(),
            Some("unknown")
        );
    }

    #[test]
    fn test_curve_ease_in() {
        let curve = CurveType::EaseIn;
        assert!((curve.apply(0.0) - 0.0).abs() < 0.001);
        assert!((curve.apply(0.5) - 0.25).abs() < 0.001);
        assert!((curve.apply(1.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_curve_ease_out() {
        let curve = CurveType::EaseOut;
        assert!((curve.apply(0.0) - 0.0).abs() < 0.001);
        assert!((curve.apply(0.5) - 0.75).abs() < 0.001);
        assert!((curve.apply(1.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_smooth_transform() {
        let transform = Transform::Smooth { factor: 0.5 };
        let mut state = TransformState::default();

        // First value passes through
        let r1 = transform.apply(&Value::Float(1.0), &mut state);
        assert_eq!(r1.as_f64(), Some(1.0));

        // Second value is smoothed
        let r2 = transform.apply(&Value::Float(0.0), &mut state);
        assert!((r2.as_f64().unwrap() - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_chain_transform() {
        let transform = Transform::Chain {
            transforms: vec![
                Transform::Scale {
                    from_min: 0.0,
                    from_max: 127.0,
                    to_min: 0.0,
                    to_max: 1.0,
                },
                Transform::Curve { curve_type: CurveType::EaseIn },
            ],
        };
        let mut state = TransformState::default();

        let result = transform.apply(&Value::Float(63.5), &mut state);
        // 63.5/127 = 0.5, then ease-in: 0.5^2 = 0.25
        assert!((result.as_f64().unwrap() - 0.25).abs() < 0.01);
    }

    #[test]
    fn test_conditional_transform() {
        let transform = Transform::Conditional {
            condition: Condition::GreaterThan { value: 0.5 },
            if_true: Box::new(Transform::Expression { expr: "1.0".to_string() }),
            if_false: Box::new(Transform::Expression { expr: "0.0".to_string() }),
        };
        let mut state = TransformState::default();

        assert_eq!(transform.apply(&Value::Float(0.7), &mut state).as_f64(), Some(1.0));
        assert_eq!(transform.apply(&Value::Float(0.3), &mut state).as_f64(), Some(0.0));
    }

    #[test]
    fn test_aggregator_average() {
        let agg = Aggregator::Average;
        let mut state = agg.new_state();

        assert_eq!(agg.add(1.0, &mut state), 1.0);
        assert_eq!(agg.add(2.0, &mut state), 1.5);
        assert_eq!(agg.add(3.0, &mut state), 2.0);
    }

    #[test]
    fn test_aggregator_moving_average() {
        let agg = Aggregator::MovingAverage { window_size: 3 };
        let mut state = agg.new_state();

        agg.add(1.0, &mut state);
        agg.add(2.0, &mut state);
        agg.add(3.0, &mut state);
        let result = agg.add(4.0, &mut state);
        // Window: [2, 3, 4], average = 3
        assert_eq!(result, 3.0);
    }
}
