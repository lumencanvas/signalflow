# P2P Connection Fix - Critical Handoff Document

**Date:** January 23, 2026  
**Status:** 🔴 **CRITICAL - P2P CONNECTIONS NOT WORKING**  
**Priority:** HIGHEST - Tests timeout because connections never complete

---

## Executive Summary

**P2P connections are timing out because the WebRTC connection completion is never detected.** The infrastructure is 95% complete, but three critical pieces are missing:

1. **Answerer never receives DataChannels** - No `on_data_channel` handler
2. **Connection state never propagates** - DataChannel `on_open` events don't reach P2P manager
3. **No connection monitoring** - P2P manager never calls `mark_connected()`

**The WASM implementation (`crates/clasp-wasm/src/p2p.rs`) has the correct pattern - use it as reference.**

---

## Current Test Results

```
Test 1: P2P Connection Establishment
  ✅ P2P connection initiated
  ❌ FAIL: P2P connection timeout (10s)
```

**Root Cause:** WebRTC connection establishes, but P2P manager never knows about it, so tests timeout waiting for `P2PEvent::Connected`.

---

## Deep Investigation Findings

### ✅ What EXISTS and Works:

1. **P2P Manager Integration** (`crates/clasp-client/src/client.rs`):
   - ✅ P2P manager created when `p2p_config` provided
   - ✅ Auto-subscribes to `/p2p/signal/{session_id}`
   - ✅ Auto-subscribes to `/p2p/announce`
   - ✅ Signaling messages forwarded correctly
   - ✅ `connect_to_peer()` method works
   - ✅ `on_p2p_event()` callback system works

2. **WebRTC Transport** (`crates/clasp-transport/src/webrtc.rs`):
   - ✅ Offerer creates DataChannels correctly
   - ✅ Answerer creates answer correctly
   - ✅ ICE candidate exchange works
   - ✅ `setup_channel_handlers()` sets up `on_open` callbacks
   - ✅ `on_open` sends `TransportEvent::Connected` to channel

3. **Signaling** (`crates/clasp-client/src/p2p.rs`):
   - ✅ Offer/Answer exchange works
   - ✅ ICE candidates exchanged via signaling
   - ✅ `mark_connected()` method exists and works
   - ✅ `P2PSignal::Connected` sent to peer

### ❌ What's MISSING (Critical Gaps):

#### Gap 1: Answerer Never Receives DataChannels

**Location:** `crates/clasp-transport/src/webrtc.rs:131-172`

**Problem:**
```rust
// In new_answerer_with_config:
// Data channels will be created by the offerer and received via on_data_channel
Ok((
    Self {
        config,
        peer_connection,
        unreliable_channel: None,  // ❌ Never set!
        reliable_channel: None,    // ❌ Never set!
    },
    sdp,
))
```

**The comment says "received via on_data_channel" but NO handler is set up!**

**Reference Implementation (WASM):**
See `crates/clasp-wasm/src/p2p.rs:260-310` - `setup_incoming_channel_handler()`:
- Sets up `ondatachannel` callback
- Handles incoming channels from offerer
- Sets up `on_open` handlers on received channels
- Updates connection state when channels open

**Fix Required:**
- Set up `peer_connection.on_data_channel()` handler in `new_answerer_with_config`
- Store received channels in `unreliable_channel` and `reliable_channel`
- Set up `on_open` handlers on received channels

#### Gap 2: Connection State Never Propagates to P2P Manager

**Location:** `crates/clasp-transport/src/webrtc.rs:312-343`

**Problem:**
```rust
fn setup_channel_handlers(channel: Arc<RTCDataChannel>) -> (mpsc::Sender<TransportEvent>, mpsc::Receiver<TransportEvent>) {
    // ...
    channel.on_open(Box::new(move || {
        let tx = tx_open.clone();
        Box::pin(async move {
            let _ = tx.send(TransportEvent::Connected).await;  // ✅ Event sent
        })
    }));
    // ...
}
```

**The `TransportEvent::Connected` is sent, but:**
- The receiver (`rx`) is only returned to caller
- P2P manager never calls `reliable_channel()` or `unreliable_channel()` to get the receiver
- Even if it did, there's no code to monitor the receiver and call `mark_connected()`

