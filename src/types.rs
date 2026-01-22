//! Request and response types for the Hyperliquid API

use serde::{Deserialize, Serialize};

/// Time in force options for orders
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeInForce {
    /// Good till cancelled - order remains active until filled or cancelled
    #[serde(rename = "Gtc")]
    Gtc,
    /// Immediate or cancel - fill what's possible immediately, cancel the rest
    #[serde(rename = "Ioc")]
    Ioc,
    /// Add liquidity only - order is rejected if it would take liquidity
    #[serde(rename = "Alo")]
    Alo,
}

/// Order type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    /// Limit order
    #[serde(rename = "limit")]
    Limit,
    /// Market order (using trigger)
    #[serde(rename = "trigger")]
    Trigger,
}

/// Order side
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    /// Buy order
    #[serde(rename = "B")]
    Buy,
    /// Sell order
    #[serde(rename = "A")]
    Sell,
}

/// Trigger type for conditional orders
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TriggerType {
    /// Trigger when price rises above the trigger price
    Tp,
    /// Trigger when price falls below the trigger price
    Sl,
}

/// Margin mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarginMode {
    /// Cross margin - share margin across positions
    #[serde(rename = "cross")]
    Cross,
    /// Isolated margin - separate margin per position
    #[serde(rename = "isolated")]
    Isolated,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_in_force_serialization() {
        assert_eq!(serde_json::to_string(&TimeInForce::Gtc).unwrap(), "\"Gtc\"");
        assert_eq!(serde_json::to_string(&TimeInForce::Ioc).unwrap(), "\"Ioc\"");
        assert_eq!(serde_json::to_string(&TimeInForce::Alo).unwrap(), "\"Alo\"");
    }

    #[test]
    fn test_time_in_force_deserialization() {
        assert_eq!(
            serde_json::from_str::<TimeInForce>("\"Gtc\"").unwrap(),
            TimeInForce::Gtc
        );
        assert_eq!(
            serde_json::from_str::<TimeInForce>("\"Ioc\"").unwrap(),
            TimeInForce::Ioc
        );
        assert_eq!(
            serde_json::from_str::<TimeInForce>("\"Alo\"").unwrap(),
            TimeInForce::Alo
        );
    }

    #[test]
    fn test_side_serialization() {
        assert_eq!(serde_json::to_string(&Side::Buy).unwrap(), "\"B\"");
        assert_eq!(serde_json::to_string(&Side::Sell).unwrap(), "\"A\"");
    }

    #[test]
    fn test_side_deserialization() {
        assert_eq!(serde_json::from_str::<Side>("\"B\"").unwrap(), Side::Buy);
        assert_eq!(serde_json::from_str::<Side>("\"A\"").unwrap(), Side::Sell);
    }

    #[test]
    fn test_margin_mode_serialization() {
        assert_eq!(
            serde_json::to_string(&MarginMode::Cross).unwrap(),
            "\"cross\""
        );
        assert_eq!(
            serde_json::to_string(&MarginMode::Isolated).unwrap(),
            "\"isolated\""
        );
    }

    #[test]
    fn test_trigger_type_serialization() {
        assert_eq!(serde_json::to_string(&TriggerType::Tp).unwrap(), "\"tp\"");
        assert_eq!(serde_json::to_string(&TriggerType::Sl).unwrap(), "\"sl\"");
    }
}
