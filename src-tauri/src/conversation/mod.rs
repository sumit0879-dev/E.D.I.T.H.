pub mod context;
pub mod core;
pub mod errors;
pub mod turn;
pub mod types;

#[cfg(test)]
mod tests;

pub use context::{ContextAssembler, MemoryRetriever, NoopMemoryRetriever, UserProfile};
pub use core::ConversationCore;
pub use errors::ConversationError;
pub use turn::{Turn, TurnStatus};
pub use types::{
    ConversationMessage, MessageRole, ModelSelection, SessionId, TurnSnapshot,
    TurnSubmissionRequest, TurnSubmissionResult,
};
