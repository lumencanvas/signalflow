//! Comprehensive integration tests for Rendezvous Server
//!
//! Tests HTTP endpoints, concurrent operations, TTL expiration, and error handling.

#[cfg(feature = "rendezvous")]
mod tests {
    use clasp_discovery::rendezvous::{
        DeviceRegistration, RendezvousClient, RendezvousConfig, RendezvousServer,
    };
    use std::collections::HashMap;
    use std::time::Duration;
    use tokio::time::timeout;

    /// Find an available port for testing
    async fn find_available_port() -> u16 {
        use tokio::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        listener.local_addr().unwrap().port()
    }

    /// Helper to create a test device registration
    fn make_test_device(name: &str, tag: &str) -> DeviceRegistration {
        let mut endpoints = HashMap::new();
        endpoints.insert("ws".to_string(), format!("ws://{}.local:7330", name));
        endpoints.insert("tcp".to_string(), format!("tcp://{}.local:7331", name));

        DeviceRegistration {
            name: name.to_string(),
            public_key: Some("test-key".to_string()),
            features: vec!["param".to_string(), "event".to_string()],
            endpoints,
            tags: vec![tag.to_string()],
            metadata: HashMap::new(),
        }
    }

    /// Test: Basic registration and discovery
    #[tokio::test]
    async fn test_register_and_discover() {
        let port = find_available_port().await;
        let addr = format!("127.0.0.1:{}", port);

        let server = RendezvousServer::new(RendezvousConfig::default());
        let addr_clone = addr.clone();
        let server_handle = tokio::spawn(async move {
            let _ = server.serve(&addr_clone).await;
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        let client = RendezvousClient::new(&format!("http://{}", addr));

        // Register device
        let registration = make_test_device("Device1", "studio");
        let response = client.register(registration).await.unwrap();
        assert!(!response.id.is_empty());
        assert!(response.ttl > 0);

        // Discover devices
        let devices = client.discover(None).await.unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].name, "Device1");
        assert_eq!(devices[0].tags, vec!["studio"]);

        server_handle.abort();
    }

