use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Deref;
use uuid::Uuid;

macro_rules! define_id {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            /// Generates a new random collision-resistant UUID v4 identifier.
            pub fn new() -> Self {
                Self(Uuid::new_v4().to_string())
            }

            /// Creates an identifier from an existing string.
            pub fn from_string(s: impl Into<String>) -> Self {
                Self(s.into())
            }

            /// Returns a string slice view of the identifier.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl Deref for $name {
            type Target = str;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(s)
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_string())
            }
        }
    };
}

define_id!(EventId, "Globally unique identifier for a single runtime event.");
define_id!(ConversationId, "Identifies a conversation or session thread.");
define_id!(TurnId, "Identifies a specific conversational turn between user and assistant.");
define_id!(StreamId, "Identifies an individual continuous token-generation stream.");
define_id!(TaskId, "Identifies an autonomous background workflow or agent task.");
define_id!(ToolExecutionId, "Identifies an individual tool proposal or invocation.");
define_id!(VoiceSessionId, "Identifies an active speech-to-speech or pipeline voice session.");
