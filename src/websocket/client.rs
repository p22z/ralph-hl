//! WebSocket client implementation

use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, watch, Mutex, RwLock};
use tokio::time::{interval, Instant};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Error as WsError, Message},
    MaybeTlsStream, WebSocketStream,
};

use crate::client::Network;
use crate::error::{Error, Result};

/// WebSocket URL for mainnet
pub const MAINNET_WS_URL: &str = "wss://api.hyperliquid.xyz/ws";

/// WebSocket URL for testnet
pub const TESTNET_WS_URL: &str = "wss://api.hyperliquid-testnet.xyz/ws";

/// Connection state of the WebSocket client
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Not connected
    Disconnected,
    /// Attempting to connect
    Connecting,
    /// Connected and ready
    Connected,
    /// Connection lost, attempting to reconnect
    Reconnecting,
    /// Client has been closed
    Closed,
}

impl std::fmt::Display for ConnectionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionState::Disconnected => write!(f, "Disconnected"),
            ConnectionState::Connecting => write!(f, "Connecting"),
            ConnectionState::Connected => write!(f, "Connected"),
            ConnectionState::Reconnecting => write!(f, "Reconnecting"),
            ConnectionState::Closed => write!(f, "Closed"),
        }
    }
}

/// Configuration for automatic reconnection
#[derive(Debug, Clone)]
pub struct ReconnectConfig {
    /// Whether to automatically reconnect on disconnect
    pub enabled: bool,
    /// Initial delay before first reconnect attempt
    pub initial_delay: Duration,
    /// Maximum delay between reconnect attempts
    pub max_delay: Duration,
    /// Multiplier for exponential backoff
    pub backoff_multiplier: f64,
    /// Maximum number of reconnect attempts (None for unlimited)
    pub max_attempts: Option<u32>,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            max_attempts: None,
        }
    }
}

impl ReconnectConfig {
    /// Create a new reconnect config with reconnection disabled
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }

    /// Calculate the delay for a given attempt number (0-indexed)
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return self.initial_delay;
        }

        let multiplier = self.backoff_multiplier.powi(attempt as i32);
        let delay_ms = (self.initial_delay.as_millis() as f64 * multiplier) as u64;
        let delay = Duration::from_millis(delay_ms);

        std::cmp::min(delay, self.max_delay)
    }

    /// Check if another reconnect attempt should be made
    pub fn should_attempt(&self, attempt: u32) -> bool {
        if !self.enabled {
            return false;
        }
        match self.max_attempts {
            Some(max) => attempt < max,
            None => true,
        }
    }
}

/// Message received from the WebSocket
#[derive(Debug, Clone)]
pub enum WsMessage {
    /// Text message (JSON data)
    Text(String),
    /// Binary message
    Binary(Vec<u8>),
    /// Ping message
    Ping(Vec<u8>),
    /// Pong message
    Pong(Vec<u8>),
    /// Connection closed
    Close,
}

impl From<Message> for WsMessage {
    fn from(msg: Message) -> Self {
        match msg {
            Message::Text(text) => WsMessage::Text(text),
            Message::Binary(data) => WsMessage::Binary(data),
            Message::Ping(data) => WsMessage::Ping(data),
            Message::Pong(data) => WsMessage::Pong(data),
            Message::Close(_) => WsMessage::Close,
            Message::Frame(_) => WsMessage::Binary(vec![]),
        }
    }
}

type WsSink = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
type WsStream = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

/// WebSocket client for Hyperliquid
///
/// Provides connection management, automatic reconnection, and heartbeat handling.
pub struct WsClient {
    /// The network to connect to
    network: Network,
    /// Current connection state
    state: Arc<RwLock<ConnectionState>>,
    /// State change notifier
    state_tx: watch::Sender<ConnectionState>,
    /// State change receiver
    state_rx: watch::Receiver<ConnectionState>,
    /// WebSocket sink for sending messages
    sink: Arc<Mutex<Option<WsSink>>>,
    /// Channel to receive incoming messages
    incoming_rx: Arc<Mutex<mpsc::Receiver<WsMessage>>>,
    /// Channel to send incoming messages (for the reader task)
    incoming_tx: mpsc::Sender<WsMessage>,
    /// Reconnection configuration
    reconnect_config: ReconnectConfig,
    /// Current reconnect attempt count
    reconnect_attempts: Arc<AtomicU64>,
    /// Last ping received timestamp
    last_ping: Arc<RwLock<Option<Instant>>>,
    /// Shutdown signal sender
    shutdown_tx: watch::Sender<bool>,
    /// Shutdown signal receiver
    shutdown_rx: watch::Receiver<bool>,
}

