# Cross-Language Chat Tutorial

Build a chat application with clients in JavaScript, Python, and Rust communicating through CLASP.

**Time:** 15-20 minutes
**Prerequisites:** [First Connection](first-connection.md) tutorial

## What You'll Build

A multi-user chat where clients in different languages can communicate:

```
┌─────────────────┐     ┌─────────────┐     ┌─────────────────┐
│  JavaScript     │     │             │     │     Python      │
│  (browser)      │◄───►│   Router    │◄───►│   (terminal)    │
└─────────────────┘     │ (port 7330) │     └─────────────────┘
                        │             │
                        └──────┬──────┘
                               │
                        ┌──────▼──────┐
                        │    Rust     │
                        │  (terminal) │
                        └─────────────┘
```

## Step 1: Start the Router

```bash
clasp server --port 7330
```

## Step 2: JavaScript Client (Browser)

Create `chat.html`:

```html
<!DOCTYPE html>
<html>
<head>
  <title>CLASP Chat</title>
  <style>
    body {
      font-family: system-ui, sans-serif;
      max-width: 600px;
      margin: 40px auto;
      padding: 20px;
      background: #1a1a2e;
      color: white;
    }
    #messages {
      height: 400px;
      overflow-y: auto;
      background: #16213e;
      padding: 15px;
      border-radius: 8px;
      margin-bottom: 10px;
    }
    .message {
      margin: 8px 0;
      padding: 8px 12px;
      background: #1f4068;
      border-radius: 8px;
    }
    .message .sender {
      font-weight: bold;
      color: #00d9ff;
    }
    .message .time {
      font-size: 11px;
      color: #666;
      float: right;
    }
    .message.system {
      background: #2d2d44;
      font-style: italic;
      color: #888;
    }
    .input-row {
      display: flex;
      gap: 10px;
    }
    input[type="text"] {
      flex: 1;
      padding: 12px;
      border: none;
      border-radius: 8px;
      background: #16213e;
      color: white;
    }
    button {
      padding: 12px 24px;
      border: none;
      border-radius: 8px;
      background: #00d9ff;
      color: black;
      cursor: pointer;
    }
    button:hover { background: #00b8d4; }
    #status {
      text-align: center;
      padding: 10px;
      margin-bottom: 10px;
      border-radius: 4px;
    }
    #status.connected { background: #1b4332; }
    #status.disconnected { background: #7f1d1d; }
  </style>
</head>
<body>
  <h1>CLASP Chat</h1>
  <div id="status" class="disconnected">Connecting...</div>
  <div id="messages"></div>
  <div class="input-row">
    <input type="text" id="input" placeholder="Type a message..." />
    <button onclick="sendMessage()">Send</button>
  </div>

  <script type="module">
    import { ClaspBuilder } from 'https://unpkg.com/@clasp-to/core/dist/index.mjs';

    const messages = document.getElementById('messages');
    const input = document.getElementById('input');
    const status = document.getElementById('status');

    // Generate a random username
    const username = 'Browser-' + Math.random().toString(36).substr(2, 4);

    // Connect to CLASP
    const client = await new ClaspBuilder('ws://localhost:7330')
      .withName(username)
      .connect();

    status.textContent = `Connected as ${username}`;
    status.className = 'connected';

    // Subscribe to chat messages
    client.on('/chat/message', (data) => {
      addMessage(data.sender, data.text, data.timestamp);
    });

    // Subscribe to system events
    client.on('/chat/system', (data) => {
      addSystemMessage(data.text);
    });

    // Announce join
    await client.emit('/chat/system', {
      text: `${username} joined from browser`,
      timestamp: Date.now()
    });

    // Make sendMessage available globally
    window.sendMessage = async function() {
      const text = input.value.trim();
      if (!text) return;

      await client.emit('/chat/message', {
        sender: username,
        text: text,
        timestamp: Date.now()
      });

      input.value = '';
    };

    // Send on Enter key
    input.addEventListener('keypress', (e) => {
      if (e.key === 'Enter') sendMessage();
    });

    function addMessage(sender, text, timestamp) {
      const time = new Date(timestamp).toLocaleTimeString();
      messages.innerHTML += `
        <div class="message">
          <span class="time">${time}</span>
          <span class="sender">${sender}:</span> ${text}
        </div>
      `;
      messages.scrollTop = messages.scrollHeight;
    }

    function addSystemMessage(text) {
      messages.innerHTML += `
        <div class="message system">${text}</div>
      `;
      messages.scrollTop = messages.scrollHeight;
    }
  </script>
</body>
</html>
```