**Reference Implementation (WASM):**
See `crates/clasp-wasm/src/p2p.rs:290-297`:
```rust
let onopen = Closure::wrap(Box::new(move |_: JsValue| {
    *state_clone.borrow_mut() = WasmP2PState::Connected;
    if let Some(callback) = on_state_change_clone.borrow().as_ref() {
        let _ = callback.call1(&JsValue::NULL, &JsValue::from_str("connected"));
    }
}) as Box<dyn FnMut(JsValue)>);
channel.set_onopen(Some(onopen.as_ref().unchecked_ref()));
```

**Fix Required:**
- When DataChannel opens, call `p2p_manager.mark_connected(peer_session_id)`
- Need to pass P2P manager reference (or callback) to transport
- OR: Monitor transport's connection state and call `mark_connected()` when ready

#### Gap 3: No Connection State Monitoring

**Location:** `crates/clasp-client/src/p2p.rs:228-232`

**Problem:**
```rust
let (transport, sdp_offer) = WebRtcTransport::new_offerer_with_config(webrtc_config).await?;
connection.transport = Some(transport);  // ✅ Transport stored
// ❌ But never monitored for connection state!
```

**The transport is stored but:**
- No code monitors when DataChannels open
- No code calls `mark_connected()` when connection completes
- Connection state stays in `GatheringCandidates` forever

**Reference Implementation (WASM):**
See `crates/clasp-wasm/src/p2p.rs:312-335` - `setup_channel_handlers()`:
- Sets up `on_open` callback on each channel
- Updates state to `Connected` when channel opens
- Notifies callback system

**Fix Required:**
- After storing transport, set up connection state monitoring
- When DataChannel opens (offerer side), call `mark_connected()`
- When DataChannel received and opens (answerer side), call `mark_connected()`

---

## Detailed Code Analysis

### Offerer Flow (Current - BROKEN):

1. ✅ `connect_to_peer()` called
2. ✅ `WebRtcTransport::new_offerer_with_config()` creates transport with DataChannels
3. ✅ Offer sent via signaling
4. ✅ Answer received, `set_remote_answer()` called
5. ✅ ICE candidates exchanged
6. ❌ **DataChannels open, but no one is listening**
7. ❌ **`mark_connected()` never called**
8. ❌ **Connection state stays `GatheringCandidates`**

### Answerer Flow (Current - BROKEN):

1. ✅ Offer received, `handle_offer()` called
2. ✅ `WebRtcTransport::new_answerer_with_config()` creates transport
3. ✅ Answer sent via signaling
4. ✅ ICE candidates exchanged
5. ❌ **`on_data_channel` handler never set up**
6. ❌ **DataChannels from offerer never received**
7. ❌ **`mark_connected()` never called**
8. ❌ **Connection state stays `GatheringCandidates`**

### Correct Flow (Reference - WASM):

1. ✅ Offerer creates transport with DataChannels
2. ✅ Sets up `on_open` handlers on created channels
3. ✅ When channel opens, updates state to `Connected`
4. ✅ Answerer sets up `on_data_channel` handler
5. ✅ When channel received, sets up `on_open` handler
6. ✅ When channel opens, updates state to `Connected`

---

## Files That Need Changes

### 1. `crates/clasp-transport/src/webrtc.rs`

**Changes Required:**

#### A. Add `on_data_channel` handler for answerer

In `new_answerer_with_config()`:
```rust
// After creating peer_connection, BEFORE returning:
// Set up handler for incoming data channels (from offerer)
let unreliable_channel_ref = Arc::new(Mutex::new(None));
let reliable_channel_ref = Arc::new(Mutex::new(None));

// Clone for closure
let unreliable_clone = unreliable_channel_ref.clone();
let reliable_clone = reliable_channel_ref.clone();

peer_connection.on_data_channel(Box::new(move |channel: Arc<RTCDataChannel>| {
    let label = channel.label();
    info!("Received data channel: {}", label);
    
    if label == "clasp" {
        *unreliable_clone.lock() = Some(channel);
    } else if label == "clasp-reliable" {
        *reliable_clone.lock() = Some(channel);
    }
}));

// Return transport with channels (will be set when received)
```

**Problem:** We can't return channels that don't exist yet. Need different approach.

