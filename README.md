# Solnix Compiler

**Verifier-Safe eBPF Compiler for High-Assurance Linux Kernel Programs**

A modern domain-specific language and compiler that generates safe,
predictable eBPF C backend code.

------------------------------------------------------------------------

## Overview

Solnix is a high-level domain-specific language designed for
writing Linux eBPF programs safely and concisely.

The Solnix Compiler translates `.snx` source files into
verifier-compliant C backend code, enabling predictable compilation into
eBPF object files suitable for:

-   Kernel tracing
-   Security enforcement
-   Observability pipelines
-   Networking (XDP)
-   LSM hooks
-   System instrumentation

Solnix eliminates common verifier errors by enforcing safety rules at
compile time.

------------------------------------------------------------------------

## Key Features

-   Verifier-aware static validation
-   High-level DSL syntax
-   Structured map definitions
-   Built-in eBPF event modeling
-   Safe register handling
-   Deterministic C backend generation
-   Minimal runtime overhead
-   Rust-implemented compiler core

------------------------------------------------------------------------

## Architecture

Solnix follows a structured compilation pipeline:

    Solnix Source (.snx)
            ↓
    Lexer + Parser
            ↓
    AST Builder
            ↓
    Semantic & Verifier Validator
            ↓
    Intermediate Representation (IR)
            ↓
    C Backend Code Generator
            ↓
    eBPF Object (via clang)

The compiler does not use LLVM directly.\
Instead, it generates structured C code tailored for eBPF compilation.

------------------------------------------------------------------------

## Installation

### Prebuilt Binary

Download the latest release:

https://github.com/solnix-lang/solnix-compiler/releases

Then:

``` bash
chmod +x solnixc
sudo mv solnixc /usr/local/bin/
```

Verify:

``` bash
solnixc --version
```

------------------------------------------------------------------------

### Build From Source

Requirements:

-   Rust (stable)
-   Cargo
-   clang (for eBPF backend compilation)

``` bash
git clone https://github.com/solnix-lang/solnix-compiler.git
cd solnix-compiler
cargo build --release
```

Binary will be located at:

    target/release/solnixc

------------------------------------------------------------------------

## Quick Example

### Solnix Source (`execve_monitor.snx`)

``` snx
map events {
    type: .ringbuf,
    max: 1 << 24
}

event exec_event {
    pid: u32,
    filename: bytes[256]
}

unit trace_exec_filename {
    section "tracepoint/syscalls/sys_enter_execve"
    license "GPL"

    reg pid_tgid = ctx.get_pid_tgid()
    reg pid = pid_tgid
}
```

Compile:

``` bash
solnix build execve_monitor.snx
```

------------------------------------------------------------------------

## Project Structure

    solnix-compiler/
    │
    ├── src/
    │   ├── lexer/
    │   ├── parser/
    │   ├── ast/
    │   ├── semantic/
    │   ├── ir/
    │   ├── codegen/
    │   └── cli/
    │
    ├── tests/
    ├── examples/
    ├── Cargo.toml
    └── README.md

------------------------------------------------------------------------

## Roadmap

-   [ ] IR optimization passes
-   [ ] LSP (Language Server Protocol)
-   [ ] Advanced static analysis
-   [ ] Cross-architecture support
-   [ ] Package registry for Solnix libraries
-   [ ] Formal verification layer

------------------------------------------------------------------------

## Contributing

We welcome contributions.

1.  Fork the repository
2.  Create a feature branch
3.  Submit a Pull Request

Please read CONTRIBUTING.md before submitting changes.

------------------------------------------------------------------------

## License

Licensed under the Apache License 2.0.

See LICENSE file for details.

------------------------------------------------------------------------

## Status

Solnix is under active development and considered experimental.