    /// Test: Discover with tag filter
    #[tokio::test]
    async fn test_discover_with_tag_filter() {
        let port = find_available_port().await;
        let addr = format!("127.0.0.1:{}", port);

        let server = RendezvousServer::new(RendezvousConfig::default());
        let addr_clone = addr.clone();
        let server_handle = tokio::spawn(async move {
            let _ = server.serve(&addr_clone).await;
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        let client = RendezvousClient::new(&format!("http://{}", addr));

        // Register devices with different tags
        client.register(make_test_device("Studio1", "studio")).await.unwrap();
        client.register(make_test_device("Live1", "live")).await.unwrap();
        client.register(make_test_device("Studio2", "studio")).await.unwrap();

        // Discover by tag
        let studio_devices = client.discover(Some("studio")).await.unwrap();
        assert_eq!(studio_devices.len(), 2);
        assert!(studio_devices.iter().all(|d| d.tags.contains(&"studio".to_string())));

        let live_devices = client.discover(Some("live")).await.unwrap();
        assert_eq!(live_devices.len(), 1);
        assert_eq!(live_devices[0].name, "Live1");

        server_handle.abort();
    }

    /// Test: Unregister device
    #[tokio::test]
    async fn test_unregister() {
        let port = find_available_port().await;
        let addr = format!("127.0.0.1:{}", port);

        let server = RendezvousServer::new(RendezvousConfig::default());
        let addr_clone = addr.clone();
        let server_handle = tokio::spawn(async move {
            let _ = server.serve(&addr_clone).await;
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        let client = RendezvousClient::new(&format!("http://{}", addr));

        // Register
        let response = client.register(make_test_device("Device1", "test")).await.unwrap();
        let device_id = response.id;

        // Should be discoverable
        let devices = client.discover(None).await.unwrap();
        assert_eq!(devices.len(), 1);

        // Unregister
        let success = client.unregister(&device_id).await.unwrap();
        assert!(success);

        // Should not be discoverable
        let devices = client.discover(None).await.unwrap();
        assert_eq!(devices.len(), 0);

        // Unregister again should fail
        let success = client.unregister(&device_id).await.unwrap();
        assert!(!success);

        server_handle.abort();
    }

    /// Test: Refresh registration (extend TTL)
    #[tokio::test]
    async fn test_refresh() {
        let port = find_available_port().await;
        let addr = format!("127.0.0.1:{}", port);

        let server = RendezvousServer::new(RendezvousConfig::default());
        let addr_clone = addr.clone();
        let server_handle = tokio::spawn(async move {
            let _ = server.serve(&addr_clone).await;
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        let client = RendezvousClient::new(&format!("http://{}", addr));

        // Register
        let response = client.register(make_test_device("Device1", "test")).await.unwrap();
        let device_id = response.id;

        // Refresh
        let success = client.refresh(&device_id).await.unwrap();
        assert!(success);

        // Should still be discoverable
        let devices = client.discover(None).await.unwrap();
        assert_eq!(devices.len(), 1);

        // Refresh non-existent should fail
        let success = client.refresh("nonexistent-id").await.unwrap();
        assert!(!success);

        server_handle.abort();
    }

    /// Test: Concurrent registrations
    #[tokio::test]
    async fn test_concurrent_registrations() {
        let port = find_available_port().await;
        let addr = format!("127.0.0.1:{}", port);

        let server = RendezvousServer::new(RendezvousConfig::default());
        let addr_clone = addr.clone();
        let server_handle = tokio::spawn(async move {
            let _ = server.serve(&addr_clone).await;
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        let client = RendezvousClient::new(&format!("http://{}", addr));

        // Register 10 devices concurrently
        let mut handles = vec![];
        for i in 0..10 {
            let client = RendezvousClient::new(&format!("http://{}", addr));
            let device = make_test_device(&format!("Device{}", i), "test");
            handles.push(tokio::spawn(async move {
                client.register(device).await
            }));
        }

        let results = futures::future::join_all(handles).await;
        for result in results {
            assert!(result.unwrap().is_ok());
        }

        // All should be discoverable
        let devices = client.discover(None).await.unwrap();
        assert_eq!(devices.len(), 10);

        server_handle.abort();
    }

    /// Test: TTL expiration
    #[tokio::test]
    async fn test_ttl_expiration() {
        let config = RendezvousConfig {
            ttl: 1, // 1 second TTL
            cleanup_interval: 1,
            ..Default::default()
        };

        let port = find_available_port().await;
        let addr = format!("127.0.0.1:{}", port);

        let server = RendezvousServer::new(config);
        let addr_clone = addr.clone();
        let server_handle = tokio::spawn(async move {
            let _ = server.serve(&addr_clone).await;
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        let client = RendezvousClient::new(&format!("http://{}", addr));

        // Register
        client.register(make_test_device("Device1", "test")).await.unwrap();

        // Should be discoverable immediately
        let devices = client.discover(None).await.unwrap();
        assert_eq!(devices.len(), 1);

        // Wait for TTL + cleanup interval
        tokio::time::sleep(Duration::from_secs(3)).await;

        // Should be expired
        let devices = client.discover(None).await.unwrap();
        assert_eq!(devices.len(), 0);

        server_handle.abort();
    }

    /// Test: Capacity limits
    #[tokio::test]
    async fn test_capacity_limits() {
        let config = RendezvousConfig {
            max_total_devices: 5,
            ..Default::default()
        };

        let port = find_available_port().await;
        let addr = format!("127.0.0.1:{}", port);

        let server = RendezvousServer::new(config);
        let addr_clone = addr.clone();
        let server_handle = tokio::spawn(async move {
            let _ = server.serve(&addr_clone).await;
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        let client = RendezvousClient::new(&format!("http://{}", addr));

        // Register 10 devices (exceeds limit of 5)
        for i in 0..10 {
            client.register(make_test_device(&format!("Device{}", i), "test")).await.unwrap();
        }

        // Should only have 5 devices (oldest removed)
        let devices = client.discover(None).await.unwrap();
        assert_eq!(devices.len(), 5);

        server_handle.abort();
    }

    /// Test: Health endpoint
    #[tokio::test]
    async fn test_health_endpoint() {
        let port = find_available_port().await;
        let addr = format!("127.0.0.1:{}", port);

        let server = RendezvousServer::new(RendezvousConfig::default());
        let addr_clone = addr.clone();
        let server_handle = tokio::spawn(async move {
            let _ = server.serve(&addr_clone).await;
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        let client = reqwest::Client::new();
        let response = client
            .get(&format!("http://{}/api/v1/health", addr))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), reqwest::StatusCode::OK);
        assert_eq!(response.text().await.unwrap(), "OK");

        server_handle.abort();
    }

    /// Test: Invalid registration (missing required fields)
    #[tokio::test]
    async fn test_invalid_registration() {
        let port = find_available_port().await;
        let addr = format!("127.0.0.1:{}", port);

        let server = RendezvousServer::new(RendezvousConfig::default());
        let addr_clone = addr.clone();
        let server_handle = tokio::spawn(async move {
            let _ = server.serve(&addr_clone).await;
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        let client = reqwest::Client::new();

        // Try to register with invalid JSON
        let response = client
            .post(&format!("http://{}/api/v1/register", addr))
            .json(&serde_json::json!({ "invalid": "data" }))
            .send()
            .await
            .unwrap();

        // Should still accept (server is lenient) or return error
        // The server will create a device with defaults
        assert!(response.status().is_success() || response.status().is_client_error());

        server_handle.abort();
    }

    /// Test: Discover with limit
    #[tokio::test]
    async fn test_discover_with_limit() {
        let port = find_available_port().await;
        let addr = format!("127.0.0.1:{}", port);

        let server = RendezvousServer::new(RendezvousConfig::default());
        let addr_clone = addr.clone();
        let server_handle = tokio::spawn(async move {
            let _ = server.serve(&addr_clone).await;
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        let client = RendezvousClient::new(&format!("http://{}", addr));

        // Register 10 devices
        for i in 0..10 {
            client.register(make_test_device(&format!("Device{}", i), "test")).await.unwrap();
        }

        // Discover with limit via query param
        let client_http = reqwest::Client::new();
        let response = client_http
            .get(&format!("http://{}/api/v1/discover?limit=5", addr))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let devices: Vec<serde_json::Value> = response.json().await.unwrap();
        assert_eq!(devices.len(), 5);

        server_handle.abort();
    }

    /// Test: Multiple clients discovering simultaneously
    #[tokio::test]
    async fn test_concurrent_discovery() {
        let port = find_available_port().await;
        let addr = format!("127.0.0.1:{}", port);

        let server = RendezvousServer::new(RendezvousConfig::default());
        let addr_clone = addr.clone();
        let server_handle = tokio::spawn(async move {
            let _ = server.serve(&addr_clone).await;
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        // Register some devices
        let client = RendezvousClient::new(&format!("http://{}", addr));
        for i in 0..5 {
            client.register(make_test_device(&format!("Device{}", i), "test")).await.unwrap();
        }

        // Multiple clients discovering concurrently
        let mut handles = vec![];
        for _ in 0..10 {
            let client = RendezvousClient::new(&format!("http://{}", addr));
            handles.push(tokio::spawn(async move {
                client.discover(None).await
            }));
        }

        let results = futures::future::join_all(handles).await;
        for result in results {
            let devices = result.unwrap().unwrap();
            assert_eq!(devices.len(), 5);
        }

        server_handle.abort();
    }

    /// Test: Device metadata preservation
    #[tokio::test]
    async fn test_device_metadata() {
        let port = find_available_port().await;
        let addr = format!("127.0.0.1:{}", port);

        let server = RendezvousServer::new(RendezvousConfig::default());
        let addr_clone = addr.clone();
        let server_handle = tokio::spawn(async move {
            let _ = server.serve(&addr_clone).await;
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        let client = RendezvousClient::new(&format!("http://{}", addr));

        // Register with metadata
        let mut registration = make_test_device("Device1", "test");
        registration.metadata.insert("version".to_string(), "1.0.0".to_string());
        registration.metadata.insert("platform".to_string(), "macos".to_string());

        client.register(registration).await.unwrap();

        // Discover and verify metadata
        let devices = client.discover(None).await.unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].metadata.get("version"), Some(&"1.0.0".to_string()));
        assert_eq!(devices[0].metadata.get("platform"), Some(&"macos".to_string()));

        server_handle.abort();
    }

    /// Test: Endpoints preservation
    #[tokio::test]
    async fn test_endpoints_preservation() {
        let port = find_available_port().await;
        let addr = format!("127.0.0.1:{}", port);

        let server = RendezvousServer::new(RendezvousConfig::default());
        let addr_clone = addr.clone();
        let server_handle = tokio::spawn(async move {
            let _ = server.serve(&addr_clone).await;
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        let client = RendezvousClient::new(&format!("http://{}", addr));

        // Register with multiple endpoints
        let registration = make_test_device("Device1", "test");
        let endpoint_count = registration.endpoints.len();
        assert!(endpoint_count > 0);

        client.register(registration).await.unwrap();

        // Discover and verify endpoints
        let devices = client.discover(None).await.unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].endpoints.len(), endpoint_count);
        assert!(devices[0].endpoints.contains_key("ws"));
        assert!(devices[0].endpoints.contains_key("tcp"));

        server_handle.abort();
    }
}
