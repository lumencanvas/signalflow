//! Clasp WebAssembly bindings
//!
//! This crate provides WebAssembly bindings for Clasp,
//! enabling browser-based clients.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::{CloseEvent, ErrorEvent, MessageEvent, WebSocket};

use clasp_core::{
    codec, frame::Frame, HelloMessage, Message, SetMessage, SubscribeMessage, SubscribeOptions,
    Value, PROTOCOL_VERSION, WS_SUBPROTOCOL,
};

#[cfg(feature = "console_error_panic_hook")]
pub fn set_panic_hook() {
    console_error_panic_hook::set_once();
}

/// Initialize the WASM module
#[wasm_bindgen(start)]
pub fn init() {
    #[cfg(feature = "console_error_panic_hook")]
    set_panic_hook();
}

/// Clasp WASM client
#[wasm_bindgen]
pub struct ClaspWasm {
    ws: WebSocket,
    session_id: Rc<RefCell<Option<String>>>,
    connected: Rc<RefCell<bool>>,
    params: Rc<RefCell<HashMap<String, JsValue>>>,
    on_message: Rc<RefCell<Option<js_sys::Function>>>,
    on_connect: Rc<RefCell<Option<js_sys::Function>>>,
    on_disconnect: Rc<RefCell<Option<js_sys::Function>>>,
    on_error: Rc<RefCell<Option<js_sys::Function>>>,
    sub_id: Rc<RefCell<u32>>,
}

#[wasm_bindgen]
impl ClaspWasm {
    /// Create a new Clasp client
    #[wasm_bindgen(constructor)]
    pub fn new(url: &str) -> Result<ClaspWasm, JsValue> {
        // Create WebSocket with subprotocol
        let ws = WebSocket::new_with_str(url, WS_SUBPROTOCOL)?;
        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

        let client = ClaspWasm {
            ws,
            session_id: Rc::new(RefCell::new(None)),
            connected: Rc::new(RefCell::new(false)),
            params: Rc::new(RefCell::new(HashMap::new())),
            on_message: Rc::new(RefCell::new(None)),
            on_connect: Rc::new(RefCell::new(None)),
            on_disconnect: Rc::new(RefCell::new(None)),
            on_error: Rc::new(RefCell::new(None)),
            sub_id: Rc::new(RefCell::new(1)),
        };

        client.setup_handlers()?;

        Ok(client)
    }