impl std::fmt::Debug for WsClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WsClient")
            .field("network", &self.network)
            .field("reconnect_config", &self.reconnect_config)
            .finish_non_exhaustive()
    }
}

impl WsClient {
    /// Create a new WebSocket client for the specified network
    pub fn new(network: Network) -> Self {
        Self::with_config(network, ReconnectConfig::default())
    }

    /// Create a new WebSocket client with custom reconnect configuration
    pub fn with_config(network: Network, reconnect_config: ReconnectConfig) -> Self {
        let (state_tx, state_rx) = watch::channel(ConnectionState::Disconnected);
        let (incoming_tx, incoming_rx) = mpsc::channel(100);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        Self {
            network,
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            state_tx,
            state_rx,
            sink: Arc::new(Mutex::new(None)),
            incoming_rx: Arc::new(Mutex::new(incoming_rx)),
            incoming_tx,
            reconnect_config,
            reconnect_attempts: Arc::new(AtomicU64::new(0)),
            last_ping: Arc::new(RwLock::new(None)),
            shutdown_tx,
            shutdown_rx,
        }
    }

    /// Create a new WebSocket client for mainnet
    pub fn mainnet() -> Self {
        Self::new(Network::Mainnet)
    }

    /// Create a new WebSocket client for testnet
    pub fn testnet() -> Self {
        Self::new(Network::Testnet)
    }

