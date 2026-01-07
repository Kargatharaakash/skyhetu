//! SkyHetu CLI and REPL
//!
//! Usage:
//!   skyhetu run <file.sky>   - Execute a SkyHetu file
//!   skyhetu repl             - Start interactive REPL
//!   skyhetu help             - Show help message

use std::env;
use std::fs;
use std::process;
use colored::Colorize;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

use skyhetu::{Lexer, Parser, VERSION};

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        print_help();
        return;
    }
    
    match args[1].as_str() {
        "run" => {
            if args.len() < 3 {
                eprintln!("{}: missing file argument", "error".red());
                eprintln!("Usage: skyhetu run <file.sky>");
                process::exit(1);
            }
            run_file(&args[2]);
        }
        "repl" => run_repl(),
        "help" | "--help" | "-h" => print_help(),
        "version" | "--version" | "-v" => println!("SkyHetu {}", VERSION),
        _ => {
            // Assume it's a file
            if args[1].ends_with(".skyh") {
                run_file(&args[1]);
            } else {
                eprintln!("{}: unknown command '{}'", "error".red(), args[1]);
                print_help();
                process::exit(1);
            }
        }
    }
}

fn print_help() {
    println!("{}", "SkyHetu".cyan().bold());
    println!("A causality-first programming language");
    println!("{} {}\n", "Version".cyan(), VERSION);
    println!("{}", "USAGE:".yellow());
    println!("  skyhetu run <file.skyh>   Execute a SkyHetu file");
    println!("  skyhetu repl             Start interactive REPL");
    println!("  skyhetu help             Show this help message");
    println!("  skyhetu version          Show version\n");
    println!("{}", "EXAMPLES:".yellow());
    println!("  skyhetu run examples/hello.skyh");
    println!("  skyhetu repl\n");
    println!("{}", "LANGUAGE FEATURES:".yellow());
    println!("  let x = 10               Immutable binding");
    println!("  state y = 0              Mutable state");
    println!("  y -> y + 1               State transition (tracked)");
    println!("  why(y)                   Query causality chain");
    println!("  fn f(a) {{ return a }}     Function definition");
}

fn run_file(path: &str) {
    let source = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("{}: cannot read file '{}': {}", "error".red(), path, e);
            process::exit(1);
        }
    };
    
    let mut lexer = Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => {
            let err = e.with_source(&source);
            eprintln!("{}", err);
            process::exit(1);
        }
    };
    
    let mut parser = Parser::new(tokens);
    let program = match parser.parse() {
        Ok(p) => p,
        Err(e) => {
            let err = e.with_source(&source);
            eprintln!("{}", err);
            process::exit(1);
        }
    };
    
    let mut vm = skyhetu::vm::VM::new();
    
    // Get the base path for module resolution
    let base_path = std::path::Path::new(path)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    
    let mut compiler = skyhetu::compiler::Compiler::with_base_path(base_path);
    let (chunk, chunks) = match compiler.compile(&program, &mut vm.heap) {
        Ok(c) => c,
        Err(e) => {
            let err = e.with_source(&source);
            eprintln!("{}", err);
            process::exit(1);
        }
    };

    vm.register_chunks(chunks);
    
    if let Err(e) = vm.run(chunk) {
        let err = e.with_source(&source);
        eprintln!("{}", err);
        process::exit(1);
    }
}

fn run_repl() {
    println!("{} {} - {}", 
        "SkyHetu".cyan().bold(), 
        VERSION.cyan(),
        "A causality-first language".dimmed()
    );
    println!("Type {} to exit, {} for help\n", 
        "exit".yellow(), 
        "help".yellow()
    );
    
    let mut rl = DefaultEditor::new().expect("Failed to create REPL");
    
    // Persist VM state across REPL lines for globals and causality
    let mut vm = skyhetu::vm::VM::new();
    let mut chunk_count = 0;
    
    loop {
        match rl.readline(&format!("{} ", "sky>".green().bold())) {
            Ok(line) => {
                let line = line.trim();
                
                if line.is_empty() {
                    continue;
                }
                
                let _ = rl.add_history_entry(line);
                
                // Handle special commands
                match line {
                    "exit" | "quit" => {
                        println!("{}", "Goodbye!".cyan());
                        break;
                    }
                    "help" => {
                        print_repl_help();
                        continue;
                    }
                    "clear" => {
                        vm = skyhetu::vm::VM::new();
                        chunk_count = 0;
                        println!("{}", "State cleared.".dimmed());
                        continue;
                    }
                    "history" => {
                        println!("{}", "Use 'print(why(variable))' to see history.".dimmed());
                        continue;
                    }
                    _ => {}
                }
                
                // Tokenize
                let mut lexer = Lexer::new(line);
                let tokens = match lexer.tokenize() {
                    Ok(t) => t,
                    Err(e) => {
                        let err = e.with_source(line);
                        eprintln!("{}", format!("{}", err).red());
                        continue;
                    }
                };
                
                // Parse
                let mut parser = Parser::new(tokens);
                let program = match parser.parse() {
                    Ok(p) => p,
                    Err(e) => {
                        let err = e.with_source(line);
                        eprintln!("{}", format!("{}", err).red());
                        continue;
                    }
                };
                
                // Compile
                let mut compiler = skyhetu::compiler::Compiler::with_offset(chunk_count);
                let (chunk, chunks) = match compiler.compile(&program, &mut vm.heap) {
                    Ok(c) => c,
                    Err(e) => {
                        let err = e.with_source(line);
                        eprintln!("{}", format!("{}", err).red());
                        continue;
                    }
                };
                
                chunk_count += chunks.len();
                vm.register_chunks(chunks);
                
                // Execute
                match vm.run(chunk) {
                    Ok(value) => {
                        if !matches!(value, skyhetu::Value::Nil) {
                            println!("{} {}", "=>".dimmed(), format!("{}", value).cyan());
                        }
                    }
                    Err(e) => {
                        let err = e.with_source(line);
                        eprintln!("{}", format!("{}", err).red());
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("{}", "^C".dimmed());
            }
            Err(ReadlineError::Eof) => {
                println!("{}", "Goodbye!".cyan());
                break;
            }
            Err(err) => {
                eprintln!("{}: {:?}", "error".red(), err);
                break;
            }
        }
    }
}

fn print_repl_help() {
    println!("{}", "REPL Commands:".yellow());
    println!("  exit, quit   Exit the REPL");
    println!("  clear        Clear state and causality history");
    println!("  history      Show all state mutations");
    println!("  help         Show this help\n");
    println!("{}", "Language Examples:".yellow());
    println!("  let x = 10");
    println!("  state counter = 0");
    println!("  counter -> counter + 1");
    println!("  print(why(counter))");
    println!("  fn double(n) {{ return n * 2 }}");
}
