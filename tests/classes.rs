//! Integration tests for classes and instances

use skyhetu::{Lexer, Parser};
use skyhetu::compiler::Compiler;
use skyhetu::vm::VM;

fn run(source: &str) -> Result<skyhetu::Value, String> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().map_err(|e| e.to_string())?;
    
    let mut parser = Parser::new(tokens);
    let program = parser.parse().map_err(|e| e.to_string())?;
    
    let mut vm = VM::new();
    let mut compiler = Compiler::new();
    let (chunk, chunks) = compiler.compile(&program, &mut vm.heap).map_err(|e| e.to_string())?;
    
    vm.register_chunks(chunks);
    vm.run(chunk).map_err(|e| e.to_string())
}

fn run_ok(source: &str) -> skyhetu::Value {
    run(source).expect("execution failed")
}

#[test]
fn test_class_instantiation() {
    let result = run_ok(r#"
        class Box {
            init(value) {
                this.value = value
            }
        }
        let b = Box(42)
        b.value
    "#);
    
    assert!(matches!(result, skyhetu::Value::Number(n) if n == 42.0));
}

#[test]
fn test_class_method() {
    let result = run_ok(r#"
        class Counter {
            init() {
                this.count = 0
            }
            inc() {
                this.count = this.count + 1
                return this.count
            }
        }
        let c = Counter()
        c.inc()
        c.inc()
        c.inc()
    "#);
    
    assert!(matches!(result, skyhetu::Value::Number(n) if n == 3.0));
}

#[test]
fn test_property_set() {
    let result = run_ok(r#"
        class Point {
            init(x, y) {
                this.x = x
                this.y = y
            }
        }
        let p = Point(1, 2)
        p.x = 10
        p.x
    "#);
    
    assert!(matches!(result, skyhetu::Value::Number(n) if n == 10.0));
}

#[test]
fn test_this_binding() {
    let result = run_ok(r#"
        class Person {
            init(name) {
                this.name = name
            }
            greet() {
                return this.name
            }
        }
        let p = Person("Alice")
        p.greet()
    "#);
    
    if let skyhetu::Value::String(s) = result {
        assert_eq!(s, "Alice");
    } else {
        panic!("Expected string, got {:?}", result);
    }
}

#[test]
fn test_init_returns_instance() {
    // Verify that init implicitly returns this
    let result = run_ok(r#"
        class Wrapper {
            init(v) {
                this.v = v
            }
        }
        let w = Wrapper(100)
        w.v
    "#);
    
    assert!(matches!(result, skyhetu::Value::Number(n) if n == 100.0));
}

#[test]
fn test_multiple_instances() {
    let result = run_ok(r#"
        class Counter {
            init(start) {
                this.val = start
            }
            add(n) {
                this.val = this.val + n
                return this.val
            }
        }
        let a = Counter(10)
        let b = Counter(20)
        a.add(5)
        b.add(3)
        a.val + b.val
    "#);
    
    assert!(matches!(result, skyhetu::Value::Number(n) if n == 38.0)); // 15 + 23
}

#[test]
fn test_class_no_init() {
    let result = run_ok(r#"
        class Empty {}
        let e = Empty()
        e.x = 42
        e.x
    "#);
    
    assert!(matches!(result, skyhetu::Value::Number(n) if n == 42.0));
}

#[test]
fn test_method_chaining() {
    let result = run_ok(r#"
        class Builder {
            init() {
                this.val = 0
            }
            add(n) {
                this.val = this.val + n
                return this
            }
            result() {
                return this.val
            }
        }
        let b = Builder()
        b.add(1).add(2).add(3).result()
    "#);
    
    assert!(matches!(result, skyhetu::Value::Number(n) if n == 6.0));
}
