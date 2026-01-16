# Contributing to CLASP

Thank you for your interest in contributing to CLASP! This document provides guidelines and information for contributors.

## Code of Conduct

Be respectful. We're all here to build cool stuff.

## How to Contribute

### Reporting Issues

- Check existing issues first to avoid duplicates
- Use the issue templates when available
- Include reproduction steps for bugs
- For feature requests, explain the use case

### Pull Requests

1. **Fork** the repository
2. **Create a branch** for your changes: `git checkout -b feature/my-feature`
3. **Make your changes** with clear, focused commits
4. **Test** your changes: `cargo test --workspace`
5. **Format** your code: `cargo fmt`
6. **Lint** your code: `cargo clippy`
7. **Submit** a pull request

### Commit Messages

Use clear, descriptive commit messages:

```
Add MQTT bridge support for QoS 2 messages

- Implement exactly-once delivery semantics
- Add configuration option for QoS level
- Update documentation with examples
```

## Development Setup

### Prerequisites

- Rust 1.75+ (install via [rustup](https://rustup.rs))
- Node.js 20+ (for desktop app and website)
- Platform dependencies:
  - **Linux**: `sudo apt install libasound2-dev libudev-dev pkg-config`
  - **macOS**: Xcode Command Line Tools

### Building

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/clasp.git
cd clasp

# Build everything
cargo build

# Run tests
cargo test --workspace

# Build desktop app
cd apps/bridge
npm install
npm run start
```

### Project Structure

```
clasp/
├── crates/           # Rust crates
│   ├── clasp-core/   # Core types and codec
│   ├── clasp-bridge/ # Protocol bridges
│   ├── clasp-cli/    # CLI tool
│   └── ...
├── apps/bridge/      # Electron desktop app
├── site/             # Vue documentation site
├── docs/             # Markdown documentation
└── test-suite/       # Integration tests
```

## Adding a New Protocol Bridge

1. Create a new module in `crates/clasp-bridge/src/`
2. Implement the `Bridge` trait:

```rust
#[async_trait]
impl Bridge for MyBridge {
    fn config(&self) -> &BridgeConfig;
    async fn start(&mut self) -> Result<mpsc::Receiver<BridgeEvent>>;
    async fn stop(&mut self) -> Result<()>;
    async fn send(&self, msg: Message) -> Result<()>;
    fn is_running(&self) -> bool;
    fn namespace(&self) -> &str;
}
```

3. Add feature flag in `Cargo.toml`
4. Export from `lib.rs`
5. Add tests
6. Update documentation

## Testing

### Unit Tests

```bash
cargo test -p clasp-core
cargo test -p clasp-bridge
```

### Integration Tests

```bash
cargo test -p test-suite
```

### Manual Testing

The desktop app is useful for manual testing:

```bash
cd apps/bridge
npm run start
```

## Documentation

- Update relevant docs in `/docs` for user-facing changes
- Add rustdoc comments to public APIs
- Include examples for new features

## Style Guide

### Rust

- Follow standard Rust conventions
- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Prefer explicit error handling over `unwrap()`

### JavaScript/Vue

- Use ES6+ features
- Follow existing code style
- No semicolons (project convention)

## Release Process

Releases are automated via GitHub Actions when a tag is pushed:

```bash
git tag v0.1.0
git push origin v0.1.0
```

This triggers builds for all platforms and creates a draft release.

## Getting Help

- Open an issue for questions
- Check existing documentation in `/docs`
- Look at existing implementations for examples

## License

By contributing, you agree that your contributions will be licensed under the same terms as the project (MIT/Apache-2.0).
