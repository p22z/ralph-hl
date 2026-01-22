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

use super::subscription::{Subscription, SubscriptionManager, SubscriptionRequest};

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
/// Provides connection management, automatic reconnection, heartbeat handling,
/// and subscription management.
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
    /// Subscription manager
    subscriptions: SubscriptionManager,
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
            subscriptions: SubscriptionManager::new(),
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

    // ========== Subscription Methods ==========

    /// Subscribe to a WebSocket channel
    ///
    /// Sends a subscription request and tracks the subscription state.
    /// The subscription is marked as pending until a confirmation is received.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use hyperliquid::websocket::{WsClient, Subscription};
    ///
    /// let client = WsClient::mainnet();
    /// client.connect().await?;
    /// client.subscribe(Subscription::trades("BTC")).await?;
    /// ```
    pub async fn subscribe(&self, subscription: Subscription) -> Result<()> {
        if !self.is_connected().await {
            return Err(Error::WebSocket("Not connected".to_string()));
        }

        // Check if already subscribed
        if self.subscriptions.contains(&subscription).await {
            return Ok(());
        }

        // Create subscription request
        let request = SubscriptionRequest::subscribe(subscription.clone());

        // Send the request
        self.send_json(&request).await?;

        // Mark as pending
        self.subscriptions.add_pending(subscription).await;

        Ok(())
    }

    /// Unsubscribe from a WebSocket channel
    ///
    /// Sends an unsubscription request and tracks the state.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use hyperliquid::websocket::{WsClient, Subscription};
    ///
    /// let client = WsClient::mainnet();
    /// // ... subscribe first ...
    /// client.unsubscribe(Subscription::trades("BTC")).await?;
    /// ```
    pub async fn unsubscribe(&self, subscription: Subscription) -> Result<()> {
        if !self.is_connected().await {
            return Err(Error::WebSocket("Not connected".to_string()));
        }

        // Check if subscription exists and is active
        if !self.subscriptions.is_active(&subscription).await {
            return Err(Error::WebSocket(
                "Subscription not active or does not exist".to_string(),
            ));
        }

        // Create unsubscription request
        let request = SubscriptionRequest::unsubscribe(subscription.clone());

        // Send the request
        self.send_json(&request).await?;

        // Mark as unsubscribing
        self.subscriptions.mark_unsubscribing(&subscription).await;

        Ok(())
    }

    /// Mark a subscription as active (called when confirmation is received)
    ///
    /// This is typically called internally when processing subscription responses,
    /// but can be used manually if needed.
    pub async fn confirm_subscription(&self, subscription: &Subscription) -> bool {
        self.subscriptions.mark_active(subscription).await
    }

    /// Mark a subscription as removed (called when unsubscribe confirmation is received)
    ///
    /// This is typically called internally when processing subscription responses,
    /// but can be used manually if needed.
    pub async fn confirm_unsubscription(&self, subscription: &Subscription) -> bool {
        self.subscriptions.remove(subscription).await
    }

    /// Get the subscription manager for direct access
    pub fn subscription_manager(&self) -> &SubscriptionManager {
        &self.subscriptions
    }

    /// Check if a subscription is active
    pub async fn is_subscribed(&self, subscription: &Subscription) -> bool {
        self.subscriptions.is_active(subscription).await
    }

    /// Get all active subscriptions
    pub async fn active_subscriptions(&self) -> Vec<Subscription> {
        self.subscriptions.active_subscriptions().await
    }

    /// Get the count of active subscriptions
    pub async fn subscription_count(&self) -> usize {
        self.subscriptions.active_count().await
    }

    /// Resubscribe to all active subscriptions
    ///
    /// This is useful after a reconnection to restore all subscriptions.
    /// Returns the number of subscriptions that were resent.
    pub async fn resubscribe_all(&self) -> Result<usize> {
        if !self.is_connected().await {
            return Err(Error::WebSocket("Not connected".to_string()));
        }

        let active = self.subscriptions.active_subscriptions().await;
        let count = active.len();

        for subscription in active {
            let request = SubscriptionRequest::subscribe(subscription);
            self.send_json(&request).await?;
        }

        Ok(count)
    }

    /// Unsubscribe from all active subscriptions
    pub async fn unsubscribe_all(&self) -> Result<usize> {
        if !self.is_connected().await {
            return Err(Error::WebSocket("Not connected".to_string()));
        }

        let active = self.subscriptions.active_subscriptions().await;
        let count = active.len();

        for subscription in active {
            let request = SubscriptionRequest::unsubscribe(subscription.clone());
            self.send_json(&request).await?;
            self.subscriptions.mark_unsubscribing(&subscription).await;
        }

        Ok(count)
    }

    /// Clear all subscription tracking (without sending unsubscribe requests)
    ///
    /// This is useful when you know the connection has been lost and
    /// subscriptions are no longer valid on the server.
    pub async fn clear_subscriptions(&self) {
        self.subscriptions.clear().await;
    }

    // ========== Market Data Subscription Methods ==========

    /// Subscribe to all mid prices
    ///
    /// Subscribes to real-time mid price updates for all trading pairs.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use hyperliquid::websocket::WsClient;
    ///
    /// let client = WsClient::mainnet();
    /// client.connect().await?;
    /// client.subscribe_all_mids().await?;
    /// ```
    pub async fn subscribe_all_mids(&self) -> Result<()> {
        self.subscribe(Subscription::all_mids()).await
    }

    /// Subscribe to all mid prices for a specific DEX
    ///
    /// # Arguments
    ///
    /// * `dex` - The DEX identifier (e.g., "perp", "spot")
    ///
    /// # Example
    ///
    /// ```ignore
    /// use hyperliquid::websocket::WsClient;
    ///
    /// let client = WsClient::mainnet();
    /// client.connect().await?;
    /// client.subscribe_all_mids_with_dex("perp").await?;
    /// ```
    pub async fn subscribe_all_mids_with_dex(&self, dex: impl Into<String>) -> Result<()> {
        self.subscribe(Subscription::all_mids_with_dex(dex)).await
    }

    /// Subscribe to L2 order book updates for a coin
    ///
    /// Receives real-time order book snapshots with full depth.
    ///
    /// # Arguments
    ///
    /// * `coin` - The coin symbol (e.g., "BTC", "ETH")
    ///
    /// # Example
    ///
    /// ```ignore
    /// use hyperliquid::websocket::WsClient;
    ///
    /// let client = WsClient::mainnet();
    /// client.connect().await?;
    /// client.subscribe_l2_book("BTC").await?;
    /// ```
    pub async fn subscribe_l2_book(&self, coin: impl Into<String>) -> Result<()> {
        self.subscribe(Subscription::l2_book(coin)).await
    }

    /// Subscribe to L2 order book updates with aggregation parameters
    ///
    /// # Arguments
    ///
    /// * `coin` - The coin symbol
    /// * `n_sig_figs` - Number of significant figures for price aggregation
    /// * `mantissa` - Mantissa for price aggregation
    ///
    /// # Example
    ///
    /// ```ignore
    /// use hyperliquid::websocket::WsClient;
    ///
    /// let client = WsClient::mainnet();
    /// client.connect().await?;
    /// client.subscribe_l2_book_with_params("BTC", Some(5), Some(2)).await?;
    /// ```
    pub async fn subscribe_l2_book_with_params(
        &self,
        coin: impl Into<String>,
        n_sig_figs: Option<u8>,
        mantissa: Option<u8>,
    ) -> Result<()> {
        self.subscribe(Subscription::l2_book_with_params(coin, n_sig_figs, mantissa))
            .await
    }

    /// Subscribe to trade stream for a coin
    ///
    /// Receives real-time trade updates as they occur.
    ///
    /// # Arguments
    ///
    /// * `coin` - The coin symbol (e.g., "BTC", "ETH")
    ///
    /// # Example
    ///
    /// ```ignore
    /// use hyperliquid::websocket::WsClient;
    ///
    /// let client = WsClient::mainnet();
    /// client.connect().await?;
    /// client.subscribe_trades("BTC").await?;
    /// ```
    pub async fn subscribe_trades(&self, coin: impl Into<String>) -> Result<()> {
        self.subscribe(Subscription::trades(coin)).await
    }

    /// Subscribe to candlestick/OHLCV updates for a coin
    ///
    /// Receives real-time candle updates for the specified interval.
    ///
    /// # Arguments
    ///
    /// * `coin` - The coin symbol (e.g., "BTC", "ETH")
    /// * `interval` - The candle interval (e.g., OneMinute, FifteenMinutes, OneHour)
    ///
    /// # Example
    ///
    /// ```ignore
    /// use hyperliquid::websocket::WsClient;
    /// use hyperliquid::types::CandleInterval;
    ///
    /// let client = WsClient::mainnet();
    /// client.connect().await?;
    /// client.subscribe_candle("BTC", CandleInterval::OneHour).await?;
    /// ```
    pub async fn subscribe_candle(
        &self,
        coin: impl Into<String>,
        interval: crate::types::CandleInterval,
    ) -> Result<()> {
        self.subscribe(Subscription::candle(coin, interval)).await
    }

    /// Subscribe to best bid/offer updates for a coin
    ///
    /// Receives real-time BBO (best bid and offer) updates.
    ///
    /// # Arguments
    ///
    /// * `coin` - The coin symbol (e.g., "BTC", "ETH")
    ///
    /// # Example
    ///
    /// ```ignore
    /// use hyperliquid::websocket::WsClient;
    ///
    /// let client = WsClient::mainnet();
    /// client.connect().await?;
    /// client.subscribe_bbo("BTC").await?;
    /// ```
    pub async fn subscribe_bbo(&self, coin: impl Into<String>) -> Result<()> {
        self.subscribe(Subscription::bbo(coin)).await
    }

    /// Subscribe to active asset context updates for a coin
    ///
    /// Receives real-time asset context updates including funding rates,
    /// open interest, and other market metrics.
    ///
    /// # Arguments
    ///
    /// * `coin` - The coin symbol (e.g., "BTC", "ETH")
    ///
    /// # Example
    ///
    /// ```ignore
    /// use hyperliquid::websocket::WsClient;
    ///
    /// let client = WsClient::mainnet();
    /// client.connect().await?;
    /// client.subscribe_active_asset_ctx("BTC").await?;
    /// ```
    pub async fn subscribe_active_asset_ctx(&self, coin: impl Into<String>) -> Result<()> {
        self.subscribe(Subscription::active_asset_ctx(coin)).await
    }

    // ========== User Data Subscription Methods ==========

    /// Subscribe to user notifications
    ///
    /// Receives real-time notifications for the specified user address.
    ///
    /// # Arguments
    ///
    /// * `user` - The user's Ethereum address (e.g., "0x...")
    ///
    /// # Example
    ///
    /// ```ignore
    /// use hyperliquid::websocket::WsClient;
    ///
    /// let client = WsClient::mainnet();
    /// client.connect().await?;
    /// client.subscribe_notification("0x1234567890123456789012345678901234567890").await?;
    /// ```
    pub async fn subscribe_notification(&self, user: impl Into<String>) -> Result<()> {
        self.subscribe(Subscription::notification(user)).await
    }

    /// Subscribe to aggregate user data (webData3)
    ///
    /// Receives comprehensive real-time user data including positions, orders,
    /// balances, and other account information in a single stream.
    ///
    /// # Arguments
    ///
    /// * `user` - The user's Ethereum address (e.g., "0x...")
    ///
    /// # Example
    ///
    /// ```ignore
    /// use hyperliquid::websocket::WsClient;
    ///
    /// let client = WsClient::mainnet();
    /// client.connect().await?;
    /// client.subscribe_web_data3("0x1234567890123456789012345678901234567890").await?;
    /// ```
    pub async fn subscribe_web_data3(&self, user: impl Into<String>) -> Result<()> {
        self.subscribe(Subscription::web_data3(user)).await
    }

    /// Subscribe to TWAP order states
    ///
    /// Receives real-time TWAP (Time-Weighted Average Price) order state updates.
    ///
    /// # Arguments
    ///
    /// * `user` - The user's Ethereum address (e.g., "0x...")
    ///
    /// # Example
    ///
    /// ```ignore
    /// use hyperliquid::websocket::WsClient;
    ///
    /// let client = WsClient::mainnet();
    /// client.connect().await?;
    /// client.subscribe_twap_states("0x1234567890123456789012345678901234567890").await?;
    /// ```
    pub async fn subscribe_twap_states(&self, user: impl Into<String>) -> Result<()> {
        self.subscribe(Subscription::twap_states(user)).await
    }

    /// Subscribe to TWAP order states for a specific DEX
    ///
    /// # Arguments
    ///
    /// * `user` - The user's Ethereum address (e.g., "0x...")
    /// * `dex` - The DEX identifier (e.g., "perp", "spot")
    ///
    /// # Example
    ///
    /// ```ignore
    /// use hyperliquid::websocket::WsClient;
    ///
    /// let client = WsClient::mainnet();
    /// client.connect().await?;
    /// client.subscribe_twap_states_with_dex("0x123...", "perp").await?;
    /// ```
    pub async fn subscribe_twap_states_with_dex(
        &self,
        user: impl Into<String>,
        dex: impl Into<String>,
    ) -> Result<()> {
        self.subscribe(Subscription::twap_states_with_dex(user, dex))
            .await
    }

    /// Subscribe to clearinghouse state updates
    ///
    /// Receives real-time updates to the user's account state including positions,
    /// margins, and balances.
    ///
    /// # Arguments
    ///
    /// * `user` - The user's Ethereum address (e.g., "0x...")
    ///
    /// # Example
    ///
    /// ```ignore
    /// use hyperliquid::websocket::WsClient;
    ///
    /// let client = WsClient::mainnet();
    /// client.connect().await?;
    /// client.subscribe_clearinghouse_state("0x1234567890123456789012345678901234567890").await?;
    /// ```
    pub async fn subscribe_clearinghouse_state(&self, user: impl Into<String>) -> Result<()> {
        self.subscribe(Subscription::clearinghouse_state(user)).await
    }

    /// Subscribe to clearinghouse state updates for a specific DEX
    ///
    /// # Arguments
    ///
    /// * `user` - The user's Ethereum address (e.g., "0x...")
    /// * `dex` - The DEX identifier (e.g., "perp", "spot")
    ///
    /// # Example
    ///
    /// ```ignore
    /// use hyperliquid::websocket::WsClient;
    ///
    /// let client = WsClient::mainnet();
    /// client.connect().await?;
    /// client.subscribe_clearinghouse_state_with_dex("0x123...", "perp").await?;
    /// ```
    pub async fn subscribe_clearinghouse_state_with_dex(
        &self,
        user: impl Into<String>,
        dex: impl Into<String>,
    ) -> Result<()> {
        self.subscribe(Subscription::clearinghouse_state_with_dex(user, dex))
            .await
    }

    /// Subscribe to open orders updates
    ///
    /// Receives real-time updates when orders are placed, modified, or removed.
    ///
    /// # Arguments
    ///
    /// * `user` - The user's Ethereum address (e.g., "0x...")
    ///
    /// # Example
    ///
    /// ```ignore
    /// use hyperliquid::websocket::WsClient;
    ///
    /// let client = WsClient::mainnet();
    /// client.connect().await?;
    /// client.subscribe_open_orders("0x1234567890123456789012345678901234567890").await?;
    /// ```
    pub async fn subscribe_open_orders(&self, user: impl Into<String>) -> Result<()> {
        self.subscribe(Subscription::open_orders(user)).await
    }

    /// Subscribe to open orders updates for a specific DEX
    ///
    /// # Arguments
    ///
    /// * `user` - The user's Ethereum address (e.g., "0x...")
    /// * `dex` - The DEX identifier (e.g., "perp", "spot")
    ///
    /// # Example
    ///
    /// ```ignore
    /// use hyperliquid::websocket::WsClient;
    ///
    /// let client = WsClient::mainnet();
    /// client.connect().await?;
    /// client.subscribe_open_orders_with_dex("0x123...", "perp").await?;
    /// ```
    pub async fn subscribe_open_orders_with_dex(
        &self,
        user: impl Into<String>,
        dex: impl Into<String>,
    ) -> Result<()> {
        self.subscribe(Subscription::open_orders_with_dex(user, dex))
            .await
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
            subscriptions: SubscriptionManager::new(),
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

    // ============ Subscription Tests (without network) ============

    #[tokio::test]
    async fn test_ws_client_subscribe_not_connected() {
        let client = WsClient::mainnet();
        let result = client.subscribe(Subscription::trades("BTC")).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Not connected"));
    }

    #[tokio::test]
    async fn test_ws_client_unsubscribe_not_connected() {
        let client = WsClient::mainnet();
        let result = client.unsubscribe(Subscription::trades("BTC")).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Not connected"));
    }

    #[tokio::test]
    async fn test_ws_client_subscription_manager_access() {
        let client = WsClient::mainnet();
        let manager = client.subscription_manager();
        assert_eq!(manager.total_count().await, 0);
    }

    #[tokio::test]
    async fn test_ws_client_initial_subscription_count() {
        let client = WsClient::mainnet();
        assert_eq!(client.subscription_count().await, 0);
        assert!(client.active_subscriptions().await.is_empty());
    }

    #[tokio::test]
    async fn test_ws_client_is_subscribed_false_initially() {
        let client = WsClient::mainnet();
        let sub = Subscription::trades("BTC");
        assert!(!client.is_subscribed(&sub).await);
    }

    #[tokio::test]
    async fn test_ws_client_clear_subscriptions() {
        let client = WsClient::mainnet();
        // Add a subscription manually via the manager
        client
            .subscription_manager()
            .add_pending(Subscription::trades("BTC"))
            .await;
        client
            .subscription_manager()
            .mark_active(&Subscription::trades("BTC"))
            .await;

        assert_eq!(client.subscription_count().await, 1);

        client.clear_subscriptions().await;
        assert_eq!(client.subscription_count().await, 0);
    }

    #[tokio::test]
    async fn test_ws_client_confirm_subscription() {
        let client = WsClient::mainnet();
        let sub = Subscription::trades("BTC");

        // Add as pending via manager
        client.subscription_manager().add_pending(sub.clone()).await;

        // Confirm it
        let result = client.confirm_subscription(&sub).await;
        assert!(result);
        assert!(client.is_subscribed(&sub).await);
    }

    #[tokio::test]
    async fn test_ws_client_confirm_unsubscription() {
        let client = WsClient::mainnet();
        let sub = Subscription::trades("BTC");

        // Add and activate via manager
        client.subscription_manager().add_pending(sub.clone()).await;
        client.subscription_manager().mark_active(&sub).await;

        // Confirm unsubscription
        let result = client.confirm_unsubscription(&sub).await;
        assert!(result);
        assert!(!client.is_subscribed(&sub).await);
    }

    #[tokio::test]
    async fn test_ws_client_resubscribe_all_not_connected() {
        let client = WsClient::mainnet();
        let result = client.resubscribe_all().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_ws_client_unsubscribe_all_not_connected() {
        let client = WsClient::mainnet();
        let result = client.unsubscribe_all().await;
        assert!(result.is_err());
    }

    // ============ Market Data Subscription Methods Tests (without network) ============

    #[tokio::test]
    async fn test_subscribe_all_mids_not_connected() {
        let client = WsClient::mainnet();
        let result = client.subscribe_all_mids().await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Not connected"));
    }

    #[tokio::test]
    async fn test_subscribe_all_mids_with_dex_not_connected() {
        let client = WsClient::mainnet();
        let result = client.subscribe_all_mids_with_dex("perp").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Not connected"));
    }

    #[tokio::test]
    async fn test_subscribe_l2_book_not_connected() {
        let client = WsClient::mainnet();
        let result = client.subscribe_l2_book("BTC").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Not connected"));
    }

    #[tokio::test]
    async fn test_subscribe_l2_book_with_params_not_connected() {
        let client = WsClient::mainnet();
        let result = client.subscribe_l2_book_with_params("BTC", Some(5), Some(2)).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Not connected"));
    }

    #[tokio::test]
    async fn test_subscribe_trades_not_connected() {
        let client = WsClient::mainnet();
        let result = client.subscribe_trades("BTC").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Not connected"));
    }

    #[tokio::test]
    async fn test_subscribe_candle_not_connected() {
        use crate::types::CandleInterval;
        let client = WsClient::mainnet();
        let result = client.subscribe_candle("BTC", CandleInterval::OneHour).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Not connected"));
    }

    #[tokio::test]
    async fn test_subscribe_bbo_not_connected() {
        let client = WsClient::mainnet();
        let result = client.subscribe_bbo("BTC").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Not connected"));
    }

    #[tokio::test]
    async fn test_subscribe_active_asset_ctx_not_connected() {
        let client = WsClient::mainnet();
        let result = client.subscribe_active_asset_ctx("BTC").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Not connected"));
    }

    // ============ Market Data Subscription Methods - Subscription Creation Tests ============

    #[test]
    fn test_subscription_all_mids_creates_correct_subscription() {
        let sub = Subscription::all_mids();
        assert_eq!(sub.channel_name(), "allMids");
        assert!(sub.is_market_subscription());
    }

    #[test]
    fn test_subscription_all_mids_with_dex_creates_correct_subscription() {
        let sub = Subscription::all_mids_with_dex("perp");
        assert_eq!(sub.channel_name(), "allMids");
        if let Subscription::AllMids { dex } = sub {
            assert_eq!(dex, Some("perp".to_string()));
        } else {
            panic!("Expected AllMids subscription");
        }
    }

    #[test]
    fn test_subscription_l2_book_creates_correct_subscription() {
        let sub = Subscription::l2_book("ETH");
        assert_eq!(sub.channel_name(), "l2Book");
        assert_eq!(sub.coin(), Some("ETH"));
    }

    #[test]
    fn test_subscription_l2_book_with_params_creates_correct_subscription() {
        let sub = Subscription::l2_book_with_params("SOL", Some(4), Some(1));
        assert_eq!(sub.channel_name(), "l2Book");
        if let Subscription::L2Book { coin, n_sig_figs, mantissa } = sub {
            assert_eq!(coin, "SOL");
            assert_eq!(n_sig_figs, Some(4));
            assert_eq!(mantissa, Some(1));
        } else {
            panic!("Expected L2Book subscription");
        }
    }

    #[test]
    fn test_subscription_trades_creates_correct_subscription() {
        let sub = Subscription::trades("DOGE");
        assert_eq!(sub.channel_name(), "trades");
        assert_eq!(sub.coin(), Some("DOGE"));
    }

    #[test]
    fn test_subscription_candle_creates_correct_subscription() {
        use crate::types::CandleInterval;
        let sub = Subscription::candle("BTC", CandleInterval::FifteenMinutes);
        assert_eq!(sub.channel_name(), "candle");
        assert_eq!(sub.coin(), Some("BTC"));
        if let Subscription::Candle { interval, .. } = sub {
            assert_eq!(interval, CandleInterval::FifteenMinutes);
        } else {
            panic!("Expected Candle subscription");
        }
    }

    #[test]
    fn test_subscription_bbo_creates_correct_subscription() {
        let sub = Subscription::bbo("AVAX");
        assert_eq!(sub.channel_name(), "bbo");
        assert_eq!(sub.coin(), Some("AVAX"));
    }

    #[test]
    fn test_subscription_active_asset_ctx_creates_correct_subscription() {
        let sub = Subscription::active_asset_ctx("MATIC");
        assert_eq!(sub.channel_name(), "activeAssetCtx");
        assert_eq!(sub.coin(), Some("MATIC"));
    }

    // ============ Market Data Subscription JSON Serialization Tests ============

    #[test]
    fn test_subscribe_all_mids_serializes_correctly() {
        let request = SubscriptionRequest::subscribe(Subscription::all_mids());
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"method\":\"subscribe\""));
        assert!(json.contains("\"type\":\"allMids\""));
    }

    #[test]
    fn test_subscribe_all_mids_with_dex_serializes_correctly() {
        let request = SubscriptionRequest::subscribe(Subscription::all_mids_with_dex("spot"));
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"method\":\"subscribe\""));
        assert!(json.contains("\"type\":\"allMids\""));
        assert!(json.contains("\"dex\":\"spot\""));
    }

    #[test]
    fn test_subscribe_l2_book_serializes_correctly() {
        let request = SubscriptionRequest::subscribe(Subscription::l2_book("BTC"));
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"method\":\"subscribe\""));
        assert!(json.contains("\"type\":\"l2Book\""));
        assert!(json.contains("\"coin\":\"BTC\""));
    }

    #[test]
    fn test_subscribe_l2_book_with_params_serializes_correctly() {
        let request = SubscriptionRequest::subscribe(
            Subscription::l2_book_with_params("ETH", Some(5), Some(2))
        );
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"type\":\"l2Book\""));
        assert!(json.contains("\"coin\":\"ETH\""));
        assert!(json.contains("\"nSigFigs\":5"));
        assert!(json.contains("\"mantissa\":2"));
    }

    #[test]
    fn test_subscribe_trades_serializes_correctly() {
        let request = SubscriptionRequest::subscribe(Subscription::trades("SOL"));
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"method\":\"subscribe\""));
        assert!(json.contains("\"type\":\"trades\""));
        assert!(json.contains("\"coin\":\"SOL\""));
    }

    #[test]
    fn test_subscribe_candle_serializes_correctly() {
        use crate::types::CandleInterval;
        let request = SubscriptionRequest::subscribe(
            Subscription::candle("BTC", CandleInterval::OneHour)
        );
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"method\":\"subscribe\""));
        assert!(json.contains("\"type\":\"candle\""));
        assert!(json.contains("\"coin\":\"BTC\""));
        assert!(json.contains("\"interval\":\"1h\""));
    }

    #[test]
    fn test_subscribe_bbo_serializes_correctly() {
        let request = SubscriptionRequest::subscribe(Subscription::bbo("LINK"));
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"method\":\"subscribe\""));
        assert!(json.contains("\"type\":\"bbo\""));
        assert!(json.contains("\"coin\":\"LINK\""));
    }

    #[test]
    fn test_subscribe_active_asset_ctx_serializes_correctly() {
        let request = SubscriptionRequest::subscribe(Subscription::active_asset_ctx("ARB"));
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"method\":\"subscribe\""));
        assert!(json.contains("\"type\":\"activeAssetCtx\""));
        assert!(json.contains("\"coin\":\"ARB\""));
    }

    // ============ Market Data Subscription is_market_subscription Tests ============

    #[test]
    fn test_all_market_subscriptions_are_market() {
        use crate::types::CandleInterval;

        let subscriptions = vec![
            Subscription::all_mids(),
            Subscription::all_mids_with_dex("perp"),
            Subscription::l2_book("BTC"),
            Subscription::l2_book_with_params("BTC", Some(5), None),
            Subscription::trades("BTC"),
            Subscription::candle("BTC", CandleInterval::OneMinute),
            Subscription::bbo("BTC"),
            Subscription::active_asset_ctx("BTC"),
        ];

        for sub in subscriptions {
            assert!(sub.is_market_subscription(), "Expected {:?} to be a market subscription", sub);
            assert!(!sub.is_user_subscription(), "Expected {:?} to NOT be a user subscription", sub);
        }
    }

    // ============ Subscription Integration Tests (require network) ============

    #[tokio::test]
    #[ignore]
    async fn test_ws_client_subscribe_trades() {
        let client = WsClient::mainnet();
        client.connect().await.unwrap();

        let sub = Subscription::trades("BTC");
        let result = client.subscribe(sub.clone()).await;
        assert!(result.is_ok());

        // Subscription should be pending (waiting for confirmation)
        let status = client.subscription_manager().status(&sub).await;
        assert!(status.is_some());

        client.close().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_ws_client_subscribe_l2_book() {
        let client = WsClient::mainnet();
        client.connect().await.unwrap();

        let sub = Subscription::l2_book("ETH");
        let result = client.subscribe(sub.clone()).await;
        assert!(result.is_ok());

        client.close().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_ws_client_subscribe_all_mids() {
        let client = WsClient::mainnet();
        client.connect().await.unwrap();

        let sub = Subscription::all_mids();
        let result = client.subscribe(sub.clone()).await;
        assert!(result.is_ok());

        client.close().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_ws_client_subscribe_duplicate() {
        let client = WsClient::mainnet();
        client.connect().await.unwrap();

        let sub = Subscription::trades("BTC");

        // Subscribe first time
        client.subscribe(sub.clone()).await.unwrap();
        // Subscribe again - should succeed (idempotent)
        let result = client.subscribe(sub.clone()).await;
        assert!(result.is_ok());

        client.close().await.unwrap();
    }

    // ============ Market Data Subscription Integration Tests (require network) ============

    #[tokio::test]
    #[ignore]
    async fn test_subscribe_all_mids_integration() {
        let client = WsClient::mainnet();
        client.connect().await.unwrap();

        let result = client.subscribe_all_mids().await;
        assert!(result.is_ok());

        // Check subscription is tracked
        let sub = Subscription::all_mids();
        assert!(client.subscription_manager().contains(&sub).await);

        client.close().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_subscribe_all_mids_with_dex_integration() {
        let client = WsClient::mainnet();
        client.connect().await.unwrap();

        let result = client.subscribe_all_mids_with_dex("perp").await;
        assert!(result.is_ok());

        client.close().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_subscribe_l2_book_integration() {
        let client = WsClient::mainnet();
        client.connect().await.unwrap();

        let result = client.subscribe_l2_book("BTC").await;
        assert!(result.is_ok());

        // Check subscription is tracked
        let sub = Subscription::l2_book("BTC");
        assert!(client.subscription_manager().contains(&sub).await);

        client.close().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_subscribe_l2_book_with_params_integration() {
        let client = WsClient::mainnet();
        client.connect().await.unwrap();

        let result = client.subscribe_l2_book_with_params("ETH", Some(5), Some(2)).await;
        assert!(result.is_ok());

        client.close().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_subscribe_trades_integration() {
        let client = WsClient::mainnet();
        client.connect().await.unwrap();

        let result = client.subscribe_trades("BTC").await;
        assert!(result.is_ok());

        // Check subscription is tracked
        let sub = Subscription::trades("BTC");
        assert!(client.subscription_manager().contains(&sub).await);

        client.close().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_subscribe_candle_integration() {
        use crate::types::CandleInterval;

        let client = WsClient::mainnet();
        client.connect().await.unwrap();

        let result = client.subscribe_candle("BTC", CandleInterval::OneHour).await;
        assert!(result.is_ok());

        // Check subscription is tracked
        let sub = Subscription::candle("BTC", CandleInterval::OneHour);
        assert!(client.subscription_manager().contains(&sub).await);

        client.close().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_subscribe_bbo_integration() {
        let client = WsClient::mainnet();
        client.connect().await.unwrap();

        let result = client.subscribe_bbo("BTC").await;
        assert!(result.is_ok());

        // Check subscription is tracked
        let sub = Subscription::bbo("BTC");
        assert!(client.subscription_manager().contains(&sub).await);

        client.close().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_subscribe_active_asset_ctx_integration() {
        let client = WsClient::mainnet();
        client.connect().await.unwrap();

        let result = client.subscribe_active_asset_ctx("BTC").await;
        assert!(result.is_ok());

        // Check subscription is tracked
        let sub = Subscription::active_asset_ctx("BTC");
        assert!(client.subscription_manager().contains(&sub).await);

        client.close().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_subscribe_multiple_market_data_integration() {
        use crate::types::CandleInterval;

        let client = WsClient::mainnet();
        client.connect().await.unwrap();

        // Subscribe to multiple market data streams
        client.subscribe_all_mids().await.unwrap();
        client.subscribe_l2_book("BTC").await.unwrap();
        client.subscribe_trades("ETH").await.unwrap();
        client.subscribe_candle("SOL", CandleInterval::FifteenMinutes).await.unwrap();
        client.subscribe_bbo("AVAX").await.unwrap();

        // Verify all subscriptions are tracked
        assert!(client.subscription_manager().total_count().await >= 5);

        client.close().await.unwrap();
    }

    // ============ User Data Subscription Methods Tests (without network) ============

    #[tokio::test]
    async fn test_subscribe_notification_not_connected() {
        let client = WsClient::mainnet();
        let result = client.subscribe_notification("0x1234567890123456789012345678901234567890").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Not connected"));
    }

    #[tokio::test]
    async fn test_subscribe_web_data3_not_connected() {
        let client = WsClient::mainnet();
        let result = client.subscribe_web_data3("0x1234567890123456789012345678901234567890").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Not connected"));
    }

    #[tokio::test]
    async fn test_subscribe_twap_states_not_connected() {
        let client = WsClient::mainnet();
        let result = client.subscribe_twap_states("0x1234567890123456789012345678901234567890").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Not connected"));
    }

    #[tokio::test]
    async fn test_subscribe_twap_states_with_dex_not_connected() {
        let client = WsClient::mainnet();
        let result = client.subscribe_twap_states_with_dex("0x123...", "perp").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Not connected"));
    }

    #[tokio::test]
    async fn test_subscribe_clearinghouse_state_not_connected() {
        let client = WsClient::mainnet();
        let result = client.subscribe_clearinghouse_state("0x1234567890123456789012345678901234567890").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Not connected"));
    }

    #[tokio::test]
    async fn test_subscribe_clearinghouse_state_with_dex_not_connected() {
        let client = WsClient::mainnet();
        let result = client.subscribe_clearinghouse_state_with_dex("0x123...", "perp").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Not connected"));
    }

    #[tokio::test]
    async fn test_subscribe_open_orders_not_connected() {
        let client = WsClient::mainnet();
        let result = client.subscribe_open_orders("0x1234567890123456789012345678901234567890").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Not connected"));
    }

    #[tokio::test]
    async fn test_subscribe_open_orders_with_dex_not_connected() {
        let client = WsClient::mainnet();
        let result = client.subscribe_open_orders_with_dex("0x123...", "spot").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Not connected"));
    }

    // ============ User Data Subscription Methods - Subscription Creation Tests ============

    #[test]
    fn test_subscription_notification_creates_correct_subscription() {
        let sub = Subscription::notification("0x1234");
        assert_eq!(sub.channel_name(), "notification");
        assert_eq!(sub.user(), Some("0x1234"));
        assert!(sub.is_user_subscription());
        assert!(!sub.is_market_subscription());
    }

    #[test]
    fn test_subscription_web_data3_creates_correct_subscription() {
        let sub = Subscription::web_data3("0x5678");
        assert_eq!(sub.channel_name(), "webData3");
        assert_eq!(sub.user(), Some("0x5678"));
        assert!(sub.is_user_subscription());
    }

    #[test]
    fn test_subscription_twap_states_creates_correct_subscription() {
        let sub = Subscription::twap_states("0xabcd");
        assert_eq!(sub.channel_name(), "twapStates");
        assert_eq!(sub.user(), Some("0xabcd"));
        assert!(sub.is_user_subscription());
    }

    #[test]
    fn test_subscription_twap_states_with_dex_creates_correct_subscription() {
        let sub = Subscription::twap_states_with_dex("0xabcd", "perp");
        assert_eq!(sub.channel_name(), "twapStates");
        assert_eq!(sub.user(), Some("0xabcd"));
        if let Subscription::TwapStates { user, dex } = sub {
            assert_eq!(user, "0xabcd");
            assert_eq!(dex, Some("perp".to_string()));
        } else {
            panic!("Expected TwapStates subscription");
        }
    }

    #[test]
    fn test_subscription_clearinghouse_state_creates_correct_subscription() {
        let sub = Subscription::clearinghouse_state("0xefgh");
        assert_eq!(sub.channel_name(), "clearinghouseState");
        assert_eq!(sub.user(), Some("0xefgh"));
        assert!(sub.is_user_subscription());
    }

    #[test]
    fn test_subscription_clearinghouse_state_with_dex_creates_correct_subscription() {
        let sub = Subscription::clearinghouse_state_with_dex("0xefgh", "spot");
        assert_eq!(sub.channel_name(), "clearinghouseState");
        if let Subscription::ClearinghouseState { user, dex } = sub {
            assert_eq!(user, "0xefgh");
            assert_eq!(dex, Some("spot".to_string()));
        } else {
            panic!("Expected ClearinghouseState subscription");
        }
    }

    #[test]
    fn test_subscription_open_orders_creates_correct_subscription() {
        let sub = Subscription::open_orders("0xijkl");
        assert_eq!(sub.channel_name(), "openOrders");
        assert_eq!(sub.user(), Some("0xijkl"));
        assert!(sub.is_user_subscription());
    }

    #[test]
    fn test_subscription_open_orders_with_dex_creates_correct_subscription() {
        let sub = Subscription::open_orders_with_dex("0xijkl", "perp");
        assert_eq!(sub.channel_name(), "openOrders");
        if let Subscription::OpenOrders { user, dex } = sub {
            assert_eq!(user, "0xijkl");
            assert_eq!(dex, Some("perp".to_string()));
        } else {
            panic!("Expected OpenOrders subscription");
        }
    }

    // ============ User Data Subscription JSON Serialization Tests ============

    #[test]
    fn test_subscribe_notification_serializes_correctly() {
        let request = SubscriptionRequest::subscribe(Subscription::notification("0x1234"));
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"method\":\"subscribe\""));
        assert!(json.contains("\"type\":\"notification\""));
        assert!(json.contains("\"user\":\"0x1234\""));
    }

    #[test]
    fn test_subscribe_web_data3_serializes_correctly() {
        let request = SubscriptionRequest::subscribe(Subscription::web_data3("0x5678"));
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"method\":\"subscribe\""));
        assert!(json.contains("\"type\":\"webData3\""));
        assert!(json.contains("\"user\":\"0x5678\""));
    }

    #[test]
    fn test_subscribe_twap_states_serializes_correctly() {
        let request = SubscriptionRequest::subscribe(Subscription::twap_states("0xabcd"));
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"method\":\"subscribe\""));
        assert!(json.contains("\"type\":\"twapStates\""));
        assert!(json.contains("\"user\":\"0xabcd\""));
    }

    #[test]
    fn test_subscribe_twap_states_with_dex_serializes_correctly() {
        let request = SubscriptionRequest::subscribe(
            Subscription::twap_states_with_dex("0xabcd", "perp")
        );
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"type\":\"twapStates\""));
        assert!(json.contains("\"user\":\"0xabcd\""));
        assert!(json.contains("\"dex\":\"perp\""));
    }

    #[test]
    fn test_subscribe_clearinghouse_state_serializes_correctly() {
        let request = SubscriptionRequest::subscribe(
            Subscription::clearinghouse_state("0xefgh")
        );
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"method\":\"subscribe\""));
        assert!(json.contains("\"type\":\"clearinghouseState\""));
        assert!(json.contains("\"user\":\"0xefgh\""));
    }

    #[test]
    fn test_subscribe_clearinghouse_state_with_dex_serializes_correctly() {
        let request = SubscriptionRequest::subscribe(
            Subscription::clearinghouse_state_with_dex("0xefgh", "spot")
        );
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"type\":\"clearinghouseState\""));
        assert!(json.contains("\"user\":\"0xefgh\""));
        assert!(json.contains("\"dex\":\"spot\""));
    }

    #[test]
    fn test_subscribe_open_orders_serializes_correctly() {
        let request = SubscriptionRequest::subscribe(Subscription::open_orders("0xijkl"));
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"method\":\"subscribe\""));
        assert!(json.contains("\"type\":\"openOrders\""));
        assert!(json.contains("\"user\":\"0xijkl\""));
    }

    #[test]
    fn test_subscribe_open_orders_with_dex_serializes_correctly() {
        let request = SubscriptionRequest::subscribe(
            Subscription::open_orders_with_dex("0xijkl", "perp")
        );
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"type\":\"openOrders\""));
        assert!(json.contains("\"user\":\"0xijkl\""));
        assert!(json.contains("\"dex\":\"perp\""));
    }

    // ============ User Data Subscription is_user_subscription Tests ============

    #[test]
    fn test_all_user_data_subscriptions_are_user() {
        let subscriptions = vec![
            Subscription::notification("0x1234"),
            Subscription::web_data3("0x1234"),
            Subscription::twap_states("0x1234"),
            Subscription::twap_states_with_dex("0x1234", "perp"),
            Subscription::clearinghouse_state("0x1234"),
            Subscription::clearinghouse_state_with_dex("0x1234", "spot"),
            Subscription::open_orders("0x1234"),
            Subscription::open_orders_with_dex("0x1234", "perp"),
        ];

        for sub in subscriptions {
            assert!(sub.is_user_subscription(), "Expected {:?} to be a user subscription", sub);
            assert!(!sub.is_market_subscription(), "Expected {:?} to NOT be a market subscription", sub);
        }
    }

    // ============ User Data Subscription Integration Tests (require network) ============

    #[tokio::test]
    #[ignore]
    async fn test_subscribe_notification_integration() {
        let client = WsClient::mainnet();
        client.connect().await.unwrap();

        let result = client.subscribe_notification("0x1234567890123456789012345678901234567890").await;
        assert!(result.is_ok());

        // Check subscription is tracked
        let sub = Subscription::notification("0x1234567890123456789012345678901234567890");
        assert!(client.subscription_manager().contains(&sub).await);

        client.close().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_subscribe_web_data3_integration() {
        let client = WsClient::mainnet();
        client.connect().await.unwrap();

        let result = client.subscribe_web_data3("0x1234567890123456789012345678901234567890").await;
        assert!(result.is_ok());

        // Check subscription is tracked
        let sub = Subscription::web_data3("0x1234567890123456789012345678901234567890");
        assert!(client.subscription_manager().contains(&sub).await);

        client.close().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_subscribe_twap_states_integration() {
        let client = WsClient::mainnet();
        client.connect().await.unwrap();

        let result = client.subscribe_twap_states("0x1234567890123456789012345678901234567890").await;
        assert!(result.is_ok());

        // Check subscription is tracked
        let sub = Subscription::twap_states("0x1234567890123456789012345678901234567890");
        assert!(client.subscription_manager().contains(&sub).await);

        client.close().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_subscribe_twap_states_with_dex_integration() {
        let client = WsClient::mainnet();
        client.connect().await.unwrap();

        let result = client.subscribe_twap_states_with_dex(
            "0x1234567890123456789012345678901234567890",
            "perp"
        ).await;
        assert!(result.is_ok());

        client.close().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_subscribe_clearinghouse_state_integration() {
        let client = WsClient::mainnet();
        client.connect().await.unwrap();

        let result = client.subscribe_clearinghouse_state("0x1234567890123456789012345678901234567890").await;
        assert!(result.is_ok());

        // Check subscription is tracked
        let sub = Subscription::clearinghouse_state("0x1234567890123456789012345678901234567890");
        assert!(client.subscription_manager().contains(&sub).await);

        client.close().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_subscribe_clearinghouse_state_with_dex_integration() {
        let client = WsClient::mainnet();
        client.connect().await.unwrap();

        let result = client.subscribe_clearinghouse_state_with_dex(
            "0x1234567890123456789012345678901234567890",
            "perp"
        ).await;
        assert!(result.is_ok());

        client.close().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_subscribe_open_orders_integration() {
        let client = WsClient::mainnet();
        client.connect().await.unwrap();

        let result = client.subscribe_open_orders("0x1234567890123456789012345678901234567890").await;
        assert!(result.is_ok());

        // Check subscription is tracked
        let sub = Subscription::open_orders("0x1234567890123456789012345678901234567890");
        assert!(client.subscription_manager().contains(&sub).await);

        client.close().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_subscribe_open_orders_with_dex_integration() {
        let client = WsClient::mainnet();
        client.connect().await.unwrap();

        let result = client.subscribe_open_orders_with_dex(
            "0x1234567890123456789012345678901234567890",
            "spot"
        ).await;
        assert!(result.is_ok());

        client.close().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_subscribe_multiple_user_data_integration() {
        let client = WsClient::mainnet();
        client.connect().await.unwrap();

        let user = "0x1234567890123456789012345678901234567890";

        // Subscribe to multiple user data streams
        client.subscribe_notification(user).await.unwrap();
        client.subscribe_web_data3(user).await.unwrap();
        client.subscribe_twap_states(user).await.unwrap();
        client.subscribe_clearinghouse_state(user).await.unwrap();
        client.subscribe_open_orders(user).await.unwrap();

        // Verify all subscriptions are tracked
        assert!(client.subscription_manager().total_count().await >= 5);

        client.close().await.unwrap();
    }
}
