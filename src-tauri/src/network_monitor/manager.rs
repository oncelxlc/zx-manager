use super::accumulator::TrafficAccumulator;
use super::collector::{create_platform_collector, PlatformNetworkCollector};
use super::dto::{
    ClearNetworkUsageRequest, ClearNetworkUsageResult, CollectorState, HelperState,
    NetworkMonitorCapabilities, NetworkMonitorStatus, NetworkMonitorWarning, NetworkRealtimeEvent,
    NetworkSubscription, NetworkUsageQuery, NetworkUsageResult, SampleState, TrafficValues,
};
use super::error::{NetworkMonitorError, NetworkMonitorErrorCode, NetworkMonitorResult};
use super::storage::{StorageWorker, UsageBatch, UsageBucketDelta};
use chrono::Utc;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tauri::ipc::Channel;

const REALTIME_CAPACITY: usize = 600;
const DEFAULT_SAMPLE_INTERVAL_SECONDS: u64 = 5;
const ALLOWED_SAMPLE_INTERVAL_SECONDS: [u64; 4] = [1, 3, 5, 10];
const FLUSH_INTERVAL: Duration = Duration::from_secs(60);

pub struct NetworkMonitorManager {
    inner: Arc<ManagerInner>,
    collector_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl Clone for NetworkMonitorManager {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            collector_handle: Arc::clone(&self.collector_handle),
        }
    }
}

struct ManagerInner {
    enabled: AtomicBool,
    authorization_ready: AtomicBool,
    shutdown: AtomicBool,
    generation: AtomicU64,
    sequence: AtomicU64,
    next_subscription_id: AtomicU64,
    subscriber_count: AtomicUsize,
    sample_interval_seconds: AtomicU64,
    sample_interval_revision: AtomicU64,
    lost_events: AtomicU64,
    unresolved_events: AtomicU64,
    collector_state: Mutex<CollectorState>,
    helper_state: Mutex<HelperState>,
    last_sampled_at: Mutex<Option<i64>>,
    last_error: Mutex<Option<NetworkMonitorWarning>>,
    realtime: Mutex<VecDeque<NetworkRealtimeEvent>>,
    subscribers: Mutex<HashMap<u64, Channel<NetworkRealtimeEvent>>>,
    accumulator: Mutex<TrafficAccumulator>,
    pending_rows: Mutex<Vec<UsageBucketDelta>>,
    storage: Mutex<StorageWorker>,
    storage_session_id: String,
}

impl NetworkMonitorManager {
    pub fn new(database_path: PathBuf) -> Self {
        Self {
            inner: Arc::new(ManagerInner {
                enabled: AtomicBool::new(false),
                authorization_ready: AtomicBool::new(false),
                shutdown: AtomicBool::new(false),
                generation: AtomicU64::new(0),
                sequence: AtomicU64::new(0),
                next_subscription_id: AtomicU64::new(1),
                subscriber_count: AtomicUsize::new(0),
                sample_interval_seconds: AtomicU64::new(DEFAULT_SAMPLE_INTERVAL_SECONDS),
                sample_interval_revision: AtomicU64::new(0),
                lost_events: AtomicU64::new(0),
                unresolved_events: AtomicU64::new(0),
                collector_state: Mutex::new(CollectorState::Disabled),
                helper_state: Mutex::new(HelperState::Stopped),
                last_sampled_at: Mutex::new(None),
                last_error: Mutex::new(None),
                realtime: Mutex::new(VecDeque::with_capacity(REALTIME_CAPACITY)),
                subscribers: Mutex::new(HashMap::new()),
                accumulator: Mutex::new(TrafficAccumulator::default()),
                pending_rows: Mutex::new(Vec::new()),
                storage: Mutex::new(StorageWorker::new(database_path)),
                storage_session_id: format!(
                    "{}-{}",
                    std::process::id(),
                    Utc::now().timestamp_nanos_opt().unwrap_or_default()
                ),
            }),
            collector_handle: Arc::new(Mutex::new(None)),
        }
    }

