#!/usr/bin/env python3
"""
Example: Embedding CLASP in Your Python Application

This demonstrates how to integrate CLASP with your Python application.

NOTE: This example uses clasp-to as a client to connect to an existing router.
A full Python server package is planned but not yet available. For now, run
the Rust router (clasp-router) separately.

To run a CLASP server from Python in the future:

    from clasp import Server
    server = Server(port=7330)
    server.run()

For now, this example shows how to build an application that:
1. Connects to a CLASP router
2. Publishes data from your business logic
3. Subscribes to external events

Usage:
    # Start a router first
    cargo run -p clasp-router-server -- --listen 0.0.0.0:7330
    
    # Then run this script
    python examples/python/embedded_server.py
"""

import asyncio
import random
import time
from clasp import Clasp

async def main():
    print('╔══════════════════════════════════════════════════════════╗')
    print('║      Python Application with CLASP Integration           ║')
    print('╚══════════════════════════════════════════════════════════╝')

    # Connect to CLASP router
    client = Clasp('ws://localhost:7330')
    await client.connect()
    print('Connected to CLASP router')

    # Subscribe to commands from other clients
    def on_command(value, address):
        print(f'Received command: {address} = {value}')
        if address == '/commands/reset':
            print('Resetting sensors...')
            # Your reset logic here

    await client.subscribe('/commands/**', on_command)

    # Track start time for uptime
    start_time = time.time()

    # Publish sensor data periodically
    print('\nPublishing sensor data every second...')
    print('Press Ctrl+C to stop\n')
    
    try:
        while True:
            cpu_usage = random.uniform(0.2, 0.8)  # 20-80%
            memory_usage = random.uniform(0.4, 0.7)  # 40-70%
            temperature = random.uniform(20, 35)  # 20-35°C
            uptime = int(time.time() - start_time)

            await client.set('/system/cpu', cpu_usage)
            await client.set('/system/memory', memory_usage)
            await client.set('/sensors/temperature', temperature)
            await client.set('/system/uptime', uptime)

            print(
                f'Published: cpu={cpu_usage * 100:.1f}%, '
                f'mem={memory_usage * 100:.1f}%, '
                f'temp={temperature:.1f}°C, '
                f'uptime={uptime}s'
            )

            await asyncio.sleep(1)

    except KeyboardInterrupt:
        print('\nShutting down...')
    finally:
        await client.close()

if __name__ == '__main__':
    asyncio.run(main())
