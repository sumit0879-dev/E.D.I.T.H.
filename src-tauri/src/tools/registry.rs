use super::types::{ToolDefinition, ToolDomain};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// The central, authoritative registry managing registered tool contracts.
#[derive(Clone, Default)]
pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, Arc<ToolDefinition>>>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Registers a tool definition. Fails if a tool with identical name is already registered.
    pub fn register(&self, def: ToolDefinition) -> Result<(), String> {
        let mut lock = self.tools.write().expect("ToolRegistry lock poisoned");
        if lock.contains_key(&def.name) {
            return Err(format!("Tool '{}' is already registered.", def.name));
        }
        lock.insert(def.name.clone(), Arc::new(def));
        Ok(())
    }

    /// Registers or overwrites a tool definition.
    pub fn register_or_update(&self, def: ToolDefinition) {
        let mut lock = self.tools.write().expect("ToolRegistry lock poisoned");
        lock.insert(def.name.clone(), Arc::new(def));
    }

    /// Retrieves an immutable reference to a tool definition by name.
    pub fn get(&self, name: &str) -> Option<Arc<ToolDefinition>> {
        let lock = self.tools.read().expect("ToolRegistry lock poisoned");
        lock.get(name).cloned()
    }

    /// Checks if a tool is registered.
    pub fn contains(&self, name: &str) -> bool {
        let lock = self.tools.read().expect("ToolRegistry lock poisoned");
        lock.contains_key(name)
    }

    /// Unregisters a tool by name, returning true if it existed.
    pub fn unregister(&self, name: &str) -> bool {
        let mut lock = self.tools.write().expect("ToolRegistry lock poisoned");
        lock.remove(name).is_some()
    }

    /// Lists all registered tool definitions.
    pub fn list(&self) -> Vec<ToolDefinition> {
        let lock = self.tools.read().expect("ToolRegistry lock poisoned");
        let mut list: Vec<ToolDefinition> = lock.values().map(|t| (**t).clone()).collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        list
    }

    /// Lists all tools belonging to a specific domain.
    pub fn list_by_domain(&self, domain: &ToolDomain) -> Vec<ToolDefinition> {
        let lock = self.tools.read().expect("ToolRegistry lock poisoned");
        let mut list: Vec<ToolDefinition> = lock
            .values()
            .filter(|t| &t.domain == domain)
            .map(|t| (**t).clone())
            .collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        list
    }

    /// Returns the number of registered tools.
    pub fn count(&self) -> usize {
        let lock = self.tools.read().expect("ToolRegistry lock poisoned");
        lock.len()
    }

    /// Clears all registered tools (primarily used in test fixtures).
    pub fn clear(&self) {
        let mut lock = self.tools.write().expect("ToolRegistry lock poisoned");
        lock.clear();
    }
}
