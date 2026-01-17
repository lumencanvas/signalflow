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

// Binaries to build (maps binary name to package name)
const binaries = [
  { name: 'clasp-service', package: 'clasp-service' },
  { name: 'clasp-router', package: 'clasp-router-server' },
];

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
function buildBinary(binaryName, packageName) {
  console.log(`Building ${binaryName} (package: ${packageName})...`);

  const startTime = Date.now();

  try {
    execSync(`cargo build --release -p ${packageName}`, {
      cwd: projectRoot,
      stdio: 'inherit',
      env: {
        ...process.env,
        CARGO_TERM_COLOR: 'always',
      },
    });

    const elapsed = ((Date.now() - startTime) / 1000).toFixed(1);
    console.log(`Built ${binaryName} in ${elapsed}s`);
  } catch (error) {
    console.error(`Failed to build ${binaryName}:`, error.message);
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
  const allExist = binaries.every(({ name }) => {
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
  for (const { name, package: pkg } of binaries) {
    buildBinary(name, pkg);
  }

  console.log('\nRust binaries built successfully!');

  // Verify binaries exist
  console.log('\nVerifying binaries:');
  for (const { name } of binaries) {
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
