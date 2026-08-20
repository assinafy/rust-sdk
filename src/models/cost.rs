//! Cost-estimation models used by `estimate-cost` endpoints.

use serde::{Deserialize, Serialize};

/// Reason a cost estimate cannot be fulfilled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockingReason {
    /// Account has unpaid invoices.
    PendingPayment,
    /// Not enough document credits.
    InsufficientDocuments,
    /// Not enough notification credits.
    InsufficientCredits,
    /// Any value the SDK does not yet model.
    #[serde(untagged)]
    Other(String),
}

/// Single line item within a cost estimate.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CostBreakdownItem {
    /// Stable identifier (e.g. `"NotificationWhatsapp"`).
    pub code: String,
    /// Human-readable label.
    pub name: String,
    /// Total cost for this line.
    pub cost: f64,
    /// Quantity of units billed.
    #[serde(default)]
    pub quantity: f64,
    /// Per-unit cost.
    #[serde(default)]
    pub unit_cost: f64,
}

/// Cost estimate response.
#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct CostEstimate {
    /// Documents consumed (always 1 for assignments).
    #[serde(default)]
    pub documents: f64,
    /// Notification credits required.
    #[serde(default)]
    pub credits: f64,
    /// Whether the operation requires purchasing an extra document credit.
    #[serde(default)]
    pub needs_extra_document: bool,
    /// Cost of an extra document credit, when needed.
    #[serde(default)]
    pub extra_document_cost: f64,
    /// Total credits to charge.
    #[serde(default)]
    pub total_credits: f64,
    /// Legacy sandbox name for `total_credits`.
    #[serde(default)]
    pub total: f64,
    /// Itemised breakdown.
    #[serde(default)]
    pub breakdown: Vec<CostBreakdownItem>,
    /// Current document credit balance.
    #[serde(default)]
    pub document_balance: f64,
    /// Current notification credit balance.
    #[serde(default)]
    pub credit_balance: f64,
    /// Whether the account has the resources to proceed.
    #[serde(default)]
    pub has_sufficient_resources: bool,
    /// Legacy sandbox name for `has_sufficient_resources`.
    #[serde(default)]
    pub has_sufficient_credits: bool,
    /// Reason the operation is blocked, when applicable.
    #[serde(default)]
    pub blocking_reason: Option<BlockingReason>,
    /// Optional human-readable explanation.
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Deserialize)]
struct CostEstimateWire {
    #[serde(default)]
    documents: f64,
    #[serde(default)]
    credits: f64,
    #[serde(default)]
    needs_extra_document: bool,
    #[serde(default)]
    extra_document_cost: f64,
    total_credits: Option<f64>,
    total: Option<f64>,
    #[serde(default)]
    breakdown: Vec<CostBreakdownItem>,
    #[serde(default)]
    document_balance: f64,
    #[serde(default)]
    credit_balance: f64,
    has_sufficient_resources: Option<bool>,
    has_sufficient_credits: Option<bool>,
    #[serde(default)]
    blocking_reason: Option<BlockingReason>,
    #[serde(default)]
    message: Option<String>,
}

impl<'de> Deserialize<'de> for CostEstimate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = CostEstimateWire::deserialize(deserializer)?;
        let total_credits = wire.total_credits.or(wire.total).unwrap_or_default();
        let has_sufficient_resources = wire
            .has_sufficient_resources
            .or(wire.has_sufficient_credits)
            .unwrap_or_default();

        Ok(Self {
            documents: wire.documents,
            credits: wire.credits,
            needs_extra_document: wire.needs_extra_document,
            extra_document_cost: wire.extra_document_cost,
            total_credits,
            total: total_credits,
            breakdown: wire.breakdown,
            document_balance: wire.document_balance,
            credit_balance: wire.credit_balance,
            has_sufficient_resources,
            has_sufficient_credits: has_sufficient_resources,
            blocking_reason: wire.blocking_reason,
            message: wire.message,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::CostEstimate;

    #[test]
    fn current_cost_fields_populate_legacy_aliases() {
        let estimate: CostEstimate =
            serde_json::from_str(r#"{"total_credits":3.5,"has_sufficient_resources":true}"#)
                .unwrap();

        assert_eq!(estimate.total_credits, 3.5);
        assert_eq!(estimate.total, 3.5);
        assert!(estimate.has_sufficient_resources);
        assert!(estimate.has_sufficient_credits);
    }

    #[test]
    fn legacy_cost_fields_populate_current_fields() {
        let estimate: CostEstimate =
            serde_json::from_str(r#"{"total":2.0,"has_sufficient_credits":true}"#).unwrap();

        assert_eq!(estimate.total_credits, 2.0);
        assert_eq!(estimate.total, 2.0);
        assert!(estimate.has_sufficient_resources);
        assert!(estimate.has_sufficient_credits);
    }

    #[test]
    fn current_cost_fields_win_when_aliases_conflict() {
        let estimate: CostEstimate = serde_json::from_str(
            r#"{"total_credits":4.0,"total":9.0,"has_sufficient_resources":false,"has_sufficient_credits":true}"#,
        )
        .unwrap();

        assert_eq!(estimate.total_credits, 4.0);
        assert_eq!(estimate.total, 4.0);
        assert!(!estimate.has_sufficient_resources);
        assert!(!estimate.has_sufficient_credits);
    }
}
