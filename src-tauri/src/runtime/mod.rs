//! runtime — E.D.I.T.H. Runtime State, Coordination & Self-Knowledge Layer.
//!
//! Provides a non-duplicating observation and coordination layer that aggregates canonical
//! state from all active subsystems (ConversationCore, TaskRuntime, ToolRouter, PolicyEngine,
//! ProviderRegistry, BrowserState, BrowserControlManager, ComputerControlManager).

pub mod autonomy;
pub mod projections;
pub mod state;

pub use autonomy::AutonomyState;
pub use projections::*;
pub use state::EdithRuntimeState;
