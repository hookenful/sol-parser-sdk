//! Streaming 客户端 - 支持 Yellowstone 与 aRPC v2 的超低延迟 DEX 事件订阅
//!
//! 支持多种事件输出模式：
//! - Unordered: 10-20μs 极低延迟
//! - MicroBatch: 50-200μs 微批次有序
//! - StreamingOrdered: 0.1-5ms 流式有序
//! - Ordered: 1-50ms 完全有序

use super::buffers::{MicroBatchBuffer, SlotBuffer};
use super::deduper::TxDeduper;
use super::types::*;
use crate::core::{now_micros, EventMetadata}; // 导入高性能时钟
use crate::grpc::arpc_proto::arpc::v2 as arpc_v2;
use crate::instr::read_pubkey_fast;
use crate::logs::timestamp_to_microseconds;
use crate::DexEvent;
use crossbeam_queue::ArrayQueue;
use futures::{SinkExt, StreamExt};
use log::{error, info, warn};
use memchr::memmem;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{Duration, Instant};
use tokio_stream::wrappers::ReceiverStream;
use tonic::metadata::MetadataValue;
use tonic::transport::{ClientTlsConfig, Endpoint};
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::prelude::*;

static PROGRAM_DATA_FINDER: Lazy<memmem::Finder> =
    Lazy::new(|| memmem::Finder::new(b"Program data: "));

// ==================== YellowstoneGrpc 客户端 ====================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StreamBackend {
    Yellowstone,
    ArpcV2,
}

#[derive(Debug, Clone)]
enum ArpcControlMessage {
    SetFilters(Vec<ArpcFilterSpec>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ArpcFilterSpec {
    filter_id: String,
    account_include: Vec<String>,
    account_exclude: Vec<String>,
    account_required: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParseSource {
    Yellowstone,
    ArpcLogless,
}

#[derive(Clone)]
struct StreamClient {
    endpoint: String,
    backend: StreamBackend,
    token: Option<String>,
    config: ClientConfig,
    yellowstone_control_tx: Arc<Mutex<Option<mpsc::Sender<SubscribeRequest>>>>,
    arpc_control_tx: Arc<Mutex<Option<mpsc::Sender<ArpcControlMessage>>>>,
    last_filters: Arc<Mutex<SubscriptionFilters>>,
    tx_deduper: Option<Arc<TxDeduper>>,
}

#[derive(Default)]
struct SubscriptionFilters {
    transaction_filters: Vec<TransactionFilter>,
    account_filters: Vec<AccountFilter>,
}

#[derive(Clone)]
pub struct YellowstoneGrpcClient {
    inner: StreamClient,
}

impl YellowstoneGrpcClient {
    pub fn new(
        endpoint: String,
        token: Option<String>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::new_with_config(endpoint, token, ClientConfig::low_latency())
    }

    pub fn new_with_config(
        endpoint: String,
        token: Option<String>,
        config: ClientConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            inner: StreamClient::new_with_backend(
                endpoint,
                token,
                config,
                StreamBackend::Yellowstone,
            ),
        })
    }

    pub fn with_tx_deduper(mut self, tx_deduper: Arc<TxDeduper>) -> Self {
        self.inner.tx_deduper = Some(tx_deduper);
        self
    }

    /// 订阅 DEX 事件（自动重连）
    pub async fn subscribe_dex_events(
        &self,
        transaction_filters: Vec<TransactionFilter>,
        account_filters: Vec<AccountFilter>,
        event_type_filter: Option<EventTypeFilter>,
    ) -> Result<Arc<ArrayQueue<DexEvent>>, Box<dyn std::error::Error>> {
        self.inner
            .subscribe_dex_events(transaction_filters, account_filters, event_type_filter)
            .await
    }

    /// 动态更新订阅过滤器
    pub async fn update_subscription(
        &self,
        transaction_filters: Vec<TransactionFilter>,
        account_filters: Vec<AccountFilter>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.inner
            .update_subscription(transaction_filters, account_filters)
            .await
    }

    pub async fn stop(&self) {
        self.inner.stop().await;
    }
}

#[derive(Clone)]
pub struct CorvusArpcV2Client {
    inner: StreamClient,
}

impl CorvusArpcV2Client {
    pub fn new(
        endpoint: String,
        token: Option<String>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::new_with_config(endpoint, token, ClientConfig::low_latency())
    }

    pub fn new_with_config(
        endpoint: String,
        token: Option<String>,
        config: ClientConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let endpoint = endpoint.trim().to_string();
        if endpoint.starts_with("arpc://")
            || endpoint.starts_with("arpcs://")
            || endpoint.starts_with("arpc+http://")
            || endpoint.starts_with("arpc+https://")
        {
            return Err(
                "CorvusArpcV2Client expects plain http(s) endpoint, not arpc-prefixed URL".into(),
            );
        }
        Ok(Self {
            inner: StreamClient::new_with_backend(endpoint, token, config, StreamBackend::ArpcV2),
        })
    }

    pub fn with_tx_deduper(mut self, tx_deduper: Arc<TxDeduper>) -> Self {
        self.inner.tx_deduper = Some(tx_deduper);
        self
    }

    pub async fn subscribe_dex_events(
        &self,
        transaction_filters: Vec<TransactionFilter>,
        account_filters: Vec<AccountFilter>,
        event_type_filter: Option<EventTypeFilter>,
    ) -> Result<Arc<ArrayQueue<DexEvent>>, Box<dyn std::error::Error>> {
        if !account_filters.is_empty() {
            return Err(
                "CorvusArpcV2Client does not support account subscriptions (SubscribeTransactions only)"
                    .into(),
            );
        }
        if event_filter_requests_account_events(&event_type_filter) {
            return Err(
                "CorvusArpcV2Client does not support account events (TokenAccount/NonceAccount/etc.)"
                    .into(),
            );
        }

        self.inner
            .subscribe_dex_events(transaction_filters, account_filters, event_type_filter)
            .await
    }

