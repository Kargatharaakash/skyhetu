use std::cell::RefCell;
use std::collections::{HashSet, HashMap};
use crate::value::Value;

/// A safe handle to a heap-allocated object.
/// This acts as an index into the Heap's object storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Handle(pub usize);

/// State of an upvalue
#[derive(Debug, Clone)]
pub enum UpvalueState {
    /// Points to a stack slot (index)
    Open(usize),
    /// Contains a closed-over value
    Closed(Value),
}

/// Upvalue object
#[derive(Debug, Clone)]
pub struct Upvalue {
    pub location: RefCell<UpvalueState>,
}

/// Closure object
#[derive(Debug, Clone)]
pub struct Closure {
    pub function: Handle, // Handle to Object::Function (the prototype)
    pub upvalues: Vec<Handle>, // Handles to Object::Upvalue
}

/// Class object
#[derive(Debug, Clone)]
pub struct Class {
    pub name: String,
    pub methods: HashMap<String, Handle>, // Name -> Closure/Function
}

/// Instance object
#[derive(Debug, Clone)]
pub struct Instance {
    pub class: Handle, // Handle to Object::Class
    pub fields: RefCell<HashMap<String, Value>>,
}

/// Bound Method object (receiver + closure)
#[derive(Debug, Clone)]
pub struct BoundMethod {
    pub receiver: Value, // The instance (or any value if we support extensions)
    pub method: Handle,  // The closure
}

pub struct Heap {
    objects: Vec<Option<Object>>,
    free_list: Vec<usize>,
    marked: HashSet<usize>,
    grey_stack: Vec<Handle>,
    
    /// String interner for deduplication
    interned_strings: HashMap<String, Handle>,
    
    pub bytes_allocated: usize,
    pub next_gc: usize,
}

pub enum Object {
    String(String),
    Function(crate::value::Function),
    Array(Vec<Value>),
    Closure(Closure),
    Upvalue(Upvalue),
    Class(Class),
    Instance(Instance),
    BoundMethod(BoundMethod),
}

impl Object {
    pub fn children(&self) -> Vec<Handle> {
        match self {
            Object::String(_) => vec![],
            Object::Function(_f) => {
                // Constants trace roots
                vec![]
            },
            Object::Array(arr) => {
                let mut children = Vec::new();
                for val in arr {
                    children.extend(val.children());
                }
                children
            }
            Object::Closure(c) => {
                let mut children = vec![c.function];
                children.extend(c.upvalues.iter().cloned());
                children
            }
            Object::Upvalue(u) => {
                match &*u.location.borrow() {
                    UpvalueState::Closed(v) => v.children(),
                    UpvalueState::Open(_) => vec![], // Open upvalues point to stack (traced by VM)
                }
            }
            Object::Class(c) => {
                // Methods are children (Closures)
                c.methods.values().cloned().collect()
            }
            Object::Instance(i) => {
                let mut children = vec![i.class];
                for val in i.fields.borrow().values() {
                    children.extend(val.children());
                }
                children
            }
            Object::BoundMethod(b) => {
                let mut children = b.receiver.children();
                children.push(b.method);
                children
            }
        }
    }
    
    pub fn size_bytes(&self) -> usize {
        match self {
            Object::String(s) => std::mem::size_of::<Object>() + s.len(),
            Object::Function(_f) => std::mem::size_of::<Object>() + std::mem::size_of::<crate::value::Function>(),
            Object::Array(arr) => std::mem::size_of::<Object>() + arr.len() * std::mem::size_of::<Value>(),
            Object::Closure(c) => std::mem::size_of::<Object>() + std::mem::size_of::<Closure>() + c.upvalues.len() * std::mem::size_of::<Handle>(),
            Object::Upvalue(_) => std::mem::size_of::<Object>() + std::mem::size_of::<Upvalue>(),
            Object::Class(c) => std::mem::size_of::<Object>() + std::mem::size_of::<Class>() + c.name.len() + c.methods.len() * (std::mem::size_of::<String>() + std::mem::size_of::<Handle>()),
            Object::Instance(i) => std::mem::size_of::<Object>() + std::mem::size_of::<Instance>() + i.fields.borrow().len() * (std::mem::size_of::<String>() + std::mem::size_of::<Value>()),
            Object::BoundMethod(_) => std::mem::size_of::<Object>() + std::mem::size_of::<BoundMethod>(),
        }
    }
}

