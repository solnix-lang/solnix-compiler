# Contributing to Solnix Compiler

Thank you for your interest in contributing to Solnix.

We welcome contributions that improve the compiler, documentation,
tooling, and ecosystem.

------------------------------------------------------------------------

## Code of Conduct

By participating in this project, you agree to maintain a respectful and
professional environment.

Be constructive, collaborative, and technically focused.

------------------------------------------------------------------------

## Ways to Contribute

You can contribute in several ways:

-   Reporting bugs
-   Suggesting features
-   Improving documentation
-   Submitting pull requests
-   Writing examples
-   Improving tests
-   Enhancing static analysis and verifier logic

------------------------------------------------------------------------

## Reporting Issues

Before opening a new issue:

1.  Search existing issues to avoid duplicates.
2.  Provide a clear and descriptive title.
3.  Include reproduction steps if reporting a bug.
4.  Attach logs or error messages when relevant.

------------------------------------------------------------------------

## Development Setup

### Requirements

-   Rust (stable)
-   Cargo
-   clang (for eBPF backend compilation)
-   Linux environment recommended

### Clone Repository

``` bash
git clone https://github.com/solnix-lang/solnix-compiler.git
cd solnix-compiler
```

### Build

``` bash
cargo build
```

### Run Tests

``` bash
cargo test
```

------------------------------------------------------------------------

## Branching Strategy

-   `main` → Stable branch
-   `dev` → Active development
-   `feature/*` → New features
-   `fix/*` → Bug fixes

Always branch from `dev` unless otherwise specified.

------------------------------------------------------------------------

## Pull Request Guidelines

Before submitting a PR:

-   Ensure the code compiles without warnings.
-   Run formatting checks:

``` bash
cargo fmt
```

-   Run lints:

``` bash
cargo clippy
```

-   Add tests if applicable.
-   Update documentation if needed.

PRs should include:

-   Clear description of changes
-   Motivation for change
-   Related issue (if applicable)

------------------------------------------------------------------------

## Coding Standards

-   Follow Rust idiomatic patterns
-   Keep modules focused and minimal
-   Avoid unnecessary complexity
-   Write descriptive commit messages

Commit message format:

    type(scope): short description

    Optional detailed explanation.

Example:

    feat(parser): add map attribute validation

------------------------------------------------------------------------

## Security Issues

If you discover a security vulnerability, do not open a public issue.

Instead, report it responsibly via:

security@solnix-lang.org

------------------------------------------------------------------------

## Roadmap Contributions

Major architectural changes should be discussed first via an issue
before implementation.

------------------------------------------------------------------------

## License

By contributing, you agree that your contributions will be licensed
under the Apache License 2.0.

------------------------------------------------------------------------

Thank you for helping improve Solnix.