Serve it:
```bash
python -m http.server 8080
```

Open http://localhost:8080

## Step 3: Python Client (Terminal)

Create `chat.py`:

```python
import asyncio
import sys
from datetime import datetime
from clasp import ClaspBuilder

async def main():
    # Get username from command line or generate one
    username = sys.argv[1] if len(sys.argv) > 1 else f'Python-{id(object()) % 10000:04d}'

    client = await (
        ClaspBuilder('ws://localhost:7330')
        .with_name(username)
        .connect()
    )

    print(f'\n=== CLASP Chat ===')
    print(f'Connected as: {username}')
    print('Type messages and press Enter to send.')
    print('Press Ctrl+C to quit.\n')

    # Subscribe to messages
    @client.on('/chat/message')
    def on_message(data, _):
        timestamp = datetime.fromtimestamp(data['timestamp'] / 1000)
        time_str = timestamp.strftime('%H:%M:%S')
        print(f'\r[{time_str}] {data["sender"]}: {data["text"]}')
        print('> ', end='', flush=True)

    # Subscribe to system events
    @client.on('/chat/system')
    def on_system(data, _):
        print(f'\r*** {data["text"]} ***')
        print('> ', end='', flush=True)

    # Announce join
    await client.emit('/chat/system', {
        'text': f'{username} joined from Python',
        'timestamp': int(datetime.now().timestamp() * 1000)
    })

    # Input loop
    async def input_loop():
        import aioconsole
        while True:
            try:
                text = await aioconsole.ainput('> ')
                if text.strip():
                    await client.emit('/chat/message', {
                        'sender': username,
                        'text': text,
                        'timestamp': int(datetime.now().timestamp() * 1000)
                    })
            except EOFError:
                break

    try:
        await input_loop()
    except KeyboardInterrupt:
        pass

    await client.emit('/chat/system', {
        'text': f'{username} left',
        'timestamp': int(datetime.now().timestamp() * 1000)
    })
    await client.close()

if __name__ == '__main__':
    asyncio.run(main())
```

Install dependencies and run:
```bash
pip install aioconsole
python chat.py Alice
```

## Step 4: Rust Client (Terminal)

Create a new Rust project:
```bash
cargo new chat-client
cd chat-client
```

