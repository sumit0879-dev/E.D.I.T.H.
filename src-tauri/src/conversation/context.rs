use crate::ai::ChatMessage;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Profile metadata describing the human operator.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserProfile {
    pub nickname: Option<String>,
    pub occupation: Option<String>,
    pub more_about_you: Option<String>,
}

/// Pluggable memory retrieval boundary decoupling Conversation Core from LanceDB.
pub trait MemoryRetriever: Send + Sync {
    fn retrieve_context<'a>(
        &'a self,
        query: &'a str,
    ) -> Pin<Box<dyn Future<Output = Vec<String>> + Send + 'a>>;
}

/// Default no-op memory retriever when vector memory is unconfigured or disabled.
pub struct NoopMemoryRetriever;

impl MemoryRetriever for NoopMemoryRetriever {
    fn retrieve_context<'a>(
        &'a self,
        _query: &'a str,
    ) -> Pin<Box<dyn Future<Output = Vec<String>> + Send + 'a>> {
        Box::pin(async move { Vec::new() })
    }
}

/// In-memory mock memory retriever for deterministic unit testing.
pub struct StaticMemoryRetriever {
    results: Vec<String>,
}

impl StaticMemoryRetriever {
    pub fn new(results: Vec<String>) -> Self {
        Self { results }
    }
}

impl MemoryRetriever for StaticMemoryRetriever {
    fn retrieve_context<'a>(
        &'a self,
        _query: &'a str,
    ) -> Pin<Box<dyn Future<Output = Vec<String>> + Send + 'a>> {
        let res = self.results.clone();
        Box::pin(async move { res })
    }
}

/// Context boundary builder for assembling model inputs for a conversational turn.
pub struct ContextAssembler {
    system_instructions: Option<String>,
    user_profile: Option<UserProfile>,
    memory_retriever: Arc<dyn MemoryRetriever>,
}

impl Default for ContextAssembler {
    fn default() -> Self {
        Self::new(None, None, Arc::new(NoopMemoryRetriever))
    }
}

impl ContextAssembler {
    pub fn new(
        system_instructions: Option<String>,
        user_profile: Option<UserProfile>,
        memory_retriever: Arc<dyn MemoryRetriever>,
    ) -> Self {
        Self {
            system_instructions,
            user_profile,
            memory_retriever,
        }
    }

    /// Builds a formatted system instruction string incorporating persona, user profile, and memory.
    pub fn build_system_prompt(&self, memory_items: &[String]) -> String {
        let mut sys = match &self.system_instructions {
            Some(instr) if !instr.trim().is_empty() => instr.trim().to_string(),
            _ => "You are E.D.I.T.H. (Even Dead, I'm The Hero), an advanced Stark-grade AI PC assistant. Keep responses clear, helpful, intelligent, and friendly. Always wrap code in Markdown triple backticks.".to_string(),
        };

        if let Some(ref profile) = self.user_profile {
            let mut info = Vec::new();
            if let Some(ref name) = profile.nickname {
                if !name.trim().is_empty() {
                    info.push(format!("- Name: {}", name.trim()));
                }
            }
            if let Some(ref occ) = profile.occupation {
                if !occ.trim().is_empty() {
                    info.push(format!("- Occupation: {}", occ.trim()));
                }
            }
            if let Some(ref about) = profile.more_about_you {
                if !about.trim().is_empty() {
                    info.push(format!("- About user: {}", about.trim()));
                }
            }
            if !info.is_empty() {
                sys.push_str("\n\nUser Information:\n");
                sys.push_str(&info.join("\n"));
            }
        }

        if !memory_items.is_empty() {
            sys.push_str("\n\n[Stored Knowledge / Memory Context]:\n");
            for item in memory_items {
                sys.push_str(&format!("- {}\n", item));
            }
            sys.push_str("\nUse the above context if relevant to answer the user.");
        }

        sys
    }

    /// Asynchronously retrieves relevant semantic memory items using the pluggable boundary.
    pub async fn retrieve_memory(&self, query: &str) -> Vec<String> {
        self.memory_retriever.retrieve_context(query).await
    }

    /// Assembles the complete list of ChatMessage items for provider consumption.
    pub async fn assemble_messages(
        &self,
        history: &[ChatMessage],
        current_input: &str,
    ) -> Vec<ChatMessage> {
        let memory_items = self.retrieve_memory(current_input).await;
        let system_prompt = self.build_system_prompt(&memory_items);

        let mut messages = Vec::with_capacity(history.len() + 2);
        messages.push(ChatMessage::system(system_prompt));

        for msg in history {
            // Do not duplicate system prompts from history
            if msg.role != "system" {
                messages.push(msg.clone());
            }
        }

        messages.push(ChatMessage::user(current_input.to_string()));

        messages
    }
}
