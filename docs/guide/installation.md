# Installation & Building

SkyHetu is written in Rust, so you will need the Rust toolchain installed.

## Prerequisites

- **Rust & Cargo**: Latest stable version.
  - Install via [rustup](https://rustup.rs): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`

## Building from Source

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/your-username/skyhetu-lang.git
    cd skyhetu-lang
    ```

2.  **Build in Release mode:**
    ```bash
    cargo build --release
    ```

    The compiled binary will be located at `./target/release/skyhetu`.

## Running

You can add the binary to your PATH or run it directly.

```bash
# Run a script
./target/release/skyhetu run examples/hello.skyh

# Run the REPL (Interactive Shell)
./target/release/skyhetu repl
```

## Editor Support

Currently, there is no official VS Code extension, but you can use the **Rust** or **JavaScript** syntax highlighting as a temporary measure, as the syntax is similar to Rust/JS.
