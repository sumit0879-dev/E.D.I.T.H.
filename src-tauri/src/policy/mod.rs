pub mod adapters;
pub mod approval;
pub mod audit;
pub mod context;
pub mod decision;
pub mod engine;
pub mod types;

#[cfg(test)]
pub mod tests;

pub use adapters::{BrowserAdapter, CommandAdapter};
pub use approval::{ApprovalRequest, ApprovalStatus, ApprovalStore, OperatorDecision};
pub use audit::{AuditRecord, SecurityAuditTrail};
pub use context::{ActionSource, PolicyContext, SecurityMode};
pub use decision::PolicyDecision;
pub use engine::PolicyEngine;
pub use types::{ActionRequest, ActionTarget, PolicyConstraints, PolicyOutcome, RiskLevel};
