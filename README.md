# SkyHetu

> **"The cause that comes from the sky"** — A causality-first programming language

SkyHetu makes **state, time, and causality** explicit. Every mutation is tracked. Every state change is queryable. Debug by asking *why*.

## Documentation

Full documentation is available in the `docs/` directory:

### The Guide (The Book)
- **[1. Introduction](docs/guide/introduction.md)** - Philosophy and "Hello World".
- **[2. Installation](docs/guide/installation.md)** - Getting set up.
- **[3. The Causality Engine](docs/guide/causality.md)** - The "Detective Board", `why()`, and Time Travel.
- **[4. Modules](docs/guide/modules.md)** - Organizing code.

### Reference
- **[Built-in Functions](docs/reference/builtins.md)** - `print`, `type`, `time`, etc.
- **[Grammar](docs/reference/grammar.md)** - Language syntax specification.

## Quick Start

```bash
# Build
cargo build --release

# Run a file
./target/release/skyhetu run examples/hello.skyh

# Interactive REPL
./target/release/skyhetu repl
```

## The "Wow" Factor

```sky
state counter = 0
counter -> counter + 1  // tracked mutation
counter -> counter + 1

// Visualize the thinking process
print(causal_graph("counter", "dot"))
```

*See [The Causality Engine](docs/guide/causality.md) for visualization examples.*

## Project Structure

```
skyhetu/
├── src/               # Rust Source Code (VM, Compiler, GC)
├── examples/          # Example Scripts
├── docs/              # Official Documentation
│   ├── guide/         # Tutorials
│   └── reference/     # API Specs
└── tests/             # Integration Tests
```

## License

MIT