Edit `Cargo.toml`:
```toml
[dependencies]
clasp-client = "3.1"
clasp-core = "3.1"
tokio = { version = "1", features = ["full"] }
chrono = "0.4"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

Edit `src/main.rs`:
```rust
use clasp_client::{ClaspBuilder, Clasp};
use clasp_core::Value;
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    sender: String,
    text: String,
    timestamp: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct SystemMessage {
    text: String,
    timestamp: i64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let username = std::env::args()
        .nth(1)
        .unwrap_or_else(|| format!("Rust-{:04}", std::process::id() % 10000));

    let client = ClaspBuilder::new("ws://localhost:7330")
        .name(&username)
        .connect()
        .await?;

    println!("\n=== CLASP Chat ===");
    println!("Connected as: {}", username);
    println!("Type messages and press Enter to send.");
    println!("Press Ctrl+C to quit.\n");

    // Clone for the subscription callback
    let client_clone = client.clone();

    // Subscribe to messages
    client.subscribe("/chat/message", move |value, _addr| {
        if let Value::Map(map) = value {
            let sender = map.get("sender").and_then(|v| match v {
                Value::String(s) => Some(s.as_str()),
                _ => None,
            }).unwrap_or("?");

            let text = map.get("text").and_then(|v| match v {
                Value::String(s) => Some(s.as_str()),
                _ => None,
            }).unwrap_or("");

            let time = Local::now().format("%H:%M:%S");
            println!("\r[{}] {}: {}", time, sender, text);
            print!("> ");
            io::stdout().flush().ok();
        }
    }).await?;

    // Subscribe to system events
    client.subscribe("/chat/system", |value, _addr| {
        if let Value::Map(map) = value {
            let text = map.get("text").and_then(|v| match v {
                Value::String(s) => Some(s.as_str()),
                _ => None,
            }).unwrap_or("");

            println!("\r*** {} ***", text);
            print!("> ");
            io::stdout().flush().ok();
        }
    }).await?;

    // Announce join
    let join_msg = SystemMessage {
        text: format!("{} joined from Rust", username),
        timestamp: chrono::Utc::now().timestamp_millis(),
    };
    client.emit("/chat/system", serde_json::to_value(&join_msg)?).await?;

    // Input loop
    let mut input = String::new();
    loop {
        print!("> ");
        io::stdout().flush()?;

        input.clear();
        if io::stdin().read_line(&mut input)? == 0 {
            break;
        }

        let text = input.trim();
        if !text.is_empty() {
            let msg = ChatMessage {
                sender: username.clone(),
                text: text.to_string(),
                timestamp: chrono::Utc::now().timestamp_millis(),
            };
            client.emit("/chat/message", serde_json::to_value(&msg)?).await?;
        }
    }

    // Announce leave
    let leave_msg = SystemMessage {
        text: format!("{} left", username),
        timestamp: chrono::Utc::now().timestamp_millis(),
    };
    client.emit("/chat/system", serde_json::to_value(&leave_msg)?).await?;
    client.close().await?;

    Ok(())
}
```

Run it:
```bash
cargo run -- Bob
```

## Testing the Chat

1. Open the browser chat at http://localhost:8080
2. Run the Python client: `python chat.py Alice`
3. Run the Rust client: `cargo run -- Bob`

Send messages from any client and see them appear in all three.

## Key Concepts

### Events vs Parameters

This tutorial uses **events** (`emit`) because chat messages:
- Are ephemeral (don't need to be stored)
- Should be delivered to all subscribers
- Don't have "current value" semantics

For state that needs to persist (like user online status), use **parameters**:

```javascript
// Set online status (persisted)
await client.set(`/chat/users/${username}/online`, true);
```

### Address Patterns

The chat uses two address patterns:
- `/chat/message` - Chat messages
- `/chat/system` - System notifications (join/leave)

You could expand this for features like:
- `/chat/room/{roomId}/message` - Multiple rooms
- `/chat/users/{userId}/typing` - Typing indicators
- `/chat/users/{userId}/status` - Online status

## Adding Features

### Typing Indicator

```javascript
// When user is typing
input.addEventListener('input', () => {
  client.set(`/chat/users/${username}/typing`, true);
  clearTimeout(typingTimeout);
  typingTimeout = setTimeout(() => {
    client.set(`/chat/users/${username}/typing`, false);
  }, 2000);
});

// Subscribe to typing status
client.on('/chat/users/*/typing', (isTyping, address) => {
  const user = address.split('/')[3];
  if (user !== username && isTyping) {
    showTypingIndicator(user);
  }
});
```

### Multiple Rooms

```javascript
const room = 'general';
client.on(`/chat/room/${room}/message`, callback);
client.emit(`/chat/room/${room}/message`, message);
```

## Next Steps

- [Cross-Language Communication](../explanation/why-clasp.md) - Why this works
- [Signal Types](../reference/protocol/signal-types.md) - Events vs Params
- [Addressing](../reference/protocol/addressing.md) - Wildcards and patterns
