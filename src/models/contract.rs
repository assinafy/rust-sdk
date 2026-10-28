//! Contract-tracking models.

use std::collections::BTreeMap;
use std::fmt;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use super::{Document, DocumentStatus};

/// Lifecycle state of tracked contract terms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractStatus {
    /// Terms are recorded but not yet in force.
    Draft,
    /// The contract is in force.
    Active,
    /// The contract reached its end date without renewal.
    Expired,
    /// The contract was explicitly terminated.
    Terminated,
    /// A server value not yet modeled by the SDK.
    #[serde(untagged)]
    Unknown(String),
}

impl ContractStatus {
    /// Returns the wire-format string.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Draft => "draft",
            Self::Active => "active",
            Self::Expired => "expired",
            Self::Terminated => "terminated",
            Self::Unknown(value) => value,
        }
    }
}

impl fmt::Display for ContractStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// How an instrument relates to its parent contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractRelationType {
    /// Changes one or more parent terms.
    Amendment,
    /// Starts the contract's next term.
    Renewal,
    /// Ends the parent contract.
    Termination,
    /// A server value not yet modeled by the SDK.
    #[serde(untagged)]
    Unknown(String),
}

impl ContractRelationType {
    /// Returns the wire-format string.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Amendment => "amendment",
            Self::Renewal => "renewal",
            Self::Termination => "termination",
            Self::Unknown(value) => value,
        }
    }
}

/// Application state of an amendment instrument.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractAmendmentStatus {
    /// The instrument has not yet changed the parent terms.
    Pending,
    /// The instrument's delta has been applied to the parent.
    Applied,
    /// A server value not yet modeled by the SDK.
    #[serde(untagged)]
    Unknown(String),
}

/// Reference to the contract in an external system.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ContractExternalReference {
    /// External system name, such as `hubspot`.
    pub system: String,
    /// Identifier used by that system.
    pub id: String,
}

impl ContractExternalReference {
    /// Build an external reference.
    pub fn new<S: Into<String>, I: Into<String>>(system: S, id: I) -> Self {
        Self {
            system: system.into(),
            id: id.into(),
        }
    }
}