    /// Get the WebSocket URL for this client's network
    pub fn ws_url(&self) -> &'static str {
        match self.network {
            Network::Mainnet => MAINNET_WS_URL,
            Network::Testnet => TESTNET_WS_URL,
        }
    }

    /// Get the current network
    pub fn network(&self) -> Network {
        self.network
    }

    /// Get the current connection state
    pub async fn state(&self) -> ConnectionState {
        *self.state.read().await
    }

    /// Check if currently connected
    pub async fn is_connected(&self) -> bool {
        *self.state.read().await == ConnectionState::Connected
    }

    /// Subscribe to state changes
    pub fn state_receiver(&self) -> watch::Receiver<ConnectionState> {
        self.state_rx.clone()
    }

    /// Get the last ping timestamp
    pub async fn last_ping_time(&self) -> Option<Instant> {
        *self.last_ping.read().await
    }

    /// Get the number of reconnect attempts
    pub fn reconnect_attempts(&self) -> u64 {
        self.reconnect_attempts.load(Ordering::Relaxed)
    }

    /// Update the connection state
    async fn set_state(&self, state: ConnectionState) {
        let mut current_state = self.state.write().await;
        if *current_state != state {
            *current_state = state;
            let _ = self.state_tx.send(state);
        }
    }

    /// Connect to the WebSocket server
    ///
    /// Returns an error if already connected or connection fails.
    pub async fn connect(&self) -> Result<()> {
        // Check current state
        let current_state = self.state().await;
        if current_state == ConnectionState::Connected {
            return Ok(());
        }
        if current_state == ConnectionState::Closed {
            return Err(Error::WebSocket("Client has been closed".to_string()));
        }

        self.set_state(ConnectionState::Connecting).await;

        match self.establish_connection().await {
            Ok(()) => {
                self.reconnect_attempts.store(0, Ordering::Relaxed);
                self.set_state(ConnectionState::Connected).await;
                Ok(())
            }
            Err(e) => {
                self.set_state(ConnectionState::Disconnected).await;
                Err(e)
            }
        }
    }

    /// Establish the actual WebSocket connection
    async fn establish_connection(&self) -> Result<()> {
        let url = self.ws_url();

        let (ws_stream, _response) = connect_async(url).await.map_err(|e| match e {
            WsError::Io(io_err) => Error::WebSocket(format!("IO error: {io_err}")),
            WsError::Tls(tls_err) => Error::WebSocket(format!("TLS error: {tls_err}")),
            WsError::ConnectionClosed => Error::WebSocket("Connection closed".to_string()),
            WsError::AlreadyClosed => Error::WebSocket("Already closed".to_string()),
            WsError::Protocol(p) => Error::WebSocket(format!("Protocol error: {p}")),
            WsError::Url(u) => Error::WebSocket(format!("URL error: {u}")),
            WsError::Http(resp) => {
                Error::WebSocket(format!("HTTP error: status {}", resp.status()))
            }
            WsError::HttpFormat(e) => Error::WebSocket(format!("HTTP format error: {e}")),
            _ => Error::WebSocket(format!("WebSocket error: {e}")),
        })?;

        let (sink, stream) = ws_stream.split();

        // Store the sink for sending messages
        *self.sink.lock().await = Some(sink);

        // Spawn the reader task
        self.spawn_reader_task(stream);

        // Spawn the heartbeat task
        self.spawn_heartbeat_task();

        Ok(())
    }

    /// Spawn a task to read from the WebSocket stream
    fn spawn_reader_task(&self, mut stream: WsStream) {
        let incoming_tx = self.incoming_tx.clone();
        let last_ping = self.last_ping.clone();
        let state = self.state.clone();
        let state_tx = self.state_tx.clone();
        let sink = self.sink.clone();
        let reconnect_config = self.reconnect_config.clone();
        let reconnect_attempts = self.reconnect_attempts.clone();
        let mut shutdown_rx = self.shutdown_rx.clone();
        let ws_url = self.ws_url();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Check for shutdown signal
                    _ = shutdown_rx.changed() => {
                        if *shutdown_rx.borrow() {
                            break;
                        }
                    }
                    // Read from WebSocket
                    msg = stream.next() => {
                        match msg {
                            Some(Ok(message)) => {
                                // Handle ping/pong at the protocol level
                                if let Message::Ping(data) = &message {
                                    *last_ping.write().await = Some(Instant::now());
                                    // Send pong response
                                    if let Some(ref mut s) = *sink.lock().await {
                                        let _ = s.send(Message::Pong(data.clone())).await;
                                    }
                                }

                                let ws_msg = WsMessage::from(message);

                                // Forward to incoming channel
                                if incoming_tx.send(ws_msg).await.is_err() {
                                    // Receiver dropped, stop reading
                                    break;
                                }
                            }
                            Some(Err(e)) => {
                                // WebSocket error
                                let _ = incoming_tx.send(WsMessage::Close).await;

                                // Try to reconnect if configured
                                if reconnect_config.enabled {
                                    let attempt = reconnect_attempts.fetch_add(1, Ordering::Relaxed) as u32;
                                    if reconnect_config.should_attempt(attempt) {
                                        *state.write().await = ConnectionState::Reconnecting;
                                        let _ = state_tx.send(ConnectionState::Reconnecting);

                                        let delay = reconnect_config.delay_for_attempt(attempt);
                                        tokio::time::sleep(delay).await;

                                        // Attempt reconnection
                                        if let Ok((new_stream, _)) = connect_async(ws_url).await {
                                            let (new_sink, new_stream_reader) = new_stream.split();
                                            *sink.lock().await = Some(new_sink);
                                            stream = new_stream_reader;

                                            reconnect_attempts.store(0, Ordering::Relaxed);
                                            *state.write().await = ConnectionState::Connected;
                                            let _ = state_tx.send(ConnectionState::Connected);
                                            continue;
                                        }
                                    } else {
                                        // Max attempts reached
                                        *state.write().await = ConnectionState::Disconnected;
                                        let _ = state_tx.send(ConnectionState::Disconnected);
                                        break;
                                    }
                                } else {
                                    *state.write().await = ConnectionState::Disconnected;
                                    let _ = state_tx.send(ConnectionState::Disconnected);
                                    break;
                                }

                                // Log the error (could be enhanced with proper logging)
                                eprintln!("WebSocket error: {e:?}");
                            }
                            None => {
                                // Stream ended
                                let _ = incoming_tx.send(WsMessage::Close).await;

                                // Try to reconnect if configured
                                if reconnect_config.enabled {
                                    let attempt = reconnect_attempts.fetch_add(1, Ordering::Relaxed) as u32;
                                    if reconnect_config.should_attempt(attempt) {
                                        *state.write().await = ConnectionState::Reconnecting;
                                        let _ = state_tx.send(ConnectionState::Reconnecting);

                                        let delay = reconnect_config.delay_for_attempt(attempt);
                                        tokio::time::sleep(delay).await;

                                        // Attempt reconnection
                                        if let Ok((new_stream, _)) = connect_async(ws_url).await {
                                            let (new_sink, new_stream_reader) = new_stream.split();
                                            *sink.lock().await = Some(new_sink);
                                            stream = new_stream_reader;

                                            reconnect_attempts.store(0, Ordering::Relaxed);
                                            *state.write().await = ConnectionState::Connected;
                                            let _ = state_tx.send(ConnectionState::Connected);
                                            continue;
                                        }
                                    } else {
                                        *state.write().await = ConnectionState::Disconnected;
                                        let _ = state_tx.send(ConnectionState::Disconnected);
                                        break;
                                    }
                                } else {
                                    *state.write().await = ConnectionState::Disconnected;
                                    let _ = state_tx.send(ConnectionState::Disconnected);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        });
    }

    /// Spawn a task to send periodic heartbeat pings
    fn spawn_heartbeat_task(&self) {
        let sink = self.sink.clone();
        let state = self.state.clone();
        let mut shutdown_rx = self.shutdown_rx.clone();

        tokio::spawn(async move {
            let mut heartbeat_interval = interval(Duration::from_secs(30));

            loop {
                tokio::select! {
                    _ = shutdown_rx.changed() => {
                        if *shutdown_rx.borrow() {
                            break;
                        }
                    }
                    _ = heartbeat_interval.tick() => {
                        // Only send ping if connected
                        if *state.read().await == ConnectionState::Connected {
                            if let Some(ref mut s) = *sink.lock().await {
                                let ping_data = b"ping".to_vec();
                                if s.send(Message::Ping(ping_data)).await.is_err() {
                                    // Failed to send ping, connection might be broken
                                    // The reader task will handle reconnection
                                }
                            }
                        }
                    }
                }
            }
        });
    }

    /// Send a text message over the WebSocket
    pub async fn send_text(&self, text: &str) -> Result<()> {
        if !self.is_connected().await {
            return Err(Error::WebSocket("Not connected".to_string()));
        }

        let mut sink_guard = self.sink.lock().await;
        if let Some(ref mut sink) = *sink_guard {
            sink.send(Message::Text(text.to_string()))
                .await
                .map_err(|e| Error::WebSocket(format!("Failed to send: {e}")))?;
            Ok(())
        } else {
            Err(Error::WebSocket("No connection".to_string()))
        }
    }

    /// Send a JSON message over the WebSocket
    pub async fn send_json<T: serde::Serialize>(&self, value: &T) -> Result<()> {
        let text =
            serde_json::to_string(value).map_err(|e| Error::WebSocket(format!("JSON error: {e}")))?;
        self.send_text(&text).await
    }

    /// Send a binary message over the WebSocket
    pub async fn send_binary(&self, data: Vec<u8>) -> Result<()> {
        if !self.is_connected().await {
            return Err(Error::WebSocket("Not connected".to_string()));
        }

        let mut sink_guard = self.sink.lock().await;
        if let Some(ref mut sink) = *sink_guard {
            sink.send(Message::Binary(data))
                .await
                .map_err(|e| Error::WebSocket(format!("Failed to send: {e}")))?;
            Ok(())
        } else {
            Err(Error::WebSocket("No connection".to_string()))
        }
    }

    /// Receive the next message from the WebSocket
    ///
    /// Returns `None` if the connection is closed.
    pub async fn recv(&self) -> Option<WsMessage> {
        let mut rx = self.incoming_rx.lock().await;
        rx.recv().await
    }

    /// Try to receive a message without blocking
    pub async fn try_recv(&self) -> Option<WsMessage> {
        let mut rx = self.incoming_rx.lock().await;
        rx.try_recv().ok()
    }

    /// Disconnect from the WebSocket server
    pub async fn disconnect(&self) -> Result<()> {
        // Send close message
        let mut sink_guard = self.sink.lock().await;
        if let Some(ref mut sink) = *sink_guard {
            let _ = sink.send(Message::Close(None)).await;
            let _ = sink.close().await;
        }
        *sink_guard = None;
        drop(sink_guard);

        self.set_state(ConnectionState::Disconnected).await;
        Ok(())
    }

    /// Close the WebSocket client permanently
    ///
    /// After calling this, the client cannot be reconnected.
    pub async fn close(&self) -> Result<()> {
        // Signal shutdown to all tasks
        let _ = self.shutdown_tx.send(true);

        // Disconnect
        self.disconnect().await?;

        // Set state to closed
        self.set_state(ConnectionState::Closed).await;

        Ok(())
    }
}

impl Clone for WsClient {
    fn clone(&self) -> Self {
        // Create new channels for the clone
        let (state_tx, state_rx) = watch::channel(ConnectionState::Disconnected);
        let (incoming_tx, incoming_rx) = mpsc::channel(100);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        Self {
            network: self.network,
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            state_tx,
            state_rx,
            sink: Arc::new(Mutex::new(None)),
            incoming_rx: Arc::new(Mutex::new(incoming_rx)),
            incoming_tx,
            reconnect_config: self.reconnect_config.clone(),
            reconnect_attempts: Arc::new(AtomicU64::new(0)),
            last_ping: Arc::new(RwLock::new(None)),
            shutdown_tx,
            shutdown_rx,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============ ConnectionState Tests ============

    #[test]
    fn test_connection_state_display() {
        assert_eq!(ConnectionState::Disconnected.to_string(), "Disconnected");
        assert_eq!(ConnectionState::Connecting.to_string(), "Connecting");
        assert_eq!(ConnectionState::Connected.to_string(), "Connected");
        assert_eq!(ConnectionState::Reconnecting.to_string(), "Reconnecting");
        assert_eq!(ConnectionState::Closed.to_string(), "Closed");
    }

    #[test]
    fn test_connection_state_equality() {
        assert_eq!(ConnectionState::Connected, ConnectionState::Connected);
        assert_ne!(ConnectionState::Connected, ConnectionState::Disconnected);
    }

    #[test]
    fn test_connection_state_clone() {
        let state = ConnectionState::Connected;
        let cloned = state;
        assert_eq!(state, cloned);
    }

    // ============ ReconnectConfig Tests ============

    #[test]
    fn test_reconnect_config_default() {
        let config = ReconnectConfig::default();
        assert!(config.enabled);
        assert_eq!(config.initial_delay, Duration::from_millis(500));
        assert_eq!(config.max_delay, Duration::from_secs(30));
        assert_eq!(config.backoff_multiplier, 2.0);
        assert!(config.max_attempts.is_none());
    }

    #[test]
    fn test_reconnect_config_disabled() {
        let config = ReconnectConfig::disabled();
        assert!(!config.enabled);
    }

    #[test]
    fn test_reconnect_config_delay_calculation() {
        let config = ReconnectConfig {
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 2.0,
            ..Default::default()
        };

        assert_eq!(config.delay_for_attempt(0), Duration::from_millis(100));
        assert_eq!(config.delay_for_attempt(1), Duration::from_millis(200));
        assert_eq!(config.delay_for_attempt(2), Duration::from_millis(400));
        assert_eq!(config.delay_for_attempt(3), Duration::from_millis(800));
    }

    #[test]
    fn test_reconnect_config_max_delay_cap() {
        let config = ReconnectConfig {
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(5),
            backoff_multiplier: 10.0,
            ..Default::default()
        };

        // After a few attempts, should hit max delay
        assert_eq!(config.delay_for_attempt(0), Duration::from_secs(1));
        assert_eq!(config.delay_for_attempt(1), Duration::from_secs(5)); // Capped at max
        assert_eq!(config.delay_for_attempt(5), Duration::from_secs(5)); // Still capped
    }

    #[test]
    fn test_reconnect_config_should_attempt() {
        let unlimited = ReconnectConfig::default();
        assert!(unlimited.should_attempt(0));
        assert!(unlimited.should_attempt(100));
        assert!(unlimited.should_attempt(1000));

        let limited = ReconnectConfig {
            max_attempts: Some(3),
            ..Default::default()
        };
        assert!(limited.should_attempt(0));
        assert!(limited.should_attempt(1));
        assert!(limited.should_attempt(2));
        assert!(!limited.should_attempt(3));
        assert!(!limited.should_attempt(4));

        let disabled = ReconnectConfig::disabled();
        assert!(!disabled.should_attempt(0));
    }

    // ============ WsMessage Tests ============

    #[test]
    fn test_ws_message_from_text() {
        let msg = Message::Text("hello".to_string());
        let ws_msg = WsMessage::from(msg);
        match ws_msg {
            WsMessage::Text(s) => assert_eq!(s, "hello"),
            _ => panic!("Expected Text message"),
        }
    }

    #[test]
    fn test_ws_message_from_binary() {
        let msg = Message::Binary(vec![1, 2, 3]);
        let ws_msg = WsMessage::from(msg);
        match ws_msg {
            WsMessage::Binary(data) => assert_eq!(data, vec![1, 2, 3]),
            _ => panic!("Expected Binary message"),
        }
    }

    #[test]
    fn test_ws_message_from_ping() {
        let msg = Message::Ping(vec![4, 5, 6]);
        let ws_msg = WsMessage::from(msg);
        match ws_msg {
            WsMessage::Ping(data) => assert_eq!(data, vec![4, 5, 6]),
            _ => panic!("Expected Ping message"),
        }
    }

    #[test]
    fn test_ws_message_from_pong() {
        let msg = Message::Pong(vec![7, 8, 9]);
        let ws_msg = WsMessage::from(msg);
        match ws_msg {
            WsMessage::Pong(data) => assert_eq!(data, vec![7, 8, 9]),
            _ => panic!("Expected Pong message"),
        }
    }

    #[test]
    fn test_ws_message_from_close() {
        let msg = Message::Close(None);
        let ws_msg = WsMessage::from(msg);
        assert!(matches!(ws_msg, WsMessage::Close));
    }

    #[test]
    fn test_ws_message_clone() {
        let msg = WsMessage::Text("test".to_string());
        let cloned = msg.clone();
        match (msg, cloned) {
            (WsMessage::Text(a), WsMessage::Text(b)) => assert_eq!(a, b),
            _ => panic!("Clone mismatch"),
        }
    }

    // ============ WsClient Creation Tests ============

    #[test]
    fn test_ws_client_new_mainnet() {
        let client = WsClient::new(Network::Mainnet);
        assert_eq!(client.network(), Network::Mainnet);
        assert_eq!(client.ws_url(), MAINNET_WS_URL);
    }

    #[test]
    fn test_ws_client_new_testnet() {
        let client = WsClient::new(Network::Testnet);
        assert_eq!(client.network(), Network::Testnet);
        assert_eq!(client.ws_url(), TESTNET_WS_URL);
    }

    #[test]
    fn test_ws_client_mainnet_helper() {
        let client = WsClient::mainnet();
        assert_eq!(client.network(), Network::Mainnet);
    }

    #[test]
    fn test_ws_client_testnet_helper() {
        let client = WsClient::testnet();
        assert_eq!(client.network(), Network::Testnet);
    }

    #[tokio::test]
    async fn test_ws_client_initial_state() {
        let client = WsClient::mainnet();
        assert_eq!(client.state().await, ConnectionState::Disconnected);
        assert!(!client.is_connected().await);
    }

    #[test]
    fn test_ws_client_with_config() {
        let config = ReconnectConfig {
            enabled: false,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            backoff_multiplier: 3.0,
            max_attempts: Some(5),
        };
        let client = WsClient::with_config(Network::Mainnet, config);
        assert_eq!(client.network(), Network::Mainnet);
    }

    #[tokio::test]
    async fn test_ws_client_state_receiver() {
        let client = WsClient::mainnet();
        let rx = client.state_receiver();
        assert_eq!(*rx.borrow(), ConnectionState::Disconnected);
    }

    #[test]
    fn test_ws_client_reconnect_attempts_initial() {
        let client = WsClient::mainnet();
        assert_eq!(client.reconnect_attempts(), 0);
    }

    #[tokio::test]
    async fn test_ws_client_last_ping_initial() {
        let client = WsClient::mainnet();
        assert!(client.last_ping_time().await.is_none());
    }

    #[test]
    fn test_ws_client_clone() {
        let client = WsClient::mainnet();
        let cloned = client.clone();
        assert_eq!(client.network(), cloned.network());
    }

    #[test]
    fn test_ws_client_debug() {
        let client = WsClient::mainnet();
        let debug_str = format!("{:?}", client);
        assert!(debug_str.contains("WsClient"));
        assert!(debug_str.contains("Mainnet"));
    }

    // ============ WsClient Error Handling Tests ============

    #[tokio::test]
    async fn test_ws_client_send_text_not_connected() {
        let client = WsClient::mainnet();
        let result = client.send_text("hello").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::WebSocket(_)));
        assert!(err.to_string().contains("Not connected"));
    }

    #[tokio::test]
    async fn test_ws_client_send_json_not_connected() {
        let client = WsClient::mainnet();
        let result = client.send_json(&serde_json::json!({"test": true})).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::WebSocket(_)));
    }

    #[tokio::test]
    async fn test_ws_client_send_binary_not_connected() {
        let client = WsClient::mainnet();
        let result = client.send_binary(vec![1, 2, 3]).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::WebSocket(_)));
    }

    #[tokio::test]
    async fn test_ws_client_disconnect_when_not_connected() {
        let client = WsClient::mainnet();
        let result = client.disconnect().await;
        assert!(result.is_ok());
        assert_eq!(client.state().await, ConnectionState::Disconnected);
    }

    #[tokio::test]
    async fn test_ws_client_close() {
        let client = WsClient::mainnet();
        let result = client.close().await;
        assert!(result.is_ok());
        assert_eq!(client.state().await, ConnectionState::Closed);
    }

    #[tokio::test]
    async fn test_ws_client_connect_after_close() {
        let client = WsClient::mainnet();
        client.close().await.unwrap();
        let result = client.connect().await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("closed"));
    }

    // ============ WsClient Send/Sync Tests ============

    #[test]
    fn test_ws_client_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<WsClient>();
    }

    #[test]
    fn test_ws_client_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<WsClient>();
    }

    #[test]
    fn test_connection_state_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<ConnectionState>();
        assert_sync::<ConnectionState>();
    }

    #[test]
    fn test_ws_message_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<WsMessage>();
        assert_sync::<WsMessage>();
    }

    // ============ URL Tests ============

    #[test]
    fn test_mainnet_ws_url() {
        assert_eq!(MAINNET_WS_URL, "wss://api.hyperliquid.xyz/ws");
    }

    #[test]
    fn test_testnet_ws_url() {
        assert_eq!(TESTNET_WS_URL, "wss://api.hyperliquid-testnet.xyz/ws");
    }

    // ============ Integration Tests (require network) ============

    // Note: These tests connect to the actual Hyperliquid WebSocket server.
    // They are marked with #[ignore] to avoid running during regular test runs.
    // Run with: cargo test -- --ignored

    #[tokio::test]
    #[ignore]
    async fn test_ws_client_connect_to_mainnet() {
        let client = WsClient::mainnet();
        let result = client.connect().await;
        assert!(result.is_ok());
        assert!(client.is_connected().await);
        client.close().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_ws_client_connect_to_testnet() {
        let client = WsClient::testnet();
        let result = client.connect().await;
        assert!(result.is_ok());
        assert!(client.is_connected().await);
        client.close().await.unwrap();
    }
}
