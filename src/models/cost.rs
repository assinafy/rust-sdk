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
    pub quantity: f64,
    /// Per-unit cost.
    pub unit_cost: f64,
}

/// Cost estimate response.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Reason the operation is blocked, when applicable.
    #[serde(default)]
    pub blocking_reason: Option<BlockingReason>,
    /// Optional human-readable explanation.
    #[serde(default)]
    pub message: Option<String>,
}