    pub fn capabilities(&self) -> NetworkMonitorCapabilities {
        NetworkMonitorCapabilities {
            platform: std::env::consts::OS.to_owned(),
            platform_supported: cfg!(windows),
            requires_elevation: cfg!(windows),
            application_traffic: cfg!(windows),
            proxy_classification: cfg!(windows),
            history_storage: true,
            retention_days: 7,
        }
    }

    pub fn status(&self) -> NetworkMonitorStatus {
        let lost_events = self.inner.lost_events.load(Ordering::Acquire);
        let unresolved_events = self.inner.unresolved_events.load(Ordering::Acquire);
        NetworkMonitorStatus {
            platform_supported: cfg!(windows),
            requires_elevation: cfg!(windows),
            authorization_ready: self.inner.authorization_ready.load(Ordering::Acquire),
            enabled: self.inner.enabled.load(Ordering::Acquire),
            collector_state: *lock(&self.inner.collector_state),
            helper_state: *lock(&self.inner.helper_state),
            generation: self.inner.generation.load(Ordering::Acquire),
            subscriber_count: self.inner.subscriber_count.load(Ordering::Acquire),
            sample_interval_seconds: self.inner.sample_interval_seconds.load(Ordering::Acquire),
            database_created: lock(&self.inner.storage).database_exists(),
            last_sampled_at: *lock(&self.inner.last_sampled_at),
            lost_events,
            unresolved_events,
            partial_data: lost_events > 0 || unresolved_events > 0,
            last_error: lock(&self.inner.last_error).clone(),
        }
    }

    pub fn set_enabled(&self, enabled: bool) -> NetworkMonitorResult<NetworkMonitorStatus> {
        if enabled && !cfg!(windows) {
            return Err(NetworkMonitorError::new(
                NetworkMonitorErrorCode::UnsupportedPlatform,
                "application network monitoring is supported on Windows only",
            ));
        }
        if enabled {
            self.prepare()?;
            self.ensure_collector_thread()?;
            let starting_new_session = !self.inner.enabled.swap(true, Ordering::AcqRel);
            if starting_new_session {
                self.inner.generation.fetch_add(1, Ordering::AcqRel);
                self.inner.sequence.store(0, Ordering::Release);
                lock(&self.inner.realtime).clear();
                lock(&self.inner.pending_rows).clear();
                lock(&self.inner.accumulator).reset();
                self.inner.lost_events.store(0, Ordering::Release);
                self.inner.unresolved_events.store(0, Ordering::Release);
                *lock(&self.inner.last_sampled_at) = None;
            }
            *lock(&self.inner.collector_state) = CollectorState::Starting;
            *lock(&self.inner.helper_state) = HelperState::Starting;
            *lock(&self.inner.last_error) = None;
        } else {
            self.inner.enabled.store(false, Ordering::Release);
            *lock(&self.inner.collector_state) = CollectorState::Disabled;
            self.flush_pending();
            self.emit_paused();
        }
        Ok(self.status())
    }

    pub fn prepare(&self) -> NetworkMonitorResult<NetworkMonitorStatus> {
        if !cfg!(windows) {
            return Err(NetworkMonitorError::new(
                NetworkMonitorErrorCode::UnsupportedPlatform,
                "application network monitoring is supported on Windows only",
            ));
        }
        if self.inner.authorization_ready.load(Ordering::Acquire) {
            return Ok(self.status());
        }
        #[cfg(windows)]
        {
            // This starts the same narrowly-scoped elevated helper used by collection,
            // validates its pipe handshake, and immediately shuts it down without sampling.
            // It intentionally does not touch the SQLite worker or enable collection.
            let mut helper = super::helper::HelperClient::start()?;
            helper.prepare()?;
            drop(helper);
        }
        self.inner
            .authorization_ready
            .store(true, Ordering::Release);
        Ok(self.status())
    }