    /// Set up WebSocket event handlers
    fn setup_handlers(&self) -> Result<(), JsValue> {
        let connected = self.connected.clone();
        let session_id = self.session_id.clone();
        let params = self.params.clone();
        let on_connect = self.on_connect.clone();
        let on_message = self.on_message.clone();
        let ws = self.ws.clone();

        // onopen handler
        let connected_open = connected.clone();
        let ws_open = ws.clone();
        let onopen = Closure::wrap(Box::new(move |_: JsValue| {
            // Send HELLO
            let hello = Message::Hello(HelloMessage {
                version: PROTOCOL_VERSION,
                name: "Clasp WASM Client".to_string(),
                features: vec![
                    "param".to_string(),
                    "event".to_string(),
                    "stream".to_string(),
                ],
                capabilities: None,
                token: None,
            });

            if let Ok(bytes) = codec::encode(&hello) {
                let array = js_sys::Uint8Array::from(bytes.as_ref());
                let _ = ws_open.send_with_array_buffer(&array.buffer());
            }
        }) as Box<dyn FnMut(JsValue)>);
        self.ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
        onopen.forget();

        // onmessage handler
        let connected_msg = connected.clone();
        let session_msg = session_id.clone();
        let params_msg = params.clone();
        let on_connect_msg = on_connect.clone();
        let on_message_msg = on_message.clone();

        let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
            if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                let array = js_sys::Uint8Array::new(&abuf);
                let bytes: Vec<u8> = array.to_vec();

                if let Ok((msg, _)) = codec::decode(&bytes) {
                    match &msg {
                        Message::Welcome(welcome) => {
                            *session_msg.borrow_mut() = Some(welcome.session.clone());
                            *connected_msg.borrow_mut() = true;

                            if let Some(callback) = on_connect_msg.borrow().as_ref() {
                                let _ = callback.call0(&JsValue::NULL);
                            }
                        }
                        Message::Set(set) => {
                            let js_value = value_to_js(&set.value);
                            params_msg
                                .borrow_mut()
                                .insert(set.address.clone(), js_value.clone());

                            if let Some(callback) = on_message_msg.borrow().as_ref() {
                                let _ = callback.call2(
                                    &JsValue::NULL,
                                    &JsValue::from_str(&set.address),
                                    &js_value,
                                );
                            }
                        }
                        Message::Snapshot(snapshot) => {
                            for param in &snapshot.params {
                                let js_value = value_to_js(&param.value);
                                params_msg
                                    .borrow_mut()
                                    .insert(param.address.clone(), js_value.clone());

                                if let Some(callback) = on_message_msg.borrow().as_ref() {
                                    let _ = callback.call2(
                                        &JsValue::NULL,
                                        &JsValue::from_str(&param.address),
                                        &js_value,
                                    );
                                }
                            }
                        }
                        Message::Publish(pub_msg) => {
                            let value = pub_msg
                                .value
                                .as_ref()
                                .or(pub_msg.payload.as_ref())
                                .map(value_to_js)
                                .unwrap_or(JsValue::NULL);

                            if let Some(callback) = on_message_msg.borrow().as_ref() {
                                let _ = callback.call2(
                                    &JsValue::NULL,
                                    &JsValue::from_str(&pub_msg.address),
                                    &value,
                                );
                            }
                        }
                        _ => {}
                    }
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        self.ws
            .set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        onmessage.forget();

        // onclose handler
        let on_disconnect_close = self.on_disconnect.clone();
        let connected_close = connected.clone();
        let onclose = Closure::wrap(Box::new(move |e: CloseEvent| {
            *connected_close.borrow_mut() = false;
            if let Some(callback) = on_disconnect_close.borrow().as_ref() {
                let _ = callback.call1(&JsValue::NULL, &JsValue::from_str(&e.reason()));
            }
        }) as Box<dyn FnMut(CloseEvent)>);
        self.ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
        onclose.forget();

        // onerror handler
        let on_error_err = self.on_error.clone();
        let onerror = Closure::wrap(Box::new(move |e: ErrorEvent| {
            if let Some(callback) = on_error_err.borrow().as_ref() {
                let _ = callback.call1(&JsValue::NULL, &JsValue::from_str(&e.message()));
            }
        }) as Box<dyn FnMut(ErrorEvent)>);
        self.ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
        onerror.forget();

        Ok(())
    }

    /// Check if connected
    #[wasm_bindgen(getter)]
    pub fn connected(&self) -> bool {
        *self.connected.borrow()
    }

    /// Get session ID
    #[wasm_bindgen(getter)]
    pub fn session_id(&self) -> Option<String> {
        self.session_id.borrow().clone()
    }

    /// Set connection callback
    pub fn set_on_connect(&self, callback: js_sys::Function) {
        *self.on_connect.borrow_mut() = Some(callback);
    }

    /// Set disconnect callback
    pub fn set_on_disconnect(&self, callback: js_sys::Function) {
        *self.on_disconnect.borrow_mut() = Some(callback);
    }

    /// Set message callback
    pub fn set_on_message(&self, callback: js_sys::Function) {
        *self.on_message.borrow_mut() = Some(callback);
    }

    /// Set error callback
    pub fn set_on_error(&self, callback: js_sys::Function) {
        *self.on_error.borrow_mut() = Some(callback);
    }

    /// Subscribe to address pattern
    pub fn subscribe(&self, pattern: &str) -> u32 {
        let id = {
            let mut sub_id = self.sub_id.borrow_mut();
            let id = *sub_id;
            *sub_id += 1;
            id
        };

        let msg = Message::Subscribe(SubscribeMessage {
            id,
            pattern: pattern.to_string(),
            types: vec![],
            options: Some(SubscribeOptions::default()),
        });

        self.send_message(&msg);
        id
    }

    /// Unsubscribe
    pub fn unsubscribe(&self, id: u32) {
        let msg = Message::Unsubscribe(clasp_core::UnsubscribeMessage { id });
        self.send_message(&msg);
    }

    /// Set a value
    pub fn set(&self, address: &str, value: JsValue) {
        let sf_value = js_to_value(&value);
        let msg = Message::Set(SetMessage {
            address: address.to_string(),
            value: sf_value,
            revision: None,
            lock: false,
            unlock: false,
        });
        self.send_message(&msg);
    }

    /// Emit an event
    pub fn emit(&self, address: &str, payload: JsValue) {
        let sf_value = js_to_value(&payload);
        let msg = Message::Publish(clasp_core::PublishMessage {
            address: address.to_string(),
            signal: Some(clasp_core::SignalType::Event),
            value: None,
            payload: Some(sf_value),
            samples: None,
            rate: None,
            id: None,
            phase: None,
            timestamp: None,
        });
        self.send_message(&msg);
    }

    /// Get cached value
    pub fn get(&self, address: &str) -> JsValue {
        self.params
            .borrow()
            .get(address)
            .cloned()
            .unwrap_or(JsValue::NULL)
    }

    /// Close connection
    pub fn close(&self) {
        let _ = self.ws.close();
    }

    /// Send a message
    fn send_message(&self, msg: &Message) {
        if let Ok(bytes) = codec::encode(msg) {
            let array = js_sys::Uint8Array::from(bytes.as_ref());
            let _ = self.ws.send_with_array_buffer(&array.buffer());
        }
    }
}

/// Convert Clasp Value to JsValue
fn value_to_js(value: &Value) -> JsValue {
    match value {
        Value::Null => JsValue::NULL,
        Value::Bool(b) => JsValue::from_bool(*b),
        Value::Int(i) => JsValue::from_f64(*i as f64),
        Value::Float(f) => JsValue::from_f64(*f),
        Value::String(s) => JsValue::from_str(s),
        Value::Bytes(b) => {
            let array = js_sys::Uint8Array::from(b.as_slice());
            array.into()
        }
        Value::Array(arr) => {
            let js_arr = js_sys::Array::new();
            for v in arr {
                js_arr.push(&value_to_js(v));
            }
            js_arr.into()
        }
        Value::Map(map) => {
            let obj = js_sys::Object::new();
            for (k, v) in map {
                js_sys::Reflect::set(&obj, &JsValue::from_str(k), &value_to_js(v)).unwrap();
            }
            obj.into()
        }
    }
}

/// Convert JsValue to Clasp Value
fn js_to_value(js: &JsValue) -> Value {
    if js.is_null() || js.is_undefined() {
        Value::Null
    } else if let Some(b) = js.as_bool() {
        Value::Bool(b)
    } else if let Some(f) = js.as_f64() {
        if f.fract() == 0.0 && f >= i64::MIN as f64 && f <= i64::MAX as f64 {
            Value::Int(f as i64)
        } else {
            Value::Float(f)
        }
    } else if let Some(s) = js.as_string() {
        Value::String(s)
    } else if js_sys::Array::is_array(js) {
        let arr: js_sys::Array = js.clone().into();
        let values: Vec<Value> = arr.iter().map(|v| js_to_value(&v)).collect();
        Value::Array(values)
    } else if js.is_object() {
        let obj: js_sys::Object = js.clone().into();
        let mut map = HashMap::new();
        let keys = js_sys::Object::keys(&obj);
        for key in keys.iter() {
            if let Some(k) = key.as_string() {
                if let Ok(v) = js_sys::Reflect::get(&obj, &key) {
                    map.insert(k, js_to_value(&v));
                }
            }
        }
        Value::Map(map)
    } else {
        Value::Null
    }
}