    pub async fn update_subscription(
        &self,
        transaction_filters: Vec<TransactionFilter>,
        account_filters: Vec<AccountFilter>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !account_filters.is_empty() {
            return Err(
                "CorvusArpcV2Client does not support account subscriptions (SubscribeTransactions only)"
                    .into(),
            );
        }
        self.inner
            .update_subscription(transaction_filters, account_filters)
            .await
    }

    pub async fn stop(&self) {
        self.inner.stop().await;
    }
}

impl StreamClient {
    fn new_with_backend(
        endpoint: String,
        token: Option<String>,
        config: ClientConfig,
        backend: StreamBackend,
    ) -> Self {
        crate::warmup::warmup_parser();
        Self {
            endpoint,
            backend,
            token,
            config,
            yellowstone_control_tx: Arc::new(Mutex::new(None)),
            arpc_control_tx: Arc::new(Mutex::new(None)),
            last_filters: Arc::new(Mutex::new(SubscriptionFilters::default())),
            tx_deduper: None,
        }
    }

    /// 订阅 DEX 事件（自动重连）
    async fn subscribe_dex_events(
        &self,
        transaction_filters: Vec<TransactionFilter>,
        account_filters: Vec<AccountFilter>,
        event_type_filter: Option<EventTypeFilter>,
    ) -> Result<Arc<ArrayQueue<DexEvent>>, Box<dyn std::error::Error>> {
        let queue = Arc::new(ArrayQueue::new(100_000));
        let queue_clone = Arc::clone(&queue);
        let self_clone = self.clone();

        {
            let mut stored = self.last_filters.lock().await;
            stored.transaction_filters = transaction_filters;
            stored.account_filters = account_filters;
        }

        tokio::spawn(async move {
            loop {
                match self_clone.stream_events(&event_type_filter, &queue_clone).await {
                    Ok(_) => {}
                    Err(e) => {
                        let delay_ms = stream_retry_delay_ms(&e);
                        println!(
                            "❌ gRPC error [{} endpoint={}]: {} - retry in {}ms",
                            stream_backend_name(self_clone.backend),
                            self_clone.endpoint,
                            e,
                            delay_ms
                        );
                        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                        continue;
                    }
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });

        Ok(queue)
    }

    /// 动态更新订阅过滤器
    async fn update_subscription(
        &self,
        transaction_filters: Vec<TransactionFilter>,
        account_filters: Vec<AccountFilter>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        {
            let mut stored = self.last_filters.lock().await;
            stored.transaction_filters = transaction_filters.clone();
            stored.account_filters = account_filters.clone();
        }

        match self.backend {
            StreamBackend::Yellowstone => {
                let request = build_subscribe_request(&transaction_filters, &account_filters);
                let sender = self
                    .yellowstone_control_tx
                    .lock()
                    .await
                    .as_ref()
                    .ok_or("No active subscription")?
                    .clone();
                sender.send(request).await.map_err(|e| e.to_string())?;
            }
            StreamBackend::ArpcV2 => {
                let specs = build_arpc_filter_specs(&transaction_filters, &account_filters);
                let sender = self
                    .arpc_control_tx
                    .lock()
                    .await
                    .as_ref()
                    .ok_or("No active subscription")?
                    .clone();
                sender
                    .send(ArpcControlMessage::SetFilters(specs))
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    async fn stop(&self) {
        println!("🛑 Stopping gRPC subscription...");
    }

    // ==================== 核心事件流处理 ====================

    async fn stream_events(
        &self,
        event_filter: &Option<EventTypeFilter>,
        queue: &Arc<ArrayQueue<DexEvent>>,
    ) -> Result<(), String> {
        match self.backend {
            StreamBackend::Yellowstone => self.stream_events_yellowstone(event_filter, queue).await,
            StreamBackend::ArpcV2 => self.stream_events_arpc(event_filter, queue).await,
        }
    }

    async fn stream_events_yellowstone(
        &self,
        event_filter: &Option<EventTypeFilter>,
        queue: &Arc<ArrayQueue<DexEvent>>,
    ) -> Result<(), String> {
        let _ = rustls::crypto::ring::default_provider().install_default();

        let (tx_filters, acc_filters) = {
            let stored = self.last_filters.lock().await;
            (stored.transaction_filters.clone(), stored.account_filters.clone())
        };
        let connect_endpoint = self.endpoint_for_backend(StreamBackend::Yellowstone);

        info!(
            "gRPC config: endpoint={} tls={} conn_timeout={}ms request_timeout={}ms keepalive_interval={}ms keepalive_timeout={}ms buffer_size={} order_mode={:?} order_timeout_ms={} micro_batch_us={}",
            connect_endpoint,
            self.config.enable_tls,
            self.config.connection_timeout_ms,
            self.config.request_timeout_ms,
            self.config.keep_alive_interval_ms,
            self.config.keep_alive_timeout_ms,
            self.config.buffer_size,
            self.config.order_mode,
            self.config.order_timeout_ms,
            self.config.micro_batch_us
        );

        // 构建客户端
        let mut builder = GeyserGrpcClient::build_from_shared(connect_endpoint.clone())
            .map_err(|e| e.to_string())?
            .x_token(self.token.clone())
            .map_err(|e| e.to_string())?
            .max_decoding_message_size(1024 * 1024 * 1024);

        if self.config.connection_timeout_ms > 0 {
            builder =
                builder.connect_timeout(Duration::from_millis(self.config.connection_timeout_ms));
        }
        if self.config.request_timeout_ms > 0 {
            builder = builder.timeout(Duration::from_millis(self.config.request_timeout_ms));
        }
        if self.config.enable_tls {
            builder = builder
                .tls_config(ClientTlsConfig::new().with_native_roots())
                .map_err(|e| e.to_string())?;
        }
        if self.config.buffer_size > 0 {
            builder = builder.buffer_size(Some(self.config.buffer_size));
        }
        if self.config.keep_alive_interval_ms > 0 {
            let interval = Duration::from_millis(self.config.keep_alive_interval_ms);
            builder = builder
                .http2_keep_alive_interval(interval)
                .keep_alive_while_idle(true)
                .tcp_keepalive(Some(interval));
        }
        if self.config.keep_alive_timeout_ms > 0 {
            builder = builder
                .keep_alive_timeout(Duration::from_millis(self.config.keep_alive_timeout_ms));
        }
        builder = builder.tcp_nodelay(true).http2_adaptive_window(true);

        let mut client = builder.connect().await.map_err(|e| e.to_string())?;
        let request = build_subscribe_request(&tx_filters, &acc_filters);

        let (subscribe_tx, mut stream) =
            client.subscribe_with_request(Some(request)).await.map_err(|e| e.to_string())?;

        self.print_mode_info();

        // 设置控制通道
        let (control_tx, mut control_rx) = mpsc::channel::<SubscribeRequest>(100);
        *self.yellowstone_control_tx.lock().await = Some(control_tx);
        let subscribe_tx = Arc::new(Mutex::new(subscribe_tx));

        // 初始化缓冲区
        let mut slot_buffer = SlotBuffer::new();
        let mut micro_batch = MicroBatchBuffer::new();
        let mut last_slot = 0u64;

        let order_mode = self.config.order_mode;
        let timeout_ms = self.config.order_timeout_ms;
        let batch_us = self.config.micro_batch_us;
        let check_interval = Duration::from_millis(timeout_ms / 2);
        let mut next_check = Instant::now() + check_interval;
        let mut ping_id: i32 = 0;

        loop {
            // Periodic timeout check for ordered modes and MicroBatch
            self.check_timeout(
                order_mode,
                &mut slot_buffer,
                &mut micro_batch,
                queue,
                timeout_ms,
                batch_us,
                &mut next_check,
                check_interval,
            );

            tokio::select! {
                msg = stream.next() => {
                    match msg {
                        Some(Ok(update)) => {
                            if let Some(subscribe_update::UpdateOneof::Ping(_)) = update.update_oneof.as_ref() {
                                ping_id = ping_id.wrapping_add(1);
                                let req = build_ping_request(ping_id);
                                if let Err(e) = subscribe_tx.lock().await.send(req).await {
                                    return Err(e.to_string());
                                }
                                continue;
                            }
                            self.handle_update(
                                update, order_mode, event_filter, queue,
                                &mut slot_buffer, &mut micro_batch, &mut last_slot, batch_us
                            );
                        }
                        Some(Err(e)) => {
                            error!("Stream error: {:?}", e);
                            self.flush_on_disconnect(order_mode, &mut slot_buffer, queue);
                            return Err(e.to_string());
                        }
                        None => {
                            self.flush_on_disconnect(order_mode, &mut slot_buffer, queue);
                            return Ok(());
                        }
                    }
                }
                Some(req) = control_rx.recv() => {
                    if let Err(e) = subscribe_tx.lock().await.send(req).await {
                        return Err(e.to_string());
                    }
                }
            }
        }
    }

    async fn stream_events_arpc(
        &self,
        event_filter: &Option<EventTypeFilter>,
        queue: &Arc<ArrayQueue<DexEvent>>,
    ) -> Result<(), String> {
        let (tx_filters, acc_filters) = {
            let stored = self.last_filters.lock().await;
            (stored.transaction_filters.clone(), stored.account_filters.clone())
        };
        let initial_specs = build_arpc_filter_specs(&tx_filters, &acc_filters);
        let connect_endpoint = self.endpoint_for_backend(StreamBackend::ArpcV2);

        info!(
            "aRPC v2 config: endpoint={} conn_timeout={}ms request_timeout={}ms keepalive_interval={}ms keepalive_timeout={}ms buffer_size={} order_mode={:?} order_timeout_ms={} micro_batch_us={}",
            connect_endpoint,
            self.config.connection_timeout_ms,
            self.config.request_timeout_ms,
            self.config.keep_alive_interval_ms,
            self.config.keep_alive_timeout_ms,
            self.config.buffer_size,
            self.config.order_mode,
            self.config.order_timeout_ms,
            self.config.micro_batch_us
        );

        let mut endpoint =
            Endpoint::from_shared(connect_endpoint.clone()).map_err(|e| e.to_string())?;
        if self.config.connection_timeout_ms > 0 {
            endpoint =
                endpoint.connect_timeout(Duration::from_millis(self.config.connection_timeout_ms));
        }
        if self.config.request_timeout_ms > 0 {
            endpoint = endpoint.timeout(Duration::from_millis(self.config.request_timeout_ms));
        }
        if self.config.enable_tls && connect_endpoint.starts_with("https://") {
            endpoint = endpoint
                .tls_config(ClientTlsConfig::new().with_native_roots())
                .map_err(|e| e.to_string())?;
        }
        if self.config.buffer_size > 0 {
            endpoint = endpoint.buffer_size(Some(self.config.buffer_size));
        }
        if self.config.keep_alive_interval_ms > 0 {
            let interval = Duration::from_millis(self.config.keep_alive_interval_ms);
            endpoint = endpoint
                .http2_keep_alive_interval(interval)
                .keep_alive_while_idle(true)
                .tcp_keepalive(Some(interval));
        }
        if self.config.keep_alive_timeout_ms > 0 {
            endpoint = endpoint
                .keep_alive_timeout(Duration::from_millis(self.config.keep_alive_timeout_ms));
        }
        endpoint = endpoint.tcp_nodelay(true).http2_adaptive_window(true);

        let channel = endpoint.connect().await.map_err(|e| e.to_string())?;
        let mut client = arpc_v2::service_client::ServiceClient::new(channel)
            .max_decoding_message_size(1024 * 1024 * 1024);

        let (outbound_tx, outbound_rx) =
            mpsc::channel::<arpc_v2::SubscribeTransactionsRequest>(256);
        let mut request = tonic::Request::new(ReceiverStream::new(outbound_rx));
        if let Some(token) = self.token.as_ref().filter(|t| !t.trim().is_empty()) {
            attach_x_token(request.metadata_mut(), token)?;
        }

        let mut stream =
            client.subscribe_transactions(request).await.map_err(|e| e.to_string())?.into_inner();

        let (control_tx, mut control_rx) = mpsc::channel::<ArpcControlMessage>(100);
        *self.arpc_control_tx.lock().await = Some(control_tx);

        let mut active_specs: HashMap<String, ArpcFilterSpec> = HashMap::new();
        self.apply_arpc_filter_set(&outbound_tx, &mut active_specs, initial_specs).await?;

        self.print_mode_info();

        let order_mode = self.config.order_mode;
        let timeout_ms = self.config.order_timeout_ms;
        let batch_us = self.config.micro_batch_us;
        let check_interval = Duration::from_millis(timeout_ms / 2);
        let mut next_check = Instant::now() + check_interval;
        let ping_every_ms = self.config.keep_alive_interval_ms.clamp(1_000, 120_000);
        let mut ping_interval = tokio::time::interval(Duration::from_millis(ping_every_ms));
        ping_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut ping_id = 0i32;
        let mut last_sequence: Option<u64> = None;

        let mut slot_buffer = SlotBuffer::new();
        let mut micro_batch = MicroBatchBuffer::new();
        let mut last_slot = 0u64;

        loop {
            self.check_timeout(
                order_mode,
                &mut slot_buffer,
                &mut micro_batch,
                queue,
                timeout_ms,
                batch_us,
                &mut next_check,
                check_interval,
            );

            tokio::select! {
                _ = ping_interval.tick() => {
                    ping_id = ping_id.wrapping_add(1);
                    outbound_tx
                        .send(build_arpc_ping_request(ping_id))
                        .await
                        .map_err(|e| e.to_string())?;
                }
                Some(cmd) = control_rx.recv() => {
                    match cmd {
                        ArpcControlMessage::SetFilters(specs) => {
                            self.apply_arpc_filter_set(&outbound_tx, &mut active_specs, specs).await?;
                        }
                    }
                }
                msg = stream.message() => {
                    match msg {
                        Ok(Some(update)) => {
                            if let Some(prev) = last_sequence {
                                if update.sequence != prev.saturating_add(1) {
                                    self.flush_on_disconnect(order_mode, &mut slot_buffer, queue);
                                    return Err(format!("aRPC sequence gap detected: prev={} current={}", prev, update.sequence));
                                }
                            }
                            last_sequence = Some(update.sequence);

                            let block_time_us = update
                                .created_at
                                .as_ref()
                                .map(|ts| timestamp_to_microseconds(ts) as i64)
                                .unwrap_or_default();
                            let grpc_recv_us = get_timestamp_us();

                            if let Some(payload) = update.payload {
                                match payload {
                                    arpc_v2::subscribe_transactions_response::Payload::TransactionUpdate(tx_update) => {
                                        if let Some(tx) = tx_update.transaction {
                                            if let Some(converted) = convert_arpc_transaction_update(&tx) {
                                                self.handle_transaction(
                                                    converted,
                                                    order_mode,
                                                    event_filter,
                                                    queue,
                                                    &mut slot_buffer,
                                                    &mut micro_batch,
                                                    &mut last_slot,
                                                    batch_us,
                                                    grpc_recv_us,
                                                    block_time_us,
                                                    ParseSource::ArpcLogless,
                                                );
                                            }
                                        }
                                    }
                                    arpc_v2::subscribe_transactions_response::Payload::Error(err_payload) => {
                                        let retry_ms = err_payload.retry_after_ms.unwrap_or(1_000) as u64;
                                        self.flush_on_disconnect(order_mode, &mut slot_buffer, queue);
                                        if retry_ms > 0 {
                                            tokio::time::sleep(Duration::from_millis(retry_ms)).await;
                                        }
                                        return Err(format!(
                                            "aRPC stream error code={} message={}",
                                            err_payload.code,
                                            err_payload.message
                                        ));
                                    }
                                    arpc_v2::subscribe_transactions_response::Payload::FilterValidation(validation) => {
                                        if !validation.accepted {
                                            let reason = validation
                                                .rejection_reason
                                                .unwrap_or_else(|| "unknown".to_string());
                                            self.flush_on_disconnect(order_mode, &mut slot_buffer, queue);
                                            return Err(format!(
                                                "aRPC rejected filter_id={} reason={}",
                                                validation.filter_id, reason
                                            ));
                                        }
                                    }
                                    arpc_v2::subscribe_transactions_response::Payload::Pong(_) => {}
                                }
                            }
                        }
                        Ok(None) => {
                            self.flush_on_disconnect(order_mode, &mut slot_buffer, queue);
                            return Ok(());
                        }
                        Err(e) => {
                            self.flush_on_disconnect(order_mode, &mut slot_buffer, queue);
                            return Err(e.to_string());
                        }
                    }
                }
            }
        }
    }

    async fn apply_arpc_filter_set(
        &self,
        outbound_tx: &mpsc::Sender<arpc_v2::SubscribeTransactionsRequest>,
        active_specs: &mut HashMap<String, ArpcFilterSpec>,
        target_specs: Vec<ArpcFilterSpec>,
    ) -> Result<(), String> {
        let mut target_map: HashMap<String, ArpcFilterSpec> = HashMap::new();
        for spec in target_specs {
            target_map.insert(spec.filter_id.clone(), spec);
        }

        let mut to_remove: Vec<String> =
            active_specs.keys().filter(|id| !target_map.contains_key(*id)).cloned().collect();
        to_remove.sort();
        if !to_remove.is_empty() {
            outbound_tx
                .send(build_arpc_unregister_request(&to_remove))
                .await
                .map_err(|e| e.to_string())?;
        }

        let mut to_add: Vec<ArpcFilterSpec> = target_map
            .iter()
            .filter_map(|(id, spec)| match active_specs.get(id) {
                Some(existing) if existing == spec => None,
                _ => Some(spec.clone()),
            })
            .collect();
        to_add.sort_by(|a, b| a.filter_id.cmp(&b.filter_id));
        if !to_add.is_empty() {
            outbound_tx
                .send(build_arpc_register_request(&to_add))
                .await
                .map_err(|e| e.to_string())?;
        }

        *active_specs = target_map;
        Ok(())
    }

    fn print_mode_info(&self) {
        match self.config.order_mode {
            OrderMode::Unordered => println!("✅ Unordered Mode (10-20μs)"),
            OrderMode::Ordered => {
                println!("✅ Ordered Mode (timeout={}ms)", self.config.order_timeout_ms)
            }
            OrderMode::StreamingOrdered => {
                println!("✅ StreamingOrdered Mode (timeout={}ms)", self.config.order_timeout_ms)
            }
            OrderMode::MicroBatch => {
                println!("✅ MicroBatch Mode (window={}μs)", self.config.micro_batch_us)
            }
        }
    }

    fn endpoint_for_backend(&self, backend: StreamBackend) -> String {
        match backend {
            StreamBackend::Yellowstone | StreamBackend::ArpcV2 => {
                normalize_endpoint(&self.endpoint, self.config.enable_tls)
            }
        }
    }

    #[inline]
    fn check_timeout(
        &self,
        mode: OrderMode,
        slot_buf: &mut SlotBuffer,
        micro_buf: &mut MicroBatchBuffer,
        queue: &Arc<ArrayQueue<DexEvent>>,
        timeout_ms: u64,
        batch_us: u64,
        next_check: &mut Instant,
        interval: Duration,
    ) {
        if Instant::now() < *next_check {
            return;
        }
        *next_check = Instant::now() + interval;

        match mode {
            OrderMode::Ordered => {
                if slot_buf.should_timeout(timeout_ms) {
                    for e in slot_buf.flush_all() {
                        let _ = queue.push(e);
                    }
                }
            }
            OrderMode::StreamingOrdered => {
                if slot_buf.should_timeout(timeout_ms) {
                    for e in slot_buf.flush_streaming_timeout() {
                        let _ = queue.push(e);
                    }
                }
            }
            OrderMode::MicroBatch => {
                // Periodic flush for MicroBatch mode
                let now_us = get_timestamp_us();
                if micro_buf.should_flush(now_us, batch_us) {
                    for e in micro_buf.flush() {
                        let _ = queue.push(e);
                    }
                }
            }
            OrderMode::Unordered => {}
        }
    }

    fn flush_on_disconnect(
        &self,
        mode: OrderMode,
        buffer: &mut SlotBuffer,
        queue: &Arc<ArrayQueue<DexEvent>>,
    ) {
        if matches!(mode, OrderMode::Ordered | OrderMode::StreamingOrdered) {
            let events = match mode {
                OrderMode::StreamingOrdered => buffer.flush_streaming_timeout(),
                _ => buffer.flush_all(),
            };
            for e in events {
                let _ = queue.push(e);
            }
        }
    }

    #[inline]
    fn handle_update(
        &self,
        update_msg: SubscribeUpdate,
        mode: OrderMode,
        filter: &Option<EventTypeFilter>,
        queue: &Arc<ArrayQueue<DexEvent>>,
        slot_buf: &mut SlotBuffer,
        micro_buf: &mut MicroBatchBuffer,
        last_slot: &mut u64,
        batch_us: u64,
    ) {
        let block_time_us =
            timestamp_to_microseconds(&update_msg.created_at.unwrap_or_default()) as i64;
        let grpc_recv_us = get_timestamp_us();

        let Some(update) = update_msg.update_oneof else { return };

        match update {
            subscribe_update::UpdateOneof::Transaction(tx) => {
                self.handle_transaction(
                    tx,
                    mode,
                    filter,
                    queue,
                    slot_buf,
                    micro_buf,
                    last_slot,
                    batch_us,
                    grpc_recv_us,
                    block_time_us,
                    ParseSource::Yellowstone,
                );
            }
            subscribe_update::UpdateOneof::Account(acc) => {
                Self::handle_account(acc, filter, queue, grpc_recv_us, block_time_us);
            }
            _ => {}
        }
    }

    #[inline]
    fn handle_transaction(
        &self,
        tx: SubscribeUpdateTransaction,
        mode: OrderMode,
        filter: &Option<EventTypeFilter>,
        queue: &Arc<ArrayQueue<DexEvent>>,
        slot_buf: &mut SlotBuffer,
        micro_buf: &mut MicroBatchBuffer,
        last_slot: &mut u64,
        batch_us: u64,
        grpc_us: i64,
        block_us: i64,
        parse_source: ParseSource,
    ) {
        let slot = tx.slot;

        if let Some(deduper) = &self.tx_deduper {
            let Some(info) = tx.transaction.as_ref() else { return };
            let sig = extract_signature(&info.signature);
            let log_enabled = log::log_enabled!(log::Level::Info);
            let log_label = deduper.log_label();
            let start_us = if log_enabled && log_label.is_some() { now_micros() } else { 0 };
            if !deduper.check(sig, slot, grpc_us) {
                return;
            }
            if let Some(label) = log_label {
                if log_enabled {
                    let dedup_us = now_micros().saturating_sub(start_us);
                    info!("{label}: {}: {sig}: {slot}: dedup_us={dedup_us}", self.endpoint);
                }
            }
        }

        match mode {
            OrderMode::Unordered => {
                for e in parse_transaction_core(
                    &tx,
                    grpc_us,
                    Some(block_us),
                    filter.as_ref(),
                    parse_source,
                ) {
                    let _ = queue.push(e);
                }
            }
            OrderMode::Ordered => {
                if slot > *last_slot && *last_slot > 0 {
                    for e in slot_buf.flush_before(slot) {
                        let _ = queue.push(e);
                    }
                }
                *last_slot = slot;
                for (idx, e) in parse_transaction_to_vec(
                    &tx,
                    grpc_us,
                    Some(block_us),
                    filter.as_ref(),
                    parse_source,
                ) {
                    slot_buf.push(slot, idx, e);
                }
            }
            OrderMode::StreamingOrdered => {
                for (idx, e) in parse_transaction_to_vec(
                    &tx,
                    grpc_us,
                    Some(block_us),
                    filter.as_ref(),
                    parse_source,
                ) {
                    for evt in slot_buf.push_streaming(slot, idx, e) {
                        let _ = queue.push(evt);
                    }
                }
            }
            OrderMode::MicroBatch => {
                for (idx, e) in parse_transaction_to_vec(
                    &tx,
                    grpc_us,
                    Some(block_us),
                    filter.as_ref(),
                    parse_source,
                ) {
                    if micro_buf.push(slot, idx, e, grpc_us, batch_us) {
                        for evt in micro_buf.flush() {
                            let _ = queue.push(evt);
                        }
                    }
                }
            }
        }
    }

    #[inline]
    fn handle_account(
        acc: SubscribeUpdateAccount,
        filter: &Option<EventTypeFilter>,
        queue: &Arc<ArrayQueue<DexEvent>>,
        grpc_us: i64,
        block_us: i64,
    ) {
        let Some(info) = acc.account else { return };
        let data = crate::accounts::AccountData {
            pubkey: read_pubkey_fast(&info.pubkey),
            executable: info.executable,
            lamports: info.lamports,
            owner: read_pubkey_fast(&info.owner),
            rent_epoch: info.rent_epoch,
            data: info.data,
        };
        let meta = EventMetadata {
            signature: Default::default(),
            slot: acc.slot,
            tx_index: 0,
            block_time_us: block_us,
            grpc_recv_us: grpc_us,
        };
        if let Some(e) = crate::accounts::parse_account_unified(&data, meta, filter.as_ref()) {
            let _ = queue.push(e);
        }
    }
}

// ==================== 辅助函数 ====================

/// 获取当前时间戳（微秒）
///
/// 使用高性能时钟，避免系统调用开销
///
/// # 性能优势
/// - 旧实现：使用 libc::clock_gettime，每次调用约 1-2μs
/// - 新实现：使用高性能时钟，每次调用约 10-50ns
/// - 性能提升：20-100 倍
#[inline(always)]
fn get_timestamp_us() -> i64 {
    now_micros()
}

fn stream_backend_name(backend: StreamBackend) -> &'static str {
    match backend {
        StreamBackend::Yellowstone => "yellowstone",
        StreamBackend::ArpcV2 => "arpc-v2",
    }
}

fn is_unimplemented_error(err: &str) -> bool {
    let lower = err.to_ascii_lowercase();
    lower.contains("unimplemented")
        || lower.contains("operation is not implemented or not supported")
        || lower.contains("method not found")
}

fn stream_retry_delay_ms(err: &str) -> u64 {
    let lower = err.to_ascii_lowercase();
    if is_unimplemented_error(&lower) {
        5_000
    } else if lower.contains("unauthorized")
        || lower.contains("permission denied")
        || lower.contains("authentication")
    {
        3_000
    } else if lower.contains("rate limit")
        || lower.contains("resource has been exhausted")
        || lower.contains("quota")
    {
        2_000
    } else {
        1_000
    }
}

fn normalize_endpoint(endpoint: &str, enable_tls: bool) -> String {
    let endpoint = endpoint.trim();
    if endpoint.starts_with("https://") || endpoint.starts_with("http://") {
        endpoint.to_string()
    } else {
        let scheme = if enable_tls { "https" } else { "http" };
        format!("{scheme}://{endpoint}")
    }
}

fn event_filter_requests_account_events(filter: &Option<EventTypeFilter>) -> bool {
    let Some(filter) = filter else { return false };
    let Some(include_only) = filter.include_only.as_ref() else {
        return false;
    };
    include_only.iter().any(|event_type| {
        matches!(
            event_type,
            EventType::TokenAccount
                | EventType::NonceAccount
                | EventType::AccountPumpSwapGlobalConfig
                | EventType::AccountPumpSwapPool
        )
    })
}

fn attach_x_token(metadata: &mut tonic::metadata::MetadataMap, token: &str) -> Result<(), String> {
    let value = MetadataValue::from_str(token).map_err(|e| e.to_string())?;
    metadata.insert("x-token", value);
    Ok(())
}

fn normalize_filter_values(values: &[String]) -> Vec<String> {
    let mut normalized: Vec<String> =
        values.iter().map(|v| v.trim()).filter(|v| !v.is_empty()).map(|v| v.to_string()).collect();
    normalized.sort();
    normalized.dedup();
    normalized
}

fn build_arpc_filter_specs(
    tx_filters: &[TransactionFilter],
    acc_filters: &[AccountFilter],
) -> Vec<ArpcFilterSpec> {
    let mut specs: HashMap<String, ArpcFilterSpec> = HashMap::new();

    for filter in tx_filters {
        let account_include = normalize_filter_values(&filter.account_include);
        let account_exclude = normalize_filter_values(&filter.account_exclude);
        let account_required = normalize_filter_values(&filter.account_required);
        if account_include.is_empty() && account_exclude.is_empty() && account_required.is_empty() {
            continue;
        }
        let filter_id = build_arpc_filter_id(&account_include, &account_exclude, &account_required);
        specs.entry(filter_id.clone()).or_insert(ArpcFilterSpec {
            filter_id,
            account_include,
            account_exclude,
            account_required,
        });
    }

    if !acc_filters.is_empty() {
        warn!(
            "aRPC v2 SubscribeTransactions does not support account subscriptions; ignoring {} account filter groups",
            acc_filters.len()
        );
    }

    let mut result: Vec<ArpcFilterSpec> = specs.into_values().collect();
    result.sort_by(|a, b| a.filter_id.cmp(&b.filter_id));
    result
}

fn build_arpc_filter_id(
    account_include: &[String],
    account_exclude: &[String],
    account_required: &[String],
) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    account_include.hash(&mut hasher);
    account_exclude.hash(&mut hasher);
    account_required.hash(&mut hasher);
    format!("f_{:016x}", hasher.finish())
}

fn build_arpc_register_request(specs: &[ArpcFilterSpec]) -> arpc_v2::SubscribeTransactionsRequest {
    let filters = specs
        .iter()
        .map(|spec| arpc_v2::TransactionFilter {
            filter_id: spec.filter_id.clone(),
            filter: Some(arpc_v2::SubscribeRequestFilterTransactions {
                account_include: spec.account_include.clone(),
                account_exclude: spec.account_exclude.clone(),
                account_required: spec.account_required.clone(),
            }),
        })
        .collect();

    arpc_v2::SubscribeTransactionsRequest {
        payload: Some(arpc_v2::subscribe_transactions_request::Payload::RegisterFilters(
            arpc_v2::RegisterTransactionFilters { filters },
        )),
    }
}

fn build_arpc_unregister_request(filter_ids: &[String]) -> arpc_v2::SubscribeTransactionsRequest {
    arpc_v2::SubscribeTransactionsRequest {
        payload: Some(arpc_v2::subscribe_transactions_request::Payload::UnregisterFilters(
            arpc_v2::UnregisterTransactionFilters { filter_ids: filter_ids.to_vec() },
        )),
    }
}

fn build_arpc_ping_request(ping_id: i32) -> arpc_v2::SubscribeTransactionsRequest {
    arpc_v2::SubscribeTransactionsRequest {
        payload: Some(arpc_v2::subscribe_transactions_request::Payload::Ping(arpc_v2::Ping {
            ping_id,
        })),
    }
}

fn convert_arpc_transaction_update(
    tx: &arpc_v2::SubscribeUpdateTransaction,
) -> Option<SubscribeUpdateTransaction> {
    let tx_data = tx.transaction.as_ref()?;

    let mut info = SubscribeUpdateTransactionInfo::default();
    info.signature = decode_arpc_signature(&tx.signature, &tx_data.signatures);
    info.is_vote = tx_data.is_vote;
    info.index = 0;
    info.transaction = Some(convert_arpc_transaction(tx_data));
    info.meta = Some(build_arpc_default_meta(tx_data.message.as_ref()));

    let mut update = SubscribeUpdateTransaction::default();
    update.slot = tx.slot;
    update.transaction = Some(info);
    Some(update)
}

fn convert_arpc_transaction(
    tx: &arpc_v2::Transaction,
) -> yellowstone_grpc_proto::prelude::Transaction {
    let mut converted = yellowstone_grpc_proto::prelude::Transaction::default();
    converted.signatures = tx.signatures.clone();
    converted.message = tx.message.as_ref().map(convert_arpc_message);
    converted
}

fn convert_arpc_message(message: &arpc_v2::Message) -> yellowstone_grpc_proto::prelude::Message {
    let mut converted = yellowstone_grpc_proto::prelude::Message::default();
    converted.header = message.header.as_ref().map(|header| {
        let mut h = yellowstone_grpc_proto::prelude::MessageHeader::default();
        h.num_required_signatures = header.num_required_signatures;
        h.num_readonly_signed_accounts = header.num_readonly_signed_accounts;
        h.num_readonly_unsigned_accounts = header.num_readonly_unsigned_accounts;
        h
    });
    converted.account_keys = message.account_keys.clone();
    converted.recent_blockhash = message.recent_blockhash.clone();
    converted.instructions = message
        .instructions
        .iter()
        .map(|ix| {
            let mut out = yellowstone_grpc_proto::prelude::CompiledInstruction::default();
            out.program_id_index = ix.program_id_index;
            out.accounts = ix.accounts.clone();
            out.data = ix.data.clone();
            out
        })
        .collect();
    converted.versioned = message.versioned.unwrap_or(false);
    converted.address_table_lookups = message
        .address_table_lookups
        .iter()
        .map(|lookup| {
            let mut out = yellowstone_grpc_proto::prelude::MessageAddressTableLookup::default();
            out.account_key = lookup.account_key.clone();
            out.writable_indexes = lookup.writable_indexes.clone();
            out.readonly_indexes = lookup.readonly_indexes.clone();
            out
        })
        .collect();
    converted
}

fn build_arpc_default_meta(
    message: Option<&arpc_v2::Message>,
) -> yellowstone_grpc_proto::prelude::TransactionStatusMeta {
    let mut meta = yellowstone_grpc_proto::prelude::TransactionStatusMeta::default();
    meta.inner_instructions_none = true;
    meta.log_messages_none = true;
    meta.return_data_none = true;
    if let Some(message) = message {
        meta.loaded_writable_addresses = message.loaded_writable_addresses.clone();
        meta.loaded_readonly_addresses = message.loaded_readonly_addresses.clone();
    }
    meta
}

fn decode_arpc_signature(signature: &str, signatures: &[Vec<u8>]) -> Vec<u8> {
    let signature = signature.trim();
    if !signature.is_empty() {
        if let Ok(decoded) = bs58::decode(signature).into_vec() {
            if decoded.len() == 64 {
                return decoded;
            }
        }
    }

    if let Some(first) = signatures.first() {
        if first.len() >= 64 {
            return first[..64].to_vec();
        }
    }

    vec![0u8; 64]
}

fn build_subscribe_request(
    tx_filters: &[TransactionFilter],
    acc_filters: &[AccountFilter],
) -> SubscribeRequest {
    let transactions = tx_filters
        .iter()
        .enumerate()
        .map(|(i, f)| {
            (
                format!("tx_{}", i),
                SubscribeRequestFilterTransactions {
                    vote: Some(false),
                    failed: Some(false),
                    signature: None,
                    account_include: f.account_include.clone(),
                    account_exclude: f.account_exclude.clone(),
                    account_required: f.account_required.clone(),
                },
            )
        })
        .collect();

    let accounts = acc_filters
        .iter()
        .enumerate()
        .map(|(i, f)| {
            (
                format!("acc_{}", i),
                SubscribeRequestFilterAccounts {
                    account: f.account.clone(),
                    owner: f.owner.clone(),
                    filters: f.filters.clone(),
                    nonempty_txn_signature: None,
                },
            )
        })
        .collect();

    SubscribeRequest {
        slots: HashMap::new(),
        accounts,
        transactions,
        transactions_status: HashMap::new(),
        blocks: HashMap::new(),
        blocks_meta: HashMap::new(),
        entry: HashMap::new(),
        commitment: Some(CommitmentLevel::Processed as i32),
        accounts_data_slice: Vec::new(),
        ping: None,
        from_slot: None,
    }
}

fn build_ping_request(id: i32) -> SubscribeRequest {
    SubscribeRequest {
        slots: HashMap::new(),
        accounts: HashMap::new(),
        transactions: HashMap::new(),
        transactions_status: HashMap::new(),
        blocks: HashMap::new(),
        blocks_meta: HashMap::new(),
        entry: HashMap::new(),
        commitment: None,
        accounts_data_slice: Vec::new(),
        ping: Some(SubscribeRequestPing { id }),
        from_slot: None,
    }
}

// ==================== 交易解析 ====================

#[inline]
fn parse_transaction_to_vec(
    tx: &SubscribeUpdateTransaction,
    grpc_us: i64,
    block_us: Option<i64>,
    filter: Option<&EventTypeFilter>,
    parse_source: ParseSource,
) -> Vec<(u64, DexEvent)> {
    let idx = tx.transaction.as_ref().map(|t| t.index).unwrap_or(0);
    parse_transaction_core(tx, grpc_us, block_us, filter, parse_source)
        .into_iter()
        .map(|e| (idx, e))
        .collect()
}

#[inline]
fn parse_transaction_core(
    tx: &SubscribeUpdateTransaction,
    grpc_us: i64,
    block_us: Option<i64>,
    filter: Option<&EventTypeFilter>,
    parse_source: ParseSource,
) -> Vec<DexEvent> {
    let Some(info) = &tx.transaction else { return Vec::new() };
    let Some(meta) = &info.meta else { return Vec::new() };

    let sig = extract_signature(&info.signature);
    let slot = tx.slot;
    let idx = info.index;

    let (log_events, instr_events) = match parse_source {
        ParseSource::Yellowstone => rayon::join(
            || {
                parse_logs(
                    meta,
                    &info.transaction,
                    &meta.log_messages,
                    sig,
                    slot,
                    idx,
                    block_us,
                    grpc_us,
                    filter,
                )
            },
            || {
                parse_instructions(
                    meta,
                    &info.transaction,
                    sig,
                    slot,
                    idx,
                    block_us,
                    grpc_us,
                    filter,
                )
            },
        ),
        ParseSource::ArpcLogless => (
            Vec::new(),
            parse_instructions_logless(
                meta,
                &info.transaction,
                sig,
                slot,
                idx,
                block_us,
                grpc_us,
                filter,
            ),
        ),
    };

    let mut result = Vec::with_capacity(log_events.len() + instr_events.len());
    result.extend(log_events);
    result.extend(instr_events);
    result
}

#[inline(always)]
fn extract_signature(bytes: &[u8]) -> solana_sdk::signature::Signature {
    let mut arr = [0u8; 64];
    if bytes.len() >= 64 {
        arr.copy_from_slice(&bytes[..64]);
    } else {
        arr[..bytes.len()].copy_from_slice(bytes);
    }
    solana_sdk::signature::Signature::from(arr)
}

#[inline]
fn parse_logs(
    meta: &TransactionStatusMeta,
    transaction: &Option<yellowstone_grpc_proto::prelude::Transaction>,
    logs: &[String],
    sig: solana_sdk::signature::Signature,
    slot: u64,
    tx_idx: u64,
    block_us: Option<i64>,
    grpc_us: i64,
    filter: Option<&EventTypeFilter>,
) -> Vec<DexEvent> {
    let needs_pumpfun = filter.map(|f| f.includes_pumpfun()).unwrap_or(true);
    let has_create = needs_pumpfun && crate::logs::optimized_matcher::detect_pumpfun_create(logs);

    let mut outer_idx: i32 = -1;
    let mut inner_idx: i32 = -1;
    let mut invokes: HashMap<&str, Vec<(i32, i32)>> = HashMap::with_capacity(8);
    let mut result = Vec::with_capacity(4);

    for log in logs {
        if let Some((pid, depth)) = crate::logs::optimized_matcher::parse_invoke_info(log) {
            if depth == 1 {
                inner_idx = -1;
                outer_idx += 1;
            } else {
                inner_idx += 1;
            }
            invokes.entry(pid).or_default().push((outer_idx, inner_idx));
        }

        if PROGRAM_DATA_FINDER.find(log.as_bytes()).is_none() {
            continue;
        }

        if let Some(mut e) =
            crate::logs::parse_log(log, sig, slot, tx_idx, block_us, grpc_us, filter, has_create)
        {
            crate::core::account_dispatcher::fill_accounts_from_transaction_data(
                &mut e,
                meta,
                transaction,
                &invokes,
            );
            crate::core::common_filler::fill_data(&mut e, meta, transaction, &invokes);
            result.push(e);
        }
    }
    result
}

#[inline]
fn parse_instructions(
    meta: &TransactionStatusMeta,
    transaction: &Option<yellowstone_grpc_proto::prelude::Transaction>,
    sig: solana_sdk::signature::Signature,
    slot: u64,
    tx_idx: u64,
    block_us: Option<i64>,
    grpc_us: i64,
    filter: Option<&EventTypeFilter>,
) -> Vec<DexEvent> {
    // 使用增强的 instruction 解析器
    // 支持：
    // - 主指令解析（8字节 discriminator）
    // - Inner instruction 解析（16字节 discriminator）
    // - 自动事件合并（instruction + inner instruction）
    crate::grpc::instruction_parser::parse_instructions_enhanced(
        meta,
        transaction,
        sig,
        slot,
        tx_idx,
        block_us,
        grpc_us,
        filter,
    )
}

#[inline]
fn parse_instructions_logless(
    meta: &TransactionStatusMeta,
    transaction: &Option<yellowstone_grpc_proto::prelude::Transaction>,
    sig: solana_sdk::signature::Signature,
    slot: u64,
    tx_idx: u64,
    block_us: Option<i64>,
    grpc_us: i64,
    filter: Option<&EventTypeFilter>,
) -> Vec<DexEvent> {
    crate::grpc::instruction_parser::parse_instructions_enhanced_logless(
        meta,
        transaction,
        sig,
        slot,
        tx_idx,
        block_us,
        grpc_us,
        filter,
    )
}