**Better Approach:** Return transport, and provide method to get channels when ready:
```rust
pub async fn wait_for_channels(&self) -> Result<()> {
    // Wait for both channels to be received
    // Return when both are ready
}
```

OR: Use callback pattern like WASM does.

#### B. Add connection state callback to transport

Add to `WebRtcTransport`:
```rust
pub fn on_channel_open<F>(&self, callback: F)
where
    F: Fn(String) + Send + Sync + 'static,
{
    // Set up on_open handlers on both channels
    // Call callback with channel label when opened
}
```

### 2. `crates/clasp-client/src/p2p.rs`

**Changes Required:**

#### A. Monitor transport connection state

In `connect_to_peer()` after storing transport:
```rust
// Set up connection state monitoring
if let Some(ref transport) = connection.transport {
    let p2p_manager = Arc::clone(self);
    let peer_id = peer_session_id.to_string();
    
    // Monitor when channels open
    transport.on_channel_open(move |channel_label| {
        info!("Channel opened: {}", channel_label);
        // When reliable channel opens, mark as connected
        if channel_label == "clasp-reliable" {
            let p2p = p2p_manager.clone();
            let peer = peer_id.clone();
            tokio::spawn(async move {
                if let Err(e) = p2p.mark_connected(&peer).await {
                    warn!("Failed to mark connected: {}", e);
                }
            });
        }
    });
}
```

In `handle_offer()` after storing transport:
```rust
// Same monitoring setup for answerer
```

#### B. Set up `on_data_channel` handler for answerer

The transport needs to handle this, but P2P manager needs to know when channels are ready.

**Alternative:** Use the transport's `reliable_channel()` method, but it returns `None` for answerer until channels are received.

**Better:** Add method to transport:
```rust
pub async fn wait_for_reliable_channel(&self) -> Result<Arc<RTCDataChannel>> {
    // Poll until reliable channel is available
    // Return when received and opened
}
```

Then in P2P manager:
```rust
// After creating answerer transport
tokio::spawn(async move {
    if let Ok(channel) = transport.wait_for_reliable_channel().await {
        // Channel is ready, mark as connected
        p2p_manager.mark_connected(from).await?;
    }
});
```

---

## Reference Implementation Analysis

### WASM Implementation (`crates/clasp-wasm/src/p2p.rs`)

**Key Patterns to Follow:**

1. **Answerer receives channels** (lines 260-310):
   - Sets up `ondatachannel` callback
   - Stores received channels
   - Sets up `on_open` handlers immediately

2. **Connection state tracking** (lines 290-297, 328-335):
   - `on_open` callback updates state to `Connected`
   - Notifies callback system
   - State is tracked per-connection

3. **Offerer channel handling** (lines 312-335):
   - Sets up `on_open` on created channels
   - Updates state when channels open

**Why WASM Works:**
- Browser WebRTC API provides `ondatachannel` event
- `onopen` callbacks are set up immediately
- State is updated synchronously in callbacks

**Why Native Doesn't Work:**
- `webrtc-rs` library requires explicit `on_data_channel` setup
- We never set it up for answerer
- We never monitor channel `on_open` events
- We never call `mark_connected()` when ready

---

## Implementation Strategy

### Option 1: Callback Pattern (Recommended - Matches WASM)

1. **Add to `WebRtcTransport`:**
   ```rust
   pub fn set_connection_callback<F>(&self, callback: F)
   where
       F: Fn(bool) + Send + Sync + 'static,  // bool = is_connected
   {
       // Set up on_open handlers on offerer channels
       // Set up on_data_channel + on_open for answerer
       // Call callback(true) when reliable channel opens
       // Call callback(false) when channel closes
   }
   ```

2. **In P2P Manager:**
   ```rust
   transport.set_connection_callback(move |connected| {
       if connected {
           p2p_manager.mark_connected(peer_id).await?;
       }
   });
   ```

### Option 2: Polling Pattern

1. **Add to `WebRtcTransport`:**
   ```rust
   pub fn is_connected(&self) -> bool {
       // Check if reliable channel exists and is open
       self.reliable_channel.as_ref()
           .map(|ch| ch.ready_state() == RTCDataChannelState::Open)
           .unwrap_or(false)
   }
   ```