/// Contract terms tracked on a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DocumentContract {
    /// Resource discriminator (`"document_contract"`) when present.
    #[serde(default)]
    pub resource: Option<String>,
    /// Document whose terms are tracked.
    pub document_id: String,
    /// Current lifecycle state.
    pub status: ContractStatus,
    /// Counterparty name.
    #[serde(default)]
    pub counterparty_name: Option<String>,
    /// Counterparty CPF or CNPJ, returned as digits only.
    #[serde(default)]
    pub counterparty_taxpayer_id: Option<String>,
    /// Contract value.
    #[serde(default)]
    pub value: Option<f64>,
    /// ISO 4217 currency code.
    #[serde(default)]
    pub currency: Option<String>,
    /// Start date in `YYYY-MM-DD` form.
    #[serde(default)]
    pub start_date: Option<String>,
    /// End date in `YYYY-MM-DD` form.
    #[serde(default)]
    pub end_date: Option<String>,
    /// Whether the lifecycle job renews the contract at its end date.
    #[serde(default)]
    pub auto_renew: bool,
    /// Months added by a term renewal.
    #[serde(default)]
    pub renewal_term_months: Option<u32>,
    /// Days before the end date when the notice period begins.
    #[serde(default)]
    pub notice_period_days: Option<u32>,
    /// Current term number, starting at one.
    #[serde(default)]
    pub term: u32,
    /// Workspace member responsible for the contract.
    #[serde(default)]
    pub owner_id: Option<String>,
    /// Caller-defined metadata.
    #[serde(default)]
    pub metadata: Option<BTreeMap<String, String>>,
    /// References to the contract in external systems.
    #[serde(default)]
    pub external_references: Vec<ContractExternalReference>,
    /// Expanded document, included only when requested by the list endpoint.
    #[serde(default)]
    pub document: Option<Document>,
    /// Instruments related to these terms, included by single/family reads.
    #[serde(default)]
    pub amendments: Vec<DocumentContractAmendment>,
    /// Creation timestamp.
    #[serde(default)]
    pub created_at: Option<String>,
    /// Last-modification timestamp used for optimistic write preconditions.
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// A document that amends, renews, or terminates a parent contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DocumentContractAmendment {
    /// Resource discriminator (`"document_contract_amendment"`) when present.
    #[serde(default)]
    pub resource: Option<String>,
    /// Instrument document identifier.
    pub document_id: String,
    /// Instrument document name.
    #[serde(default)]
    pub document_name: String,
    /// Root contract document identifier.
    pub parent_document_id: String,
    /// Relationship to the parent contract.
    pub relation_type: ContractRelationType,
    /// Position among applied amendments; absent while pending.
    #[serde(default)]
    pub number: Option<u32>,
    /// Whether the delta is pending or applied.
    pub status: ContractAmendmentStatus,
    /// Current lifecycle state of the instrument document.
    pub document_status: DocumentStatus,
    /// Why a signed instrument could not yet be applied.
    #[serde(default)]
    pub apply_error: Option<String>,
    /// Currency used by monetary values in `terms` and `changes`.
    #[serde(default)]
    pub currency: Option<String>,
    /// Terms changed by this instrument.
    #[serde(default)]
    pub terms: Option<BTreeMap<String, serde_json::Value>>,
    /// Parent values before and after each change.
    #[serde(default)]
    pub changes: Option<BTreeMap<String, serde_json::Value>>,
    /// Time the delta was applied.
    #[serde(default)]
    pub applied_at: Option<String>,
    /// Creation timestamp.
    #[serde(default)]
    pub created_at: Option<String>,
    /// Last-modification timestamp used for optimistic write preconditions.
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// Result of reading or updating one document's contract data.
///
/// A document either owns tracked [`Contract`](Self::Contract) terms or is an
/// [`Amendment`](Self::Amendment) linked to another contract. Unknown future
/// resource kinds are preserved as raw JSON.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
#[non_exhaustive]
pub enum DocumentContractDetails {
    /// Terms owned by this document.
    Contract(Box<DocumentContract>),
    /// This document's link and delta against a parent contract.
    Amendment(Box<DocumentContractAmendment>),
    /// A future resource shape not yet modeled by the SDK.
    Unknown(serde_json::Value),
}

impl<'de> Deserialize<'de> for DocumentContractDetails {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        let resource = value.get("resource").and_then(serde_json::Value::as_str);
        let amendment_without_discriminator = resource.is_none()
            && value.get("parent_document_id").is_some()
            && value.get("relation_type").is_some();
        let contract_without_discriminator = resource.is_none()
            && value.get("document_id").is_some()
            && value.get("status").is_some();

        match resource {
            Some("document_contract") => serde_json::from_value(value)
                .map(Box::new)
                .map(Self::Contract)
                .map_err(D::Error::custom),
            Some("document_contract_amendment") => serde_json::from_value(value)
                .map(Box::new)
                .map(Self::Amendment)
                .map_err(D::Error::custom),
            None if amendment_without_discriminator => serde_json::from_value(value)
                .map(Box::new)
                .map(Self::Amendment)
                .map_err(D::Error::custom),
            None if contract_without_discriminator => serde_json::from_value(value)
                .map(Box::new)
                .map(Self::Contract)
                .map_err(D::Error::custom),
            _ => Ok(Self::Unknown(value)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn details_use_the_resource_discriminator_and_preserve_unknown_kinds() {
        let contract: DocumentContractDetails = serde_json::from_value(serde_json::json!({
            "resource": "document_contract",
            "document_id": "document",
            "status": "active"
        }))
        .unwrap();
        assert!(matches!(contract, DocumentContractDetails::Contract(_)));

        let amendment: DocumentContractDetails = serde_json::from_value(serde_json::json!({
            "resource": "document_contract_amendment",
            "document_id": "amendment",
            "parent_document_id": "document",
            "relation_type": "amendment",
            "status": "pending",
            "document_status": "pending_signature"
        }))
        .unwrap();
        assert!(matches!(amendment, DocumentContractDetails::Amendment(_)));

        let unknown: DocumentContractDetails = serde_json::from_value(serde_json::json!({
            "resource": "future_contract_resource",
            "id": "future"
        }))
        .unwrap();
        assert!(matches!(unknown, DocumentContractDetails::Unknown(_)));
    }
}
