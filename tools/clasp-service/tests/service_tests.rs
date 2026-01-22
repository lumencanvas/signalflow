//! Integration tests for the CLASP Bridge Service
//!
//! These tests verify:
//! - Server startup for various protocols (WebSocket, HTTP, etc.)
//! - Bridge lifecycle (create, list, delete)
//! - Diagnostics and health check functionality
//! - Error handling for invalid configurations

use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::time::timeout;

// ============================================================================
// Test Helpers
// ============================================================================

/// Parse potentially concatenated JSON objects from a string
/// e.g., `{"a":1}{"b":2}` -> [{"a":1}, {"b":2}]
fn parse_concatenated_json(s: &str) -> Vec<serde_json::Value> {
    let mut results = Vec::new();
    let mut remaining = s.trim();

    while !remaining.is_empty() {
        // Try to parse from the start
        let mut stream =
            serde_json::Deserializer::from_str(remaining).into_iter::<serde_json::Value>();

        if let Some(Ok(value)) = stream.next() {
            let bytes_read = stream.byte_offset();
            results.push(value);
            remaining = &remaining[bytes_read..].trim_start();
        } else {
            break;
        }
    }

    results
}

/// Find an available port for testing
async fn find_available_port() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    listener.local_addr().unwrap().port()
}

/// Verify a port is listening
async fn wait_for_port(port: u16, max_wait: Duration) -> bool {
    let start = std::time::Instant::now();
    while start.elapsed() < max_wait {
        if tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port))
            .await
            .is_ok()
        {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    false
}

/// Verify a port is NOT listening
async fn verify_port_closed(port: u16) -> bool {
    tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port))
        .await
        .is_err()
}

/// A test harness for the clasp-service process
struct ServiceHarness {
    child: Child,
    stdin: tokio::process::ChildStdin,
    stdout: BufReader<tokio::process::ChildStdout>,
    pending_responses: Vec<serde_json::Value>,
}