2. **In P2P Manager:**
   ```rust
   // Spawn task to poll connection state
   tokio::spawn(async move {
       loop {
           if transport.is_connected() {
               p2p_manager.mark_connected(peer_id).await?;
               break;
           }
           sleep(Duration::from_millis(100)).await;
       }
   });
   ```

### Option 3: Event Channel Pattern (Current Partial Implementation)

1. **Use existing `reliable_channel()` method:**
   ```rust
   if let Some((_sender, mut receiver)) = transport.reliable_channel() {
       tokio::spawn(async move {
           while let Some(event) = receiver.recv().await {
               if matches!(event, TransportEvent::Connected) {
                   p2p_manager.mark_connected(peer_id).await?;
                   break;
               }
           }
       });
   }
   ```

**Problem:** For answerer, `reliable_channel()` returns `None` until channel is received. Need to wait for it.

---

## Recommended Fix (Detailed)

### Step 1: Fix Answerer DataChannel Reception

**File:** `crates/clasp-transport/src/webrtc.rs`

**In `new_answerer_with_config()`:**

```rust
pub async fn new_answerer_with_config(
    remote_offer: &str,
    config: WebRtcConfig,
) -> Result<(Self, String)> {
    let peer_connection = Self::create_peer_connection(&config).await?;

    // ... existing offer/answer code ...

    // Set up handler for incoming data channels (from offerer)
    let unreliable_channel_ref = Arc::new(Mutex::new(None));
    let reliable_channel_ref = Arc::new(Mutex::new(None));
    
    let unreliable_clone = unreliable_channel_ref.clone();
    let reliable_clone = reliable_channel_ref.clone();
    
    peer_connection.on_data_channel(Box::new(move |channel: Arc<RTCDataChannel>| {
        let label = channel.label();
        info!("Received data channel from offerer: {}", label);
        
        if label == "clasp" {
            *unreliable_clone.lock() = Some(channel);
        } else if label == "clasp-reliable" {
            *reliable_clone.lock() = Some(channel);
        }
    }));

    Ok((
        Self {
            config,
            peer_connection,
            unreliable_channel: None,  // Will be set when received
            reliable_channel: None,    // Will be set when received
        },
        sdp,
    ))
}
```

**Problem:** Channels are received asynchronously, can't return them immediately.

**Solution:** Add method to wait for channels:
```rust
impl WebRtcTransport {
    /// Wait for data channels to be received (answerer only)
    pub async fn wait_for_channels(&mut self, timeout: Duration) -> Result<()> {
        let deadline = Instant::now() + timeout;
        
        while Instant::now() < deadline {
            // Check if channels are set (they're set in on_data_channel callback)
            // This requires making channels accessible or using a channel/oneshot
            // Better: Use Arc<Mutex<Option<...>>> pattern and poll
            sleep(Duration::from_millis(50)).await;
        }
        
        if self.reliable_channel.is_some() && self.unreliable_channel.is_some() {
            Ok(())
        } else {
            Err(TransportError::ConnectionFailed("Channels not received".into()))
        }
    }
}
```

**Better Solution:** Use callback pattern from the start.

### Step 2: Add Connection State Callback

**File:** `crates/clasp-transport/src/webrtc.rs`

Add to `WebRtcTransport`:
```rust
pub fn on_connection_ready<F>(&self, callback: F)
where
    F: Fn() + Send + Sync + 'static,
{
    // For offerer: Set up on_open on existing channels
    if let Some(ref reliable) = self.reliable_channel {
        let cb = callback.clone();
        reliable.on_open(Box::new(move || {
            cb();
        }));
    }
    
    // For answerer: Set up on_data_channel + on_open
    // This is trickier - need to store callback and call when channel received
    // OR: Use the existing setup_channel_handlers pattern
}
```

### Step 3: Wire Up in P2P Manager

**File:** `crates/clasp-client/src/p2p.rs`

**In `connect_to_peer()` after storing transport:**
```rust
// Set up connection monitoring
if let Some(ref transport) = connection.transport {
    let p2p_manager = Arc::new(self);  // Need Arc<Self>
    let peer_id = peer_session_id.to_string();
    
    transport.on_connection_ready(move || {
        let p2p = p2p_manager.clone();
        let peer = peer_id.clone();
        tokio::spawn(async move {
            if let Err(e) = p2p.mark_connected(&peer).await {
                warn!("Failed to mark connected: {}", e);
            }
        });
    });
}
```

