# Language Grammar (EBNF-ish)

This document describes the syntax of SkyHetu v0.2.0.

```ebnf
program        ::= declaration* EOF

declaration    ::= classDecl
                 | funDecl
                 | varDecl
                 | statement

classDecl      ::= "class" IDENTIFIER "{" function* "}"
funDecl        ::= "export"? "fn" function
varDecl        ::= "export"? "let" IDENTIFIER "=" expression
                 | "export"? "state" IDENTIFIER "=" expression
                 
statement      ::= exprStmt
                 | forStmt
                 | ifStmt
                 | returnStmt
                 | whileStmt
                 | block
                 | transitionStmt  // Key feature!
                 
transitionStmt ::= IDENTIFIER "->" expression  // State mutation

exprStmt       ::= expression
forStmt        ::= "for" IDENTIFIER "in" expression block
ifStmt         ::= "if" expression block ("else" block)?
returnStmt     ::= "return" expression?
whileStmt      ::= "while" expression block
block          ::= "{" declaration* "}"

expression     ::= assignment
assignment     ::= logic_or
logic_or       ::= logic_and ( "or" logic_and )*
logic_and      ::= equality ( "and" equality )*
equality       ::= comparison ( ( "!=" | "==" ) comparison )*
comparison     ::= term ( ( ">" | ">=" | "<" | "<=" ) term )*
term           ::= factor ( ( "-" | "+" ) factor )*
factor         ::= unary ( ( "/" | "*" ) unary )*

unary          ::= ( "!" | "-" ) unary | call
call           ::= primary ( "(" arguments? ")" | "." IDENTIFIER )*
primary        ::= "true" | "false" | "nil" | NUMBER | STRING
                 | IDENTIFIER | "(" expression ")"
                 | "import" "{" IDENTIFIER ("," IDENTIFIER)* "}" "from" STRING
```

## Notes

- **Precedence:** Standard C-style precedence.
- **State vs Let:** usage is enforced semantically, not just syntactically. Attempting to assign to a `let` variable typically fails at compile/runtime based on scope checks.
- **-> Operator:** Distinct from generic assignment `=`. Reserved for `state` variables.