impl ServiceHarness {
    /// Start the service process
    async fn start() -> Result<Self, String> {
        let mut child = Command::new(env!("CARGO_BIN_EXE_clasp-service"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to spawn service: {}", e))?;

        let stdin = child.stdin.take().ok_or("No stdin")?;
        let stdout = child.stdout.take().ok_or("No stdout")?;
        let stdout = BufReader::new(stdout);

        let mut harness = Self {
            child,
            stdin,
            stdout,
            pending_responses: Vec::new(),
        };

        // Wait for ready signal
        let ready = harness.next_response().await?;
        if ready["type"] != "ready" {
            return Err(format!("Expected ready, got: {:?}", ready));
        }

        Ok(harness)
    }

    /// Send a request and get the direct response (not async events)
    async fn request(&mut self, req: &str) -> Result<serde_json::Value, String> {
        self.stdin
            .write_all(req.as_bytes())
            .await
            .map_err(|e| format!("Write failed: {}", e))?;
        self.stdin
            .write_all(b"\n")
            .await
            .map_err(|e| format!("Write newline failed: {}", e))?;
        self.stdin
            .flush()
            .await
            .map_err(|e| format!("Flush failed: {}", e))?;

        // Read responses until we get an "ok" or "error" type (not async events)
        loop {
            let response = self.next_response().await?;

            // Return direct responses, skip async events
            match response["type"].as_str() {
                Some("ok") | Some("error") => return Ok(response),
                Some("bridge_event") | Some("signal") => continue, // Skip async events
                _ => return Ok(response),
            }
        }
    }

    /// Get next parsed JSON response, handling multiple objects per line
    async fn next_response(&mut self) -> Result<serde_json::Value, String> {
        // First check if we have pending responses
        if !self.pending_responses.is_empty() {
            return Ok(self.pending_responses.remove(0));
        }

        // Read lines until we get valid JSON
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while std::time::Instant::now() < deadline {
            let mut line = String::new();
            match timeout(Duration::from_millis(100), self.stdout.read_line(&mut line)).await {
                Ok(Ok(0)) => return Err("EOF".to_string()),
                Ok(Ok(_)) => {}
                Ok(Err(e)) => return Err(format!("Read error: {}", e)),
                Err(_) => continue, // Timeout on this read, try again
            }

            // Parse potentially multiple JSON objects from the line
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue; // Skip empty lines
            }

            let objects = parse_concatenated_json(trimmed);

            if objects.is_empty() {
                continue; // No valid JSON, try next line
            }

            // Store all but the first in pending
            for obj in objects.into_iter().skip(1) {
                self.pending_responses.push(obj);
            }

            // Return the first
            let first = parse_concatenated_json(trimmed).into_iter().next().unwrap();
            return Ok(first);
        }

        Err("Timeout waiting for JSON response".to_string())
    }

    /// Send shutdown and wait for process
    async fn shutdown(mut self) -> Result<(), String> {
        let _ = self.request(r#"{"type":"shutdown"}"#).await;
        let _ = timeout(Duration::from_secs(2), self.child.wait()).await;
        Ok(())
    }
}

// ============================================================================
// Bridge Lifecycle Tests
// ============================================================================

#[tokio::test]
async fn test_service_ready_signal() {
    let harness = ServiceHarness::start().await.expect("Service should start");
    harness.shutdown().await.expect("Shutdown should succeed");
}

#[tokio::test]
async fn test_ping_pong() {
    let mut harness = ServiceHarness::start().await.expect("Service should start");

    let response = harness
        .request(r#"{"type":"ping"}"#)
        .await
        .expect("Ping should work");

    assert_eq!(response["type"], "ok");
    assert_eq!(response["data"]["pong"], true);

    harness.shutdown().await.expect("Shutdown should succeed");
}

#[tokio::test]
async fn test_health_check_empty() {
    let mut harness = ServiceHarness::start().await.expect("Service should start");

    let response = harness
        .request(r#"{"type":"health_check"}"#)
        .await
        .expect("Health check should work");

    assert_eq!(response["type"], "ok");
    assert_eq!(response["data"]["status"], "idle");
    assert_eq!(response["data"]["bridges_total"], 0);
    assert_eq!(response["data"]["bridges_running"], 0);

    harness.shutdown().await.expect("Shutdown should succeed");
}

#[tokio::test]
async fn test_list_bridges_empty() {
    let mut harness = ServiceHarness::start().await.expect("Service should start");

    let response = harness
        .request(r#"{"type":"list_bridges"}"#)
        .await
        .expect("List should work");

    assert_eq!(response["type"], "ok");
    assert!(response["data"].as_array().unwrap().is_empty());

    harness.shutdown().await.expect("Shutdown should succeed");
}

#[tokio::test]
async fn test_invalid_protocol() {
    let mut harness = ServiceHarness::start().await.expect("Service should start");

    let response = harness
        .request(
            r#"{"type":"create_bridge","source":"invalid_proto","source_addr":"localhost:1234","target":"clasp","target_addr":"localhost:7330"}"#,
        )
        .await
        .expect("Request should return response");

    assert_eq!(response["type"], "error");
    assert!(response["message"]
        .as_str()
        .unwrap()
        .contains("Unsupported"));

    harness.shutdown().await.expect("Shutdown should succeed");
}

#[tokio::test]
async fn test_delete_nonexistent_bridge() {
    let mut harness = ServiceHarness::start().await.expect("Service should start");

    let response = harness
        .request(r#"{"type":"delete_bridge","id":"nonexistent-id"}"#)
        .await
        .expect("Request should return response");

    assert_eq!(response["type"], "error");
    assert!(response["message"].as_str().unwrap().contains("not found"));

    harness.shutdown().await.expect("Shutdown should succeed");
}

#[tokio::test]
async fn test_get_diagnostics_nonexistent() {
    let mut harness = ServiceHarness::start().await.expect("Service should start");

    let response = harness
        .request(r#"{"type":"get_diagnostics","bridge_id":"nonexistent"}"#)
        .await
        .expect("Request should return response");

    assert_eq!(response["type"], "error");
    assert!(response["message"].as_str().unwrap().contains("not found"));

    harness.shutdown().await.expect("Shutdown should succeed");
}

// ============================================================================
// WebSocket Server Tests
// ============================================================================

#[tokio::test]
async fn test_websocket_server_startup() {
    let mut harness = ServiceHarness::start().await.expect("Service should start");
    let port = find_available_port().await;

    // Create WebSocket server bridge
    let request = format!(
        r#"{{"type":"create_bridge","id":"ws-test","source":"websocket","source_addr":"127.0.0.1:{}","target":"clasp","target_addr":"localhost:7330","config":{{"mode":"server"}}}}"#,
        port
    );

    let response = harness.request(&request).await.expect("Create should work");
    assert_eq!(response["type"], "ok", "Response: {:?}", response);
    assert_eq!(response["data"]["id"], "ws-test");
    assert_eq!(response["data"]["source"], "websocket");
    assert!(response["data"]["active"].as_bool().unwrap());

    // Verify port is listening
    let listening = wait_for_port(port, Duration::from_secs(2)).await;
    assert!(
        listening,
        "WebSocket server should be listening on port {}",
        port
    );

    // Verify in list
    let list = harness
        .request(r#"{"type":"list_bridges"}"#)
        .await
        .expect("List should work");
    assert_eq!(list["data"].as_array().unwrap().len(), 1);
    assert_eq!(list["data"][0]["id"], "ws-test");

    // Check health
    let health = harness
        .request(r#"{"type":"health_check"}"#)
        .await
        .expect("Health check should work");
    assert_eq!(health["data"]["bridges_running"], 1);

    // Delete bridge
    let del = harness
        .request(r#"{"type":"delete_bridge","id":"ws-test"}"#)
        .await
        .expect("Delete should work");
    assert_eq!(del["type"], "ok");

    // Verify port is no longer listening (give it time to close)
    tokio::time::sleep(Duration::from_millis(100)).await;
    let closed = verify_port_closed(port).await;
    assert!(closed, "Port should be closed after bridge deletion");

    harness.shutdown().await.expect("Shutdown should succeed");
}

#[tokio::test]
async fn test_websocket_server_diagnostics() {
    let mut harness = ServiceHarness::start().await.expect("Service should start");
    let port = find_available_port().await;

    // Create WebSocket server
    let request = format!(
        r#"{{"type":"create_bridge","id":"ws-diag","source":"websocket","source_addr":"127.0.0.1:{}","target":"clasp","target_addr":"localhost:7330","config":{{"mode":"server"}}}}"#,
        port
    );
    harness.request(&request).await.expect("Create should work");

    // Wait for server to start
    wait_for_port(port, Duration::from_secs(2)).await;

    // Get diagnostics
    let diag = harness
        .request(r#"{"type":"get_diagnostics","bridge_id":"ws-diag"}"#)
        .await
        .expect("Diagnostics should work");

    assert_eq!(diag["type"], "ok");
    assert_eq!(diag["data"]["id"], "ws-diag");
    assert_eq!(diag["data"]["protocol"], "websocket");
    assert_eq!(diag["data"]["status"], "running");
    assert!(diag["data"]["metrics"]["messages_received"]
        .as_u64()
        .is_some());

    // Cleanup
    harness
        .request(r#"{"type":"delete_bridge","id":"ws-diag"}"#)
        .await
        .ok();
    harness.shutdown().await.expect("Shutdown should succeed");
}

// ============================================================================
// HTTP Server Tests
// ============================================================================

#[tokio::test]
async fn test_http_server_startup() {
    let mut harness = ServiceHarness::start().await.expect("Service should start");
    let port = find_available_port().await;

    // Create HTTP server bridge
    let request = format!(
        r#"{{"type":"create_bridge","id":"http-test","source":"http","source_addr":"127.0.0.1:{}","target":"clasp","target_addr":"localhost:7330","config":{{"base_path":"/api","cors":true}}}}"#,
        port
    );

    let response = harness.request(&request).await.expect("Create should work");
    assert_eq!(response["type"], "ok", "Response: {:?}", response);
    assert_eq!(response["data"]["id"], "http-test");
    assert_eq!(response["data"]["source"], "http");

    // Verify port is listening
    let listening = wait_for_port(port, Duration::from_secs(2)).await;
    assert!(
        listening,
        "HTTP server should be listening on port {}",
        port
    );

    // Cleanup
    harness
        .request(r#"{"type":"delete_bridge","id":"http-test"}"#)
        .await
        .ok();
    harness.shutdown().await.expect("Shutdown should succeed");
}

// ============================================================================
// Multiple Bridge Tests
// ============================================================================

#[tokio::test]
async fn test_multiple_bridges() {
    let mut harness = ServiceHarness::start().await.expect("Service should start");
    let port1 = find_available_port().await;
    let port2 = find_available_port().await;

    // Create two WebSocket servers
    let req1 = format!(
        r#"{{"type":"create_bridge","id":"ws-1","source":"websocket","source_addr":"127.0.0.1:{}","target":"clasp","target_addr":"localhost:7330","config":{{"mode":"server"}}}}"#,
        port1
    );
    let req2 = format!(
        r#"{{"type":"create_bridge","id":"ws-2","source":"websocket","source_addr":"127.0.0.1:{}","target":"clasp","target_addr":"localhost:7330","config":{{"mode":"server"}}}}"#,
        port2
    );

    harness.request(&req1).await.expect("Create 1 should work");
    harness.request(&req2).await.expect("Create 2 should work");

    // Verify both listening
    assert!(wait_for_port(port1, Duration::from_secs(2)).await);
    assert!(wait_for_port(port2, Duration::from_secs(2)).await);

    // List should show 2
    let list = harness
        .request(r#"{"type":"list_bridges"}"#)
        .await
        .expect("List should work");
    assert_eq!(list["data"].as_array().unwrap().len(), 2);

    // Health check
    let health = harness
        .request(r#"{"type":"health_check"}"#)
        .await
        .expect("Health check should work");
    assert_eq!(health["data"]["bridges_total"], 2);
    assert_eq!(health["data"]["bridges_running"], 2);
    assert_eq!(health["data"]["status"], "healthy");

    // Delete first
    harness
        .request(r#"{"type":"delete_bridge","id":"ws-1"}"#)
        .await
        .expect("Delete should work");

    // Health should be degraded or still show 1 running
    let health2 = harness
        .request(r#"{"type":"health_check"}"#)
        .await
        .expect("Health check should work");
    assert_eq!(health2["data"]["bridges_total"], 1);

    // Cleanup
    harness
        .request(r#"{"type":"delete_bridge","id":"ws-2"}"#)
        .await
        .ok();
    harness.shutdown().await.expect("Shutdown should succeed");
}

// ============================================================================
// Bridge Info Tests
// ============================================================================

#[tokio::test]
async fn test_bridge_info_has_metrics() {
    let mut harness = ServiceHarness::start().await.expect("Service should start");
    let port = find_available_port().await;

    // Create bridge
    let request = format!(
        r#"{{"type":"create_bridge","id":"info-test","source":"websocket","source_addr":"127.0.0.1:{}","target":"clasp","target_addr":"localhost:7330","config":{{"mode":"server"}}}}"#,
        port
    );
    harness.request(&request).await.expect("Create should work");
    wait_for_port(port, Duration::from_secs(2)).await;

    // Wait a moment for uptime
    tokio::time::sleep(Duration::from_millis(100)).await;

    // List bridges
    let list = harness
        .request(r#"{"type":"list_bridges"}"#)
        .await
        .expect("List should work");

    let bridge = &list["data"][0];
    assert_eq!(bridge["id"], "info-test");
    assert!(bridge["started_at"].as_u64().is_some());
    assert!(bridge["uptime_secs"].as_u64().is_some());
    assert!(bridge["messages_sent"].as_u64().is_some());
    assert!(bridge["messages_received"].as_u64().is_some());

    // Cleanup
    harness
        .request(r#"{"type":"delete_bridge","id":"info-test"}"#)
        .await
        .ok();
    harness.shutdown().await.expect("Shutdown should succeed");
}

// ============================================================================
// Error Recovery Tests
// ============================================================================

#[tokio::test]
async fn test_malformed_json_recovery() {
    let mut harness = ServiceHarness::start().await.expect("Service should start");

    // Send malformed JSON
    harness.stdin.write_all(b"not valid json\n").await.unwrap();
    harness.stdin.flush().await.unwrap();

    // Should get error response
    let parsed = harness.next_response().await.expect("Should get response");
    assert_eq!(parsed["type"], "error");

    // Service should still be functional
    let ping = harness
        .request(r#"{"type":"ping"}"#)
        .await
        .expect("Ping should still work");
    assert_eq!(ping["data"]["pong"], true);

    harness.shutdown().await.expect("Shutdown should succeed");
}

#[tokio::test]
async fn test_unknown_request_type() {
    let mut harness = ServiceHarness::start().await.expect("Service should start");

    // Send unknown request type
    let response = harness.request(r#"{"type":"unknown_type"}"#).await;

    // Should get error (serde will fail to parse)
    assert!(response.is_err() || response.unwrap()["type"] == "error");

    // Service should still be functional
    let ping = harness
        .request(r#"{"type":"ping"}"#)
        .await
        .expect("Ping should still work");
    assert_eq!(ping["data"]["pong"], true);

    harness.shutdown().await.expect("Shutdown should succeed");
}