    pub fn set_sample_interval(
        &self,
        sample_interval_seconds: u64,
    ) -> NetworkMonitorResult<NetworkMonitorStatus> {
        if !ALLOWED_SAMPLE_INTERVAL_SECONDS.contains(&sample_interval_seconds) {
            return Err(NetworkMonitorError::new(
                NetworkMonitorErrorCode::InvalidRequest,
                "sampleIntervalSeconds must be one of 1, 3, 5, or 10",
            ));
        }
        let previous = self
            .inner
            .sample_interval_seconds
            .swap(sample_interval_seconds, Ordering::AcqRel);
        if previous != sample_interval_seconds {
            self.inner
                .sample_interval_revision
                .fetch_add(1, Ordering::AcqRel);
        }
        Ok(self.status())
    }

    pub fn subscribe(
        &self,
        channel: Channel<NetworkRealtimeEvent>,
    ) -> NetworkMonitorResult<NetworkSubscription> {
        let subscription_id = self
            .inner
            .next_subscription_id
            .fetch_add(1, Ordering::AcqRel);
        lock(&self.inner.subscribers).insert(subscription_id, channel);
        self.inner.subscriber_count.fetch_add(1, Ordering::AcqRel);
        Ok(NetworkSubscription {
            subscription_id,
            generation: self.inner.generation.load(Ordering::Acquire),
            initial_events: lock(&self.inner.realtime).iter().cloned().collect(),
        })
    }

    pub fn unsubscribe(&self, subscription_id: u64) {
        if lock(&self.inner.subscribers)
            .remove(&subscription_id)
            .is_some()
        {
            self.inner.subscriber_count.fetch_sub(1, Ordering::AcqRel);
        }
    }

    pub fn query(&self, request: NetworkUsageQuery) -> NetworkMonitorResult<NetworkUsageResult> {
        self.flush_pending();
        let generation = self.inner.generation.load(Ordering::Acquire);
        lock(&self.inner.storage).query(request, generation)
    }

    pub fn clear(
        &self,
        request: ClearNetworkUsageRequest,
    ) -> NetworkMonitorResult<ClearNetworkUsageResult> {
        self.flush_pending();
        let result = lock(&self.inner.storage).clear(request)?;
        let generation = self.inner.generation.fetch_add(1, Ordering::AcqRel) + 1;
        lock(&self.inner.realtime).clear();
        lock(&self.inner.pending_rows).clear();
        lock(&self.inner.accumulator).reset();
        self.inner.sequence.store(0, Ordering::Release);
        self.inner.lost_events.store(0, Ordering::Release);
        self.inner.unresolved_events.store(0, Ordering::Release);
        Ok(ClearNetworkUsageResult {
            generation,
            deleted_buckets: result.deleted_buckets,
            cleared_at: Utc::now().timestamp_millis(),
        })
    }

    pub fn shutdown(&self) {
        self.inner.shutdown.store(true, Ordering::Release);
        self.inner.enabled.store(false, Ordering::Release);
        if let Some(handle) = lock(&self.collector_handle).take() {
            let _ = handle.join();
        }
        self.flush_pending();
        lock(&self.inner.storage).shutdown(Duration::from_secs(3));
        *lock(&self.inner.collector_state) = CollectorState::Stopped;
        *lock(&self.inner.helper_state) = HelperState::Stopped;
    }

    fn ensure_collector_thread(&self) -> NetworkMonitorResult<()> {
        let mut handle = lock(&self.collector_handle);
        if handle.is_some() {
            return Ok(());
        }
        self.inner.shutdown.store(false, Ordering::Release);
        let inner = Arc::clone(&self.inner);
        *handle = Some(
            thread::Builder::new()
                .name("network-collector".to_owned())
                .spawn(move || collector_loop(inner))
                .map_err(|error| {
                    NetworkMonitorError::new(
                        NetworkMonitorErrorCode::CollectorUnavailable,
                        format!("failed to start collector: {error}"),
                    )
                })?,
        );
        Ok(())
    }

    fn flush_pending(&self) {
        flush_pending_rows(&self.inner);
    }

    fn emit_paused(&self) {
        let event = NetworkRealtimeEvent {
            generation: self.inner.generation.load(Ordering::Acquire),
            sequence: self.inner.sequence.fetch_add(1, Ordering::AcqRel) + 1,
            sampled_at: Utc::now().timestamp_millis(),
            elapsed_ms: None,
            sample_state: SampleState::Paused,
            applications: Vec::new(),
            unknown_traffic: TrafficValues::default(),
            lost_events: 0,
            unresolved_events: 0,
            warnings: Vec::new(),
        };
        broadcast_event(&self.inner, event);
    }
}

