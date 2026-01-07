use skyhetu::run;
use skyhetu::value::Value;

#[test]
fn test_basic_closure() {
    let source = r#"
        let x = "global"
        fn makeClosure() {
            let y = "captured"
            fn inner() {
                return x + " " + y
            }
            return inner
        }
        let closure = makeClosure()
        let result = closure()
        result
    "#;
    let result = run(source).expect("Execution failed");
    match result {
        Value::String(s) => assert_eq!(s, "global captured"),
        _ => panic!("Expected string, got {:?}", result),
    }
}

#[test]
fn test_counter_state() {
    // Note: arrays/objects can't be inspected after run() drops VM, 
    // so we return string representation.
    let source = r#"
        fn makeCounter() {
            state i = 0
            fn count() {
                i -> i + 1
                return i
            }
            return count
        }
        let c1 = makeCounter()
        let c2 = makeCounter()
        let r1 = c1()
        let r2 = c1()
        let r3 = c2()
        
        "" + r1 + "," + r2 + "," + r3
    "#;
    let result = run(source).expect("Execution failed");
    match result {
        Value::String(s) => assert_eq!(s, "1,2,1"), // Numbers are floats formatted as X.X usually
        _ => panic!("Expected string, got {:?}", result),
    }
}

#[test]
fn test_close_upvalue() {
    // Test that upvalues are correctly closed (values moved to heap) when stack frame pops
    let source = r#"
        fn loop() {
            let i = 0 
           
            fn make() {
                let a = "first"
                fn f() { return a }
                return f
            }
            
            let f1 = make()
            // make() returned, 'a' should be closed.
            return f1()
        }
        loop()
    "#;
    let result = run(source).expect("Execution failed");
    match result {
        Value::String(s) => assert_eq!(s, "first"),
        _ => panic!("Expected string, got {:?}", result),
    }
}
