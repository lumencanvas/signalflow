/**
 * Platform-aware Rust binary builder for CLASP Bridge
 *
 * This script builds the required Rust binaries (clasp-service, clasp-router)
 * before Electron packaging. It handles cross-compilation for different
 * architectures when needed.
 */

const { execSync } = require('child_process');
const path = require('path');
const fs = require('fs');
const os = require('os');

// Root of the CLASP project
const projectRoot = path.join(__dirname, '..', '..', '..');
const targetDir = path.join(projectRoot, 'target', 'release');

// Binaries to build
const binaries = ['clasp-service', 'clasp-router'];

// Get current platform and architecture
const platform = process.platform;
const arch = process.arch;

// Map Node arch to Rust target
function getRustTarget() {
  if (platform === 'darwin') {
    return arch === 'arm64' ? 'aarch64-apple-darwin' : 'x86_64-apple-darwin';
  } else if (platform === 'win32') {
    return arch === 'arm64' ? 'aarch64-pc-windows-msvc' : 'x86_64-pc-windows-msvc';
  } else if (platform === 'linux') {
    return arch === 'arm64' ? 'aarch64-unknown-linux-gnu' : 'x86_64-unknown-linux-gnu';
  }
  throw new Error(`Unsupported platform: ${platform}`);
}

// Get binary extension for current platform
function getBinaryExtension() {
  return platform === 'win32' ? '.exe' : '';
}

// Check if binary exists and is recent
function binaryExists(name) {
  const ext = getBinaryExtension();
  const binaryPath = path.join(targetDir, `${name}${ext}`);
  return fs.existsSync(binaryPath);
}

// Build a single binary
function buildBinary(name) {
  console.log(`Building ${name}...`);

  const startTime = Date.now();

  try {
    execSync(`cargo build --release -p ${name}`, {
      cwd: projectRoot,
      stdio: 'inherit',
      env: {
        ...process.env,
        CARGO_TERM_COLOR: 'always',
      },
    });

    const elapsed = ((Date.now() - startTime) / 1000).toFixed(1);
    console.log(`Built ${name} in ${elapsed}s`);
  } catch (error) {
    console.error(`Failed to build ${name}:`, error.message);
    process.exit(1);
  }
}

// Main build function
function main() {
  console.log('CLASP Bridge - Rust Binary Builder');
  console.log(`Platform: ${platform} (${arch})`);
  console.log(`Target directory: ${targetDir}`);
  console.log('');

  // Check if we should skip building (e.g., CI already built)
  if (process.env.SKIP_RUST_BUILD === '1') {
    console.log('SKIP_RUST_BUILD=1, skipping Rust build');
    return;
  }

  // Check if binaries already exist (for faster dev iteration)
  const ext = getBinaryExtension();
  const allExist = binaries.every(name => {
    const exists = binaryExists(name);
    if (exists) {
      console.log(`Found existing binary: ${name}${ext}`);
    }
    return exists;
  });

  if (allExist && process.env.FORCE_RUST_BUILD !== '1') {
    console.log('\nAll binaries exist. Use FORCE_RUST_BUILD=1 to rebuild.');
    console.log('Skipping Rust build.');
    return;
  }

  console.log('\nBuilding Rust binaries...\n');

  // Build each binary
  for (const name of binaries) {
    buildBinary(name);
  }

  console.log('\nRust binaries built successfully!');

  // Verify binaries exist
  console.log('\nVerifying binaries:');
  for (const name of binaries) {
    const binaryPath = path.join(targetDir, `${name}${ext}`);
    if (fs.existsSync(binaryPath)) {
      const stats = fs.statSync(binaryPath);
      const sizeMB = (stats.size / 1024 / 1024).toFixed(2);
      console.log(`  ${name}${ext} (${sizeMB} MB)`);
    } else {
      console.error(`  ERROR: ${name}${ext} not found!`);
      process.exit(1);
    }
  }
}

main();