impl Heap {
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
            free_list: Vec::new(),
            marked: HashSet::new(),
            grey_stack: Vec::new(),
            interned_strings: HashMap::new(),
            bytes_allocated: 0,
            next_gc: 1024 * 1024, // Start at 1MB
        }
    }
    
    /// Allocate or return existing interned string
    pub fn alloc_string(&mut self, s: String) -> Handle {
        // Check if string is already interned
        if let Some(&handle) = self.interned_strings.get(&s) {
            return handle;
        }
        
        // Allocate new string and intern it
        let handle = self.alloc(Object::String(s.clone()));
        self.interned_strings.insert(s, handle);
        handle
    }
    
    pub fn alloc_function(&mut self, f: crate::value::Function) -> Handle {
        self.alloc(Object::Function(f))
    }
    
    pub fn alloc_array(&mut self, arr: Vec<Value>) -> Handle {
        self.alloc(Object::Array(arr))
    }
    
    pub fn alloc_closure(&mut self, function: Handle, upvalues: Vec<Handle>) -> Handle {
        self.alloc(Object::Closure(Closure { function, upvalues }))
    }
    
    pub fn alloc_upvalue(&mut self, slot: usize) -> Handle {
        self.alloc(Object::Upvalue(Upvalue { location: RefCell::new(UpvalueState::Open(slot)) }))
    }
    
    pub fn alloc_class(&mut self, name: String) -> Handle {
        self.alloc(Object::Class(Class { name, methods: HashMap::new() }))
    }
    
    pub fn alloc_instance(&mut self, class: Handle) -> Handle {
        self.alloc(Object::Instance(Instance { class, fields: RefCell::new(HashMap::new()) }))
    }
    
    pub fn alloc_bound_method(&mut self, receiver: Value, method: Handle) -> Handle {
        self.alloc(Object::BoundMethod(BoundMethod { receiver, method }))
    }
    
    fn alloc(&mut self, obj: Object) -> Handle {
        let size = obj.size_bytes();
        self.bytes_allocated += size;
        
        // Simple threshold trigger would go here, but VM orchestrates it
        
        if let Some(idx) = self.free_list.pop() {
            self.objects[idx] = Some(obj);
            Handle(idx)
        } else {
            let idx = self.objects.len();
            self.objects.push(Some(obj));
            Handle(idx)
        }
    }
    
    pub fn get_string(&self, handle: Handle) -> Option<&String> {
        match self.objects.get(handle.0)? {
            Some(Object::String(s)) => Some(s),
            _ => None,
        }
    }

    pub fn get_function(&self, handle: Handle) -> Option<&crate::value::Function> {
        match self.objects.get(handle.0)? {
            Some(Object::Function(f)) => Some(f),
            _ => None,
        }
    }

    pub fn get_array(&self, handle: Handle) -> Option<&Vec<Value>> {
        match self.objects.get(handle.0)? {
            Some(Object::Array(arr)) => Some(arr),
            _ => None,
        }
    }
    
    pub fn get_array_mut(&mut self, handle: Handle) -> Option<&mut Vec<Value>> {
        match self.objects.get_mut(handle.0)? {
            Some(Object::Array(arr)) => Some(arr),
            _ => None,
        }
    }
    
    pub fn get_closure(&self, handle: Handle) -> Option<&Closure> {
        match self.objects.get(handle.0)? {
            Some(Object::Closure(c)) => Some(c),
            _ => None,
        }
    }
    
    pub fn get_upvalue(&self, handle: Handle) -> Option<&Upvalue> {
        match self.objects.get(handle.0)? {
            Some(Object::Upvalue(u)) => Some(u),
            _ => None,
        }
    }
    
    pub fn get_class(&self, handle: Handle) -> Option<&Class> {
        match self.objects.get(handle.0)? {
            Some(Object::Class(c)) => Some(c),
            _ => None,
        }
    }
    
    pub fn get_class_mut(&mut self, handle: Handle) -> Option<&mut Class> {
        match self.objects.get_mut(handle.0)? {
            Some(Object::Class(c)) => Some(c),
            _ => None,
        }
    }
    
    pub fn get_instance(&self, handle: Handle) -> Option<&Instance> {
        match self.objects.get(handle.0)? {
            Some(Object::Instance(i)) => Some(i),
            _ => None,
        }
    }
    
    pub fn get_bound_method(&self, handle: Handle) -> Option<&BoundMethod> {
        match self.objects.get(handle.0)? {
            Some(Object::BoundMethod(b)) => Some(b),
            _ => None,
        }
    }
    
    pub fn is_marked(&self, handle: Handle) -> bool {
        self.marked.contains(&handle.0)
    }

    pub fn should_collect(&self) -> bool {
        self.bytes_allocated > self.next_gc
    }

    
    pub fn mark(&mut self, handle: Handle) {
        if self.marked.contains(&handle.0) {
            return;
        }
        
        if self.objects.get(handle.0).and_then(|o| o.as_ref()).is_some() {
            self.marked.insert(handle.0);
            self.grey_stack.push(handle);
        }
    }
    
    pub fn trace_references(&mut self) {
        while let Some(handle) = self.grey_stack.pop() {
            // Get children. Note: we cannot borrow self.objects while calling self.mark
            // So we extract children first.
            let children = if let Some(Some(obj)) = self.objects.get(handle.0) {
                obj.children()
            } else {
                Vec::new()
            };
            
            for child in children {
                self.mark(child);
            }
        }
    }
    
    pub fn sweep(&mut self) {
        let mut freed_bytes = 0;
        
        for i in 0..self.objects.len() {
            if !self.marked.contains(&i) {
                if let Some(obj) = &self.objects[i] {
                    freed_bytes += obj.size_bytes();
                    self.objects[i] = None;
                    self.free_list.push(i);
                }
            }
        }
        
        // Clean up interned strings that were freed
        self.interned_strings.retain(|_, &mut handle| {
            self.marked.contains(&handle.0)
        });
        
        self.bytes_allocated -= freed_bytes;
        self.marked.clear();
        
        // Adjust threshold
        self.next_gc = std::cmp::max(self.bytes_allocated * 2, 1024 * 1024);
    }
}
