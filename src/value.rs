//! Runtime value types for SkyHetu

use std::fmt;
use crate::gc::Heap;

/// Runtime values in SkyHetu
#[derive(Clone)]
pub enum Value {
    /// Numeric value
    Number(f64),
    
    /// String value
    String(String),
    
    /// Boolean value
    Bool(bool),
    
    /// Nil/null value
    Nil,
    
    /// User-defined function (Prototype/Code)
    Function(crate::gc::Handle),
    
    /// Closure (Runtime Function Instance)
    Closure(crate::gc::Handle),
    
    /// Built-in function
    NativeFunction(NativeFn),
    
    /// Array/list
    Array(crate::gc::Handle),
    
    /// Class definition
    Class(crate::gc::Handle),
    
    /// Class instance
    Instance(crate::gc::Handle),
    
    /// Bound Method
    BoundMethod(crate::gc::Handle),
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Bool(_) => "bool",
            Value::Nil => "nil",
            Value::Function(_) => "function",
            Value::Closure(_) => "closure",
            Value::NativeFunction(_) => "native function",
            Value::Array(_) => "array",
            Value::Class(_) => "class",
            Value::Instance(_) => "instance",
            Value::BoundMethod(_) => "method",
        }
    }
    
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Nil => false,
            Value::Bool(b) => *b,
            Value::Number(n) => *n != 0.0,
            Value::String(s) => !s.is_empty(),
            _ => true,
        }
    }


    pub fn display(&self, heap: &Heap) -> String {
        match self {
            Value::Number(n) => format!("{}", n),
            Value::String(s) => s.clone(), 
            Value::Bool(b) => format!("{}", b),
            Value::Nil => "nil".to_string(),
            Value::Function(handle) => {
                if let Some(f) = heap.get_function(*handle) {
                    format!("<fn {}>", f.name)
                } else {
                    "<fn (collected)>".to_string()
                }
            }
            Value::Closure(handle) => {
                if let Some(c) = heap.get_closure(*handle) {
                    if let Some(f) = heap.get_function(c.function) {
                        format!("<fn {}>", f.name)
                    } else {
                         "<fn (collected)>".to_string()
                    }
                } else {
                    "<closure (collected)>".to_string()
                }
            }
            Value::NativeFunction(nf) => format!("<native fn {}>", nf.name),
            Value::Array(_handle) => {
                "<array>".to_string() 
            },
            Value::Class(handle) => {
                if let Some(c) = heap.get_class(*handle) {
                    format!("<class {}>", c.name)
                } else {
                    "<class (collected)>".to_string()
                }
            },
            Value::Instance(handle) => {
                if let Some(i) = heap.get_instance(*handle) {
                    if let Some(c) = heap.get_class(i.class) {
                        format!("<{} instance>", c.name)
                    } else {
                        "<instance (class collected)>".to_string()
                    }
                } else {
                     "<instance (collected)>".to_string()
                }
            },
            Value::BoundMethod(handle) => {
                 if let Some(b) = heap.get_bound_method(*handle) {
                    let mut s = "<method".to_string();
                    if let Some(c) = heap.get_closure(b.method) {
                        if let Some(f) = heap.get_function(c.function) {
                            s.push_str(" ");
                            s.push_str(&f.name);
                        }
                    }
                    s.push_str(">");
                    s
                 } else {
                     "<method (collected)>".to_string()
                 }
            }
        }
    }

    pub fn children(&self) -> Vec<crate::gc::Handle> {
        match self {
            Value::Function(handle) => vec![*handle],
            Value::Closure(handle) => vec![*handle],
            Value::Array(handle) => vec![*handle],
            Value::Class(handle) => vec![*handle],
            Value::Instance(handle) => vec![*handle],
            Value::BoundMethod(handle) => vec![*handle],
            _ => vec![],
        }
    }
}



impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{}", n),
            Value::String(s) => write!(f, "{}", s),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Nil => write!(f, "nil"),
            Value::Function(_) => write!(f, "<fn>"),
            Value::Closure(_) => write!(f, "<fn>"), // Cannot access name without heap
            Value::NativeFunction(nf) => write!(f, "<native fn {}>", nf.name),
            Value::Array(_) => write!(f, "<array>"), // Cannot access elements without heap
            Value::Class(_) => write!(f, "<class>"),
            Value::Instance(_) => write!(f, "<instance>"),
            Value::BoundMethod(_) => write!(f, "<method>"),
        }
    }

}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Nil, Value::Nil) => true,
            _ => false,
        }
    }
}

/// User-defined function
#[derive(Debug, Clone)]
pub struct Function {
    // Closure environment (legacy? keeping for now if used, but likely unused by bytecode VM)
    // Actually, let's remove legacy env if possible, or keep it if unsure.
    // I recall checking vm.rs and it doesn't use it.
    // But let's check if it's used in type checking/resolution.
    // Safest is to keep what is there unless I know.
    // But `chunk_index` is definitely being replaced.
    
    // Changing to:
    pub chunk: std::rc::Rc<crate::bytecode::Chunk>,
    pub upvalue_count: usize,
    pub name: String, // moved for packing? no, just keep order
    pub params: Vec<String>,
}

impl Function {
    pub fn new(
        name: String, 
        params: Vec<String>, 
        chunk: std::rc::Rc<crate::bytecode::Chunk>,
        upvalue_count: usize,
    ) -> Self {
        Self { 
            name, 
            params, 
            chunk, 
            upvalue_count, 
        }
    }
    
    pub fn children(&self) -> Vec<crate::gc::Handle> {
        vec![] // No heap references yet (Environment is Rc)
    }
}

/// Native function type
pub type NativeFnPtr = fn(&mut crate::vm::VM, &[Value]) -> Result<Value, String>;

/// Native/built-in function
#[derive(Clone)]
pub struct NativeFn {
    pub name: String,
    pub arity: Option<usize>, // None means variadic
    pub func: NativeFnPtr,
}

impl NativeFn {
    pub fn new(name: &str, arity: Option<usize>, func: NativeFnPtr) -> Self {
        Self {
            name: name.to_string(),
            arity,
            func,
        }
    }
}

impl fmt::Debug for NativeFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<native fn {}>", self.name)
    }
}