fn collector_loop(inner: Arc<ManagerInner>) {
    let mut collector: Option<Box<dyn PlatformNetworkCollector>> = None;
    let mut last_flush = Instant::now();
    let mut previous_sample: Option<Instant> = None;
    while !inner.shutdown.load(Ordering::Acquire) {
        if !inner.enabled.load(Ordering::Acquire) {
            if collector.take().is_some() {
                *lock(&inner.helper_state) = HelperState::Stopped;
            }
            thread::sleep(Duration::from_millis(100));
            previous_sample = None;
            continue;
        }

        if collector.is_none() {
            collector = Some(create_platform_collector());
            *lock(&inner.helper_state) = HelperState::Starting;
        }
        let started = Instant::now();
        let sample_interval =
            Duration::from_secs(inner.sample_interval_seconds.load(Ordering::Acquire));
        let sample_interval_revision = inner.sample_interval_revision.load(Ordering::Acquire);
        let result = collector.as_mut().expect("collector initialized").collect();
        match result {
            Ok(sample) => {
                let finished = Instant::now();
                let elapsed = previous_sample
                    .map(|previous| finished.saturating_duration_since(previous))
                    .unwrap_or(sample_interval);
                previous_sample = Some(finished);
                let sampled_at = Utc::now().timestamp_millis();
                let generation = inner.generation.load(Ordering::Acquire);
                let sequence = inner.sequence.fetch_add(1, Ordering::AcqRel) + 1;
                inner
                    .lost_events
                    .fetch_add(sample.lost_events, Ordering::AcqRel);
                inner
                    .unresolved_events
                    .fetch_add(sample.unresolved_events, Ordering::AcqRel);
                let (event, rows) = lock(&inner.accumulator).process(
                    sample,
                    generation,
                    sequence,
                    sampled_at,
                    elapsed.max(Duration::from_millis(1)),
                );
                lock(&inner.pending_rows).extend(rows);
                *lock(&inner.last_sampled_at) = Some(sampled_at);
                *lock(&inner.last_error) = None;
                *lock(&inner.collector_state) = if event.sample_state == SampleState::Gap {
                    CollectorState::Degraded
                } else {
                    CollectorState::Running
                };
                *lock(&inner.helper_state) = HelperState::Running;
                broadcast_event(&inner, event);
                if last_flush.elapsed() >= FLUSH_INTERVAL {
                    flush_pending_rows(&inner);
                    last_flush = Instant::now();
                }
            }
            Err(error) => {
                inner.enabled.store(false, Ordering::Release);
                *lock(&inner.collector_state) = CollectorState::Degraded;
                *lock(&inner.helper_state) = HelperState::Failed;
                *lock(&inner.last_error) = Some(NetworkMonitorWarning {
                    code: error_code_name(error.code).to_owned(),
                    message: Some(error.message),
                });
                collector.take();
                continue;
            }
        }
        sleep_interruptibly(
            &inner,
            sample_interval.saturating_sub(started.elapsed()),
            sample_interval_revision,
        );
    }
    drop(collector);
}

fn error_code_name(code: NetworkMonitorErrorCode) -> &'static str {
    match code {
        NetworkMonitorErrorCode::InvalidRequest => "invalidRequest",
        NetworkMonitorErrorCode::UnsupportedPlatform => "unsupportedPlatform",
        NetworkMonitorErrorCode::ElevationCancelled => "elevationCancelled",
        NetworkMonitorErrorCode::HelperDisconnected => "helperDisconnected",
        NetworkMonitorErrorCode::ProtocolMismatch => "protocolMismatch",
        NetworkMonitorErrorCode::CollectorUnavailable => "collectorUnavailable",
        NetworkMonitorErrorCode::StorageUnavailable => "storageUnavailable",
        NetworkMonitorErrorCode::Internal => "internal",
    }
}

