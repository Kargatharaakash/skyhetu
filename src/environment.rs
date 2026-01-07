//! Variable environment for SkyHetu
//!
//! Handles scoped variable storage with immutability tracking.

use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use crate::value::Value;
use crate::error::{ErrorKind, Result, SkyHetuError};

/// A binding in the environment
#[derive(Debug, Clone)]
struct Binding {
    value: Value,
    mutable: bool, // true for 'state', false for 'let'
}

/// Variable environment with lexical scoping
#[derive(Debug)]
pub struct Environment {
    values: HashMap<String, Binding>,
    parent: Option<Rc<RefCell<Environment>>>,
}

impl Environment {
    /// Create a new global environment
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            parent: None,
        }
    }
    
    /// Create a child environment with parent scope
    pub fn with_parent(parent: Rc<RefCell<Environment>>) -> Self {
        Self {
            values: HashMap::new(),
            parent: Some(parent),
        }
    }
    
    /// Define an immutable variable (let)
    pub fn define(&mut self, name: String, value: Value) {
        self.values.insert(name, Binding { value, mutable: false });
    }
    
    /// Define a mutable state variable
    pub fn define_state(&mut self, name: String, value: Value) {
        self.values.insert(name, Binding { value, mutable: true });
    }
    
    /// Get a variable's value
    pub fn get(&self, name: &str) -> Result<Value> {
        if let Some(binding) = self.values.get(name) {
            Ok(binding.value.clone())
        } else if let Some(parent) = &self.parent {
            parent.borrow().get(name)
        } else {
            Err(SkyHetuError::new(
                ErrorKind::UndefinedVariable(name.to_string()),
                None,
            ))
        }
    }
    
    /// Check if a variable is mutable
    pub fn is_mutable(&self, name: &str) -> Result<bool> {
        if let Some(binding) = self.values.get(name) {
            Ok(binding.mutable)
        } else if let Some(parent) = &self.parent {
            parent.borrow().is_mutable(name)
        } else {
            Err(SkyHetuError::new(
                ErrorKind::UndefinedVariable(name.to_string()),
                None,
            ))
        }
    }
    
    /// Assign to a variable (only if it exists and is mutable)
    pub fn assign(&mut self, name: &str, value: Value) -> Result<()> {
        if let Some(binding) = self.values.get_mut(name) {
            if binding.mutable {
                binding.value = value;
                Ok(())
            } else {
                Err(SkyHetuError::new(
                    ErrorKind::ImmutableVariable(name.to_string()),
                    None,
                ))
            }
        } else if let Some(parent) = &self.parent {
            parent.borrow_mut().assign(name, value)
        } else {
            Err(SkyHetuError::new(
                ErrorKind::UndefinedVariable(name.to_string()),
                None,
            ))
        }
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for Environment {
    fn clone(&self) -> Self {
        Self {
            values: self.values.clone(),
            parent: self.parent.clone(),
        }
    }
}