**In `handle_offer()` after storing transport:**
```rust
// Same setup for answerer
```

---

## Critical Implementation Notes

### 1. Answerer Channel Reception

**The answerer MUST set up `on_data_channel` handler BEFORE the offerer's channels are created.** This is a timing issue - if handler is set up too late, channels might be missed.

**Solution:** Set up handler immediately when creating answerer transport, before returning.

### 2. Connection State Detection

**Both offerer and answerer need to detect when reliable channel opens.**

**Offerer:** Channels are created immediately, set up `on_open` handlers.

**Answerer:** Channels are received asynchronously, set up `on_open` when received.

### 3. Race Conditions

**Potential race:** `mark_connected()` might be called before ICE completes.

**Solution:** Only call `mark_connected()` when DataChannel is actually open (not just created/received).

**Check:** `channel.ready_state() == RTCDataChannelState::Open`

### 4. Both Channels or Just Reliable?

**Question:** Should we wait for both channels or just reliable?

**Answer:** Just reliable channel is sufficient for "connected" state. Unreliable can come later.

**Reference:** WASM implementation marks connected when reliable channel opens.

---

## Testing Strategy

### Test 1: Verify Answerer Receives Channels

```rust
// Create answerer transport
let (transport, _) = WebRtcTransport::new_answerer_with_config(offer, config).await?;

// Wait a bit for channels
sleep(Duration::from_secs(2)).await;

// Check if channels were received
assert!(transport.reliable_channel().is_some());
assert!(transport.unreliable_channel().is_some());
```

### Test 2: Verify Connection State Propagation

```rust
let connected = Arc::new(AtomicBool::new(false));
let connected_clone = connected.clone();

p2p_manager.on_event(move |event| {
    if matches!(event, P2PEvent::Connected { .. }) {
        connected_clone.store(true, Ordering::SeqCst);
    }
});

client_a.connect_to_peer(&session_b).await?;

// Wait for connection
let deadline = Instant::now() + Duration::from_secs(10);
while Instant::now() < deadline {
    if connected.load(Ordering::SeqCst) {
        break;
    }
    sleep(Duration::from_millis(100)).await;
}

assert!(connected.load(Ordering::SeqCst), "Connection should be established");
```

### Test 3: Verify Both Sides Detect Connection

```rust
// Both clients should receive Connected event
let a_connected = Arc::new(AtomicBool::new(false));
let b_connected = Arc::new(AtomicBool::new(false));

client_a.on_p2p_event(|e| { if matches!(e, P2PEvent::Connected { .. }) { a_connected.store(true); } });
client_b.on_p2p_event(|e| { if matches!(e, P2PEvent::Connected { .. }) { b_connected.store(true); } });

client_a.connect_to_peer(&session_b).await?;
wait_for_both_connected().await;

assert!(a_connected.load(Ordering::SeqCst));
assert!(b_connected.load(Ordering::SeqCst));
```

---

## Files to Modify

1. **`crates/clasp-transport/src/webrtc.rs`**
   - Add `on_data_channel` handler in `new_answerer_with_config()`
   - Add connection state callback mechanism
   - Ensure channels are properly stored when received

2. **`crates/clasp-client/src/p2p.rs`**
   - Wire up connection state monitoring in `connect_to_peer()`
   - Wire up connection state monitoring in `handle_offer()`
   - Call `mark_connected()` when channels open

3. **`test-suite/src/bin/p2p_connection_tests.rs`**
   - Update tests to verify connection completion
   - Add test for answerer channel reception
   - Add test for connection state propagation

---

## Success Criteria

✅ **Test 1 passes:** P2P connection establishment completes within 10 seconds  
✅ **Both clients receive `P2PEvent::Connected`**  
✅ **Connection state transitions:** Disconnected → Connecting → GatheringCandidates → Connected  
✅ **Answerer receives DataChannels from offerer**  
✅ **Both offerer and answerer detect channel open**  
✅ **`mark_connected()` is called on both sides**

---

