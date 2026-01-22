/**
 * Example: Embedding CLASP Server in Your Node.js Application
 * 
 * This demonstrates how to run a CLASP router alongside your own code.
 * 
 * NOTE: This example uses @clasp-to/core as a client to connect to an existing
 * router. A full Node.js server package (@clasp-to/server) is planned but not
 * yet available. For now, run the Rust router (clasp-router) separately.
 * 
 * To run a CLASP server from Node.js in the future:
 * 
 *   const { createServer } = require('@clasp-to/server');
 *   const server = createServer({ port: 7330 });
 *   server.start();
 * 
 * For now, this example shows how to build an application that:
 * 1. Connects to a CLASP router
 * 2. Publishes data from your business logic
 * 3. Subscribes to external events
 * 
 * Usage:
 *   # Start a router first
 *   cargo run -p clasp-router-server -- --listen 0.0.0.0:7330
 *   
 *   # Then run this script
 *   node examples/js/embedded-server.js
 */

const { ClaspBuilder } = require('@clasp-to/core');

async function main() {
  console.log('╔══════════════════════════════════════════════════════════╗');
  console.log('║      Node.js Application with CLASP Integration          ║');
  console.log('╚══════════════════════════════════════════════════════════╝');

  // Connect to CLASP router
  const clasp = await new ClaspBuilder('ws://localhost:7330')
    .name('Node.js Sensor Hub')
    .connect();

  console.log('Connected to CLASP router');

  // Subscribe to commands from other clients
  clasp.on('/commands/**', (value, address) => {
    console.log(`Received command: ${address} = ${JSON.stringify(value)}`);
    
    // Handle commands
    if (address === '/commands/reset') {
      console.log('Resetting sensors...');
      // Your reset logic here
    }
  });

  // Publish sensor data periodically
  setInterval(() => {
    const cpuUsage = Math.random() * 0.6 + 0.2; // 20-80%
    const memoryUsage = Math.random() * 0.3 + 0.4; // 40-70%
    const temperature = Math.random() * 15 + 20; // 20-35°C
    const uptime = process.uptime();

    clasp.set('/system/cpu', cpuUsage);
    clasp.set('/system/memory', memoryUsage);
    clasp.set('/sensors/temperature', temperature);
    clasp.set('/system/uptime', Math.floor(uptime));

    console.log(
      `Published: cpu=${(cpuUsage * 100).toFixed(1)}%, ` +
      `mem=${(memoryUsage * 100).toFixed(1)}%, ` +
      `temp=${temperature.toFixed(1)}°C`
    );
  }, 1000);

  // Handle shutdown
  process.on('SIGINT', async () => {
    console.log('\nShutting down...');
    await clasp.close();
    process.exit(0);
  });

  console.log('\nPublishing sensor data every second...');
  console.log('Press Ctrl+C to stop\n');
}

main().catch(console.error);
