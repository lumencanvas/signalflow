//! WASM WebSocket transport implementation
//!
//! This module provides a WebSocket client for WASM environments using web-sys.
//! Note: WASM cannot act as a WebSocket server, only as a client.

use async_trait::async_trait;
use bytes::Bytes;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

use crate::error::{Result, TransportError};
use crate::traits::{Transport, TransportEvent, TransportReceiver, TransportSender};

use clasp_core::WS_SUBPROTOCOL;

/// WASM WebSocket configuration
#[derive(Debug, Clone)]
pub struct WasmWebSocketConfig {
    /// Subprotocol to use
    pub subprotocol: String,
}

impl Default for WasmWebSocketConfig {
    fn default() -> Self {
        Self {
            subprotocol: WS_SUBPROTOCOL.to_string(),
        }
    }
}

/// WASM WebSocket transport
pub struct WasmWebSocketTransport {
    config: WasmWebSocketConfig,
}

impl WasmWebSocketTransport {
    pub fn new() -> Self {
        Self {
            config: WasmWebSocketConfig::default(),
        }
    }

    pub fn with_config(config: WasmWebSocketConfig) -> Self {
        Self { config }
    }
}

impl Default for WasmWebSocketTransport {
    fn default() -> Self {
        Self::new()
    }
}

/// Internal state for the WebSocket connection
struct WasmWsState {
    connected: bool,
    event_queue: Vec<TransportEvent>,
}

/// WASM WebSocket sender
pub struct WasmWebSocketSender {
    ws: web_sys::WebSocket,
    state: Rc<RefCell<WasmWsState>>,
}

impl WasmWebSocketSender {
    fn new(ws: web_sys::WebSocket, state: Rc<RefCell<WasmWsState>>) -> Self {
        Self { ws, state }
    }
}

#[async_trait(?Send)]
impl TransportSender for WasmWebSocketSender {
    async fn send(&self, data: Bytes) -> Result<()> {
        if !self.is_connected() {
            return Err(TransportError::NotConnected);
        }

        self.ws
            .send_with_u8_array(&data)
            .map_err(|e| TransportError::SendFailed(format!("{:?}", e)))?;

        Ok(())
    }

    fn try_send(&self, data: Bytes) -> Result<()> {
        // WebSocket send in WASM is already non-blocking
        if !self.is_connected() {
            return Err(TransportError::NotConnected);
        }

        self.ws
            .send_with_u8_array(&data)
            .map_err(|e| TransportError::SendFailed(format!("{:?}", e)))?;

        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.state.borrow().connected && self.ws.ready_state() == web_sys::WebSocket::OPEN
    }

    async fn close(&self) -> Result<()> {
        self.ws
            .close()
            .map_err(|e| TransportError::SendFailed(format!("{:?}", e)))?;
        self.state.borrow_mut().connected = false;
        Ok(())
    }
}

/// WASM WebSocket receiver
pub struct WasmWebSocketReceiver {
    state: Rc<RefCell<WasmWsState>>,
    _closures: Vec<Closure<dyn FnMut(web_sys::Event)>>,
}

#[async_trait(?Send)]
impl TransportReceiver for WasmWebSocketReceiver {
    async fn recv(&mut self) -> Option<TransportEvent> {
        // In WASM, events are pushed to the queue by callbacks
        // We poll the queue here
        let mut state = self.state.borrow_mut();
        if state.event_queue.is_empty() {
            // Yield to allow callbacks to run
            drop(state);

            // Use a small delay to prevent busy-waiting
            let promise = js_sys::Promise::new(&mut |resolve, _| {
                let window = web_sys::window().unwrap();
                window
                    .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 10)
                    .unwrap();
            });
            let _ = JsFuture::from(promise).await;

            let mut state = self.state.borrow_mut();
            if state.event_queue.is_empty() {
                return None;
            }
            Some(state.event_queue.remove(0))
        } else {
            Some(state.event_queue.remove(0))
        }
    }
}

#[async_trait(?Send)]
impl Transport for WasmWebSocketTransport {
    type Sender = WasmWebSocketSender;
    type Receiver = WasmWebSocketReceiver;

