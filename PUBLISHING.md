# Publishing CLASP to Package Managers

This guide covers publishing CLASP to Cargo (crates.io) and npm.

## Pre-Publishing Checklist

- [x] Rename GitHub repo from `signalflow` to `clasp`
- [ ] Run `cargo test --workspace` to verify everything builds
- [ ] Run `cargo clippy --workspace` to check for warnings

---

## Cargo (crates.io)

### Setup (One-time)

1. Create account at https://crates.io (login with GitHub)
2. Get API token from https://crates.io/settings/tokens
3. Login locally:
   ```bash
   cargo login <your-api-token>
   ```

### Publish Order

Publish in this order (dependencies first). Wait ~1 minute between each for crates.io to index.

```bash
# 1. Core (no dependencies on other clasp crates)
cargo publish -p clasp-core

# 2. Transport (depends on clasp-core)
cargo publish -p clasp-transport

# 3. Discovery (depends on clasp-core)
cargo publish -p clasp-discovery

# 4. Router (depends on clasp-core)
cargo publish -p clasp-router

# 5. Client (depends on clasp-core, clasp-transport)
cargo publish -p clasp-client

# 6. Bridge (depends on clasp-core)
cargo publish -p clasp-bridge

# 7. CLI (depends on clasp-core, clasp-bridge, clasp-transport)
cargo publish -p clasp-cli
```

### Dry Run (Test without publishing)

```bash
cargo publish -p clasp-core --dry-run
```

---

## npm

### Setup (One-time)

1. Create account at https://npmjs.com
2. Create organization `@clasp-to` at https://www.npmjs.com/org/create
3. Login locally:
   ```bash
   npm login
   ```

### Publish

```bash
cd bindings/js/packages/clasp-core
npm run build
npm publish --access public
```

**Package name:** `@clasp-to/core`

---

## PyPI

### Setup (One-time)

1. Create account at https://pypi.org
2. Create API token at https://pypi.org/manage/account/token/
3. Configure credentials:
   ```bash
   pip install build twine
   ```

### Publish

```bash
cd bindings/python
python -m build
twine upload dist/*
```

**Package name:** `clasp-to`

---

## Package Names Summary

| Manager | Package | Install Command |
|---------|---------|-----------------|
| Cargo | `clasp-cli` | `cargo install clasp-cli` |
| Cargo | `clasp-core` | `clasp-core = "0.1"` in Cargo.toml |
| Cargo | `clasp-bridge` | `clasp-bridge = "0.1"` in Cargo.toml |
| npm | `@clasp-to/core` | `npm install @clasp-to/core` |
| PyPI | `clasp-to` | `pip install clasp-to` |

---

## After Publishing

1. Create a GitHub release with tag `v0.1.0`
2. The release workflow will automatically build and attach:
   - Desktop app binaries (macOS, Windows, Linux)
   - CLI binaries for all platforms
3. Update the website download links if needed

---

## Troubleshooting

### "crate already exists"
Someone else published a crate with that name. Check https://crates.io/crates/<name>

### "dependency not found"
Wait a minute after publishing dependencies before publishing dependent crates.

### "missing field"
Run `cargo package -p <crate-name>` to see what's missing.