fn broadcast_event(inner: &ManagerInner, event: NetworkRealtimeEvent) {
    {
        let mut realtime = lock(&inner.realtime);
        if realtime.len() == REALTIME_CAPACITY {
            realtime.pop_front();
        }
        realtime.push_back(event.clone());
    }
    let failed_ids = {
        let subscribers = lock(&inner.subscribers);
        subscribers
            .iter()
            .filter_map(|(id, channel)| channel.send(event.clone()).err().map(|_| *id))
            .collect::<Vec<_>>()
    };
    if !failed_ids.is_empty() {
        let mut subscribers = lock(&inner.subscribers);
        for id in failed_ids {
            if subscribers.remove(&id).is_some() {
                inner.subscriber_count.fetch_sub(1, Ordering::AcqRel);
            }
        }
    }
}

fn flush_pending_rows(inner: &ManagerInner) {
    let rows = {
        let mut pending = lock(&inner.pending_rows);
        if pending.is_empty() {
            return;
        }
        std::mem::take(&mut *pending)
    };
    let sequence = inner.sequence.load(Ordering::Acquire);
    let generation = inner.generation.load(Ordering::Acquire);
    let batch = UsageBatch {
        batch_id: format!("{}-g{generation}-s{sequence}", inner.storage_session_id),
        rows: rows.clone(),
    };
    if let Err(error) = lock(&inner.storage).write(batch) {
        let mut pending = lock(&inner.pending_rows);
        let mut retained = rows;
        retained.append(&mut *pending);
        *pending = retained;
        *lock(&inner.last_error) = Some(NetworkMonitorWarning {
            code: "storageWriteFailed".to_owned(),
            message: Some(error.message),
        });
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn sleep_interruptibly(inner: &ManagerInner, duration: Duration, sample_interval_revision: u64) {
    let deadline = Instant::now() + duration;
    while !inner.shutdown.load(Ordering::Acquire)
        && inner.enabled.load(Ordering::Acquire)
        && inner.sample_interval_revision.load(Ordering::Acquire) == sample_interval_revision
        && Instant::now() < deadline
    {
        thread::sleep(
            deadline
                .saturating_duration_since(Instant::now())
                .min(Duration::from_millis(100)),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::NetworkMonitorManager;
    use crate::network_monitor::dto::{NetworkPathFilter, NetworkUsageQuery};
    use tempfile::tempdir;

    #[test]
    fn construction_is_lazy_and_exposes_windows_only_capability() {
        let directory = tempdir().unwrap();
        let database_path = directory.path().join("network-usage.sqlite3");
        let manager = NetworkMonitorManager::new(database_path.clone());
        assert!(!manager.status().database_created);
        assert_eq!(manager.status().sample_interval_seconds, 5);
        assert_eq!(manager.capabilities().platform_supported, cfg!(windows));
        assert!(!database_path.exists());
        manager.shutdown();
    }

    #[test]
    fn sample_interval_is_validated_without_changing_generation() {
        let directory = tempdir().unwrap();
        let manager = NetworkMonitorManager::new(directory.path().join("network-usage.sqlite3"));
        let generation = manager.status().generation;
        let status = manager.set_sample_interval(10).unwrap();
        assert_eq!(status.sample_interval_seconds, 10);
        assert_eq!(status.generation, generation);
        assert!(manager.set_sample_interval(2).is_err());
        manager.shutdown();
    }

    #[test]
    fn history_query_opens_storage_even_when_monitoring_is_disabled() {
        let directory = tempdir().unwrap();
        let database_path = directory.path().join("network-usage.sqlite3");
        let manager = NetworkMonitorManager::new(database_path.clone());
        let result = manager
            .query(NetworkUsageQuery {
                from: 0,
                to: 60_000,
                network_path: NetworkPathFilter::All,
                time_zone: "UTC".to_owned(),
                limit: Some(10),
                cursor: None,
                sort_by: Default::default(),
                sort_direction: Default::default(),
            })
            .unwrap();
        assert!(result.points.is_empty());
        assert!(database_path.exists());
        manager.shutdown();
    }
}