## Reference Code Locations

**Working Implementation (WASM):**
- `crates/clasp-wasm/src/p2p.rs:260-310` - Answerer channel reception
- `crates/clasp-wasm/src/p2p.rs:290-297` - Connection state on open
- `crates/clasp-wasm/src/p2p.rs:312-335` - Offerer channel handlers

**Broken Implementation (Native):**
- `crates/clasp-transport/src/webrtc.rs:131-172` - Answerer (missing `on_data_channel`)
- `crates/clasp-client/src/p2p.rs:228-232` - Transport stored but not monitored
- `crates/clasp-client/src/p2p.rs:415-422` - Answerer transport stored but not monitored

---

## Critical Prompt for Next LLM

**DO NOT take shortcuts. This is a production-critical fix.**

1. **Read the WASM implementation first** (`crates/clasp-wasm/src/p2p.rs`) - it works correctly
2. **Understand the WebRTC flow:**
   - Offerer creates channels → Answerer receives them via `on_data_channel`
   - Both sides need `on_open` handlers to detect when channels are ready
   - Connection is "connected" when reliable channel opens
3. **Fix answerer first** - add `on_data_channel` handler in `new_answerer_with_config()`
4. **Add connection state monitoring** - set up callbacks to call `mark_connected()` when channels open
5. **Test thoroughly** - verify both offerer and answerer detect connection
6. **Check for race conditions** - ensure `mark_connected()` is only called when channel is actually open
7. **Verify tests pass** - `test_p2p_connection_establishment()` should complete in < 10 seconds

**Key Principle:** The transport layer should notify the P2P manager when connection is ready. The P2P manager should NOT poll - it should use callbacks/events.

**Architecture:** Transport → Callback → P2P Manager → `mark_connected()` → Event → Test detects completion

**DO NOT:**
- ❌ Poll connection state (inefficient, race conditions)
- ❌ Skip answerer channel reception (connections will never complete)
- ❌ Assume channels are ready immediately (they're async)
- ❌ Call `mark_connected()` before channel is open (wrong state)

**DO:**
- ✅ Set up `on_data_channel` handler for answerer
- ✅ Set up `on_open` handlers for both offerer and answerer
- ✅ Call `mark_connected()` when reliable channel opens
- ✅ Test with actual WebRTC connections (not mocks)
- ✅ Verify both sides detect connection

---

## Additional Context

### WebRTC Connection Flow

1. **Offerer:**
   - Creates `RTCPeerConnection`
   - Creates DataChannels (`clasp`, `clasp-reliable`)
   - Creates offer SDP
   - Sends offer via signaling
   - Receives answer, sets remote description
   - Exchanges ICE candidates
   - **DataChannels open when ICE completes** → `on_open` fires

2. **Answerer:**
   - Receives offer via signaling
   - Creates `RTCPeerConnection`
   - Sets remote description (offer)
   - Creates answer SDP
   - Sends answer via signaling
   - Exchanges ICE candidates
   - **Receives DataChannels from offerer** → `on_data_channel` fires
   - **DataChannels open when ICE completes** → `on_open` fires

### Current Broken Flow

1. **Offerer:**
   - ✅ Creates channels
   - ✅ Sends offer
   - ✅ Receives answer
   - ✅ ICE exchange
   - ❌ Channels open, but no handler calls `mark_connected()`

2. **Answerer:**
   - ✅ Receives offer
   - ✅ Sends answer
   - ✅ ICE exchange
   - ❌ Never receives DataChannels (no `on_data_channel` handler)
   - ❌ Never detects connection

### Fixed Flow (Target)

1. **Offerer:**
   - ✅ Creates channels
   - ✅ Sets up `on_open` handlers
   - ✅ When channel opens → calls `mark_connected()`
   - ✅ Connection state → Connected

2. **Answerer:**
   - ✅ Sets up `on_data_channel` handler
   - ✅ Receives channels from offerer
   - ✅ Sets up `on_open` handlers on received channels
   - ✅ When channel opens → calls `mark_connected()`
   - ✅ Connection state → Connected

---

**This handoff contains all information needed to fix P2P connections. Follow the WASM implementation pattern, fix the three gaps identified, and verify with real tests. Do not take shortcuts.**