    async fn connect(url: &str) -> Result<(Self::Sender, Self::Receiver)> {
        // Create WebSocket with subprotocol
        let protocols = js_sys::Array::new();
        protocols.push(&JsValue::from_str(WS_SUBPROTOCOL));

        let ws = web_sys::WebSocket::new_with_str_sequence(url, &protocols)
            .map_err(|e| TransportError::ConnectionFailed(format!("{:?}", e)))?;

        // Set binary type to arraybuffer for efficient binary handling
        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

        // Create shared state
        let state = Rc::new(RefCell::new(WasmWsState {
            connected: false,
            event_queue: Vec::new(),
        }));

        let mut closures: Vec<Closure<dyn FnMut(web_sys::Event)>> = Vec::new();

        // Setup onopen handler
        {
            let state_clone = state.clone();
            let onopen = Closure::wrap(Box::new(move |_: web_sys::Event| {
                let mut state = state_clone.borrow_mut();
                state.connected = true;
                state.event_queue.push(TransportEvent::Connected);
            }) as Box<dyn FnMut(web_sys::Event)>);
            ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
            closures.push(onopen);
        }

        // Setup onmessage handler
        {
            let state_clone = state.clone();
            let onmessage = Closure::wrap(Box::new(move |event: web_sys::Event| {
                let event = event.dyn_into::<web_sys::MessageEvent>().unwrap();

                // Handle binary data (ArrayBuffer)
                if let Ok(buffer) = event.data().dyn_into::<js_sys::ArrayBuffer>() {
                    let array = js_sys::Uint8Array::new(&buffer);
                    let data = array.to_vec();
                    let mut state = state_clone.borrow_mut();
                    state
                        .event_queue
                        .push(TransportEvent::Data(Bytes::from(data)));
                }
                // Handle Blob data
                else if event.data().is_instance_of::<web_sys::Blob>() {
                    // For simplicity, we set binary_type to arraybuffer, so this shouldn't happen
                    // But we handle it just in case
                    web_sys::console::warn_1(&JsValue::from_str(
                        "Received Blob instead of ArrayBuffer",
                    ));
                }
                // Handle text data
                else if let Some(text) = event.data().as_string() {
                    let mut state = state_clone.borrow_mut();
                    state
                        .event_queue
                        .push(TransportEvent::Data(Bytes::from(text)));
                }
            }) as Box<dyn FnMut(web_sys::Event)>);
            ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
            closures.push(onmessage);
        }

        // Setup onerror handler
        {
            let state_clone = state.clone();
            let onerror = Closure::wrap(Box::new(move |event: web_sys::Event| {
                let event = event.dyn_into::<web_sys::ErrorEvent>().ok();
                let message = event
                    .map(|e| e.message())
                    .unwrap_or_else(|| "Unknown error".to_string());
                let mut state = state_clone.borrow_mut();
                state.event_queue.push(TransportEvent::Error(message));
            }) as Box<dyn FnMut(web_sys::Event)>);
            ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
            closures.push(onerror);
        }

        // Setup onclose handler
        {
            let state_clone = state.clone();
            let onclose = Closure::wrap(Box::new(move |event: web_sys::Event| {
                let event = event.dyn_into::<web_sys::CloseEvent>().unwrap();
                let reason = if event.reason().is_empty() {
                    None
                } else {
                    Some(event.reason())
                };
                let mut state = state_clone.borrow_mut();
                state.connected = false;
                state
                    .event_queue
                    .push(TransportEvent::Disconnected { reason });
            }) as Box<dyn FnMut(web_sys::Event)>);
            ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
            closures.push(onclose);
        }

        // Wait for connection to open
        let connected_state = state.clone();
        let connect_promise = js_sys::Promise::new(&mut |resolve, reject| {
            let state = connected_state.clone();
            let ws_clone = ws.clone();

            // Check periodically if connected
            let check_interval = Closure::wrap(Box::new(move || {
                let state = state.borrow();
                if state.connected {
                    resolve.call0(&JsValue::NULL).unwrap();
                } else if ws_clone.ready_state() == web_sys::WebSocket::CLOSED {
                    reject
                        .call1(&JsValue::NULL, &JsValue::from_str("Connection closed"))
                        .unwrap();
                }
            }) as Box<dyn FnMut()>);

            let window = web_sys::window().unwrap();
            window
                .set_interval_with_callback_and_timeout_and_arguments_0(
                    check_interval.as_ref().unchecked_ref(),
                    50,
                )
                .unwrap();
            check_interval.forget(); // Leak the closure to keep it alive
        });

        // Wait with timeout
        let timeout_promise = js_sys::Promise::new(&mut |_, reject| {
            let window = web_sys::window().unwrap();
            let reject_closure = Closure::wrap(Box::new(move || {
                reject
                    .call1(&JsValue::NULL, &JsValue::from_str("Connection timeout"))
                    .unwrap();
            }) as Box<dyn FnMut()>);
            window
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    reject_closure.as_ref().unchecked_ref(),
                    10000, // 10 second timeout
                )
                .unwrap();
            reject_closure.forget();
        });

        let race = js_sys::Promise::race(&js_sys::Array::of2(&connect_promise, &timeout_promise));
        JsFuture::from(race)
            .await
            .map_err(|e| TransportError::ConnectionFailed(format!("{:?}", e)))?;

        let sender = WasmWebSocketSender::new(ws, state.clone());
        let receiver = WasmWebSocketReceiver {
            state,
            _closures: closures,
        };

        Ok((sender, receiver))
    }

    fn local_addr(&self) -> Option<std::net::SocketAddr> {
        // Not available in WASM
        None
    }

    fn remote_addr(&self) -> Option<std::net::SocketAddr> {
        // Not available in WASM
        None
    }
}

#[cfg(test)]
mod tests {
    // WASM tests would require wasm-bindgen-test
    // See: https://rustwasm.github.io/docs/wasm-bindgen/wasm-bindgen-test/index.html
}
