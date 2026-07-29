use super::accumulator::TrafficAccumulator;
use super::collector::create_platform_collector;
use super::dto::{
    AttributionQuality, CapabilityStatus, ClearNetworkUsageRequest, ClearNetworkUsageResult,
    CollectorState, NetworkMonitorCapabilities, NetworkMonitorStatus, NetworkMonitorWarning,
    NetworkRealtimeEvent, NetworkSubscription, NetworkUsagePoint, NetworkUsageQuery,
    NetworkUsageResult, QueryGroupBy, QueryInterval, SampleState, TrafficLayer, TrafficValues,
};
use super::error::{NetworkMonitorError, NetworkMonitorErrorCode, NetworkMonitorResult};
use super::storage::{StorageWorker, UsageBatch, UsageBucketDelta};
use chrono::Utc;
use std::collections::{BTreeMap, HashMap, VecDeque};
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
    shutdown: AtomicBool,
    generation: AtomicU64,
    sequence: AtomicU64,
    next_subscription_id: AtomicU64,
    subscriber_count: AtomicUsize,
    sample_interval_seconds: AtomicU64,
    sample_interval_revision: AtomicU64,
    collector_state: Mutex<CollectorState>,
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
                shutdown: AtomicBool::new(false),
                generation: AtomicU64::new(0),
                sequence: AtomicU64::new(0),
                next_subscription_id: AtomicU64::new(1),
                subscriber_count: AtomicUsize::new(0),
                sample_interval_seconds: AtomicU64::new(DEFAULT_SAMPLE_INTERVAL_SECONDS),
                sample_interval_revision: AtomicU64::new(0),
                collector_state: Mutex::new(CollectorState::Disabled),
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
        let available = CapabilityStatus {
            available: true,
            quality: AttributionQuality::Exact,
            reason: None,
        };
        let unavailable_application = CapabilityStatus {
            available: false,
            quality: AttributionQuality::Unavailable,
            reason: Some("applicationAttributionUnavailable".to_owned()),
        };
        let proxy_available = cfg!(windows);
        let route_available = cfg!(any(windows, target_os = "linux"));
        NetworkMonitorCapabilities {
            platform: std::env::consts::OS.to_owned(),
            interface_traffic: available,
            application_traffic: unavailable_application,
            proxy_configuration: CapabilityStatus {
                available: proxy_available,
                quality: if proxy_available {
                    AttributionQuality::Exact
                } else {
                    AttributionQuality::Unavailable
                },
                reason: (!proxy_available).then(|| "proxyConfigurationUnavailable".to_owned()),
            },
            vpn_detection: CapabilityStatus {
                available: true,
                quality: AttributionQuality::InterfaceOnly,
                reason: Some("vpnDetectedFromTunnelInterfaces".to_owned()),
            },
            route_mode_detection: CapabilityStatus {
                available: route_available,
                quality: if route_available {
                    AttributionQuality::InterfaceOnly
                } else {
                    AttributionQuality::Unavailable
                },
                reason: (!route_available).then(|| "routeModeUnavailable".to_owned()),
            },
            history_storage: CapabilityStatus {
                available: true,
                quality: AttributionQuality::Exact,
                reason: None,
            },
            retention_days: 7,
        }
    }

    pub fn status(&self) -> NetworkMonitorStatus {
        NetworkMonitorStatus {
            enabled: self.inner.enabled.load(Ordering::Acquire),
            collector_state: *lock(&self.inner.collector_state),
            generation: self.inner.generation.load(Ordering::Acquire),
            subscriber_count: self.inner.subscriber_count.load(Ordering::Acquire),
            sample_interval_seconds: self.inner.sample_interval_seconds.load(Ordering::Acquire),
            database_created: lock(&self.inner.storage).database_exists(),
            last_sampled_at: *lock(&self.inner.last_sampled_at),
            last_error: lock(&self.inner.last_error).clone(),
        }
    }

    pub fn set_enabled(&self, enabled: bool) -> NetworkMonitorResult<NetworkMonitorStatus> {
        if enabled {
            self.ensure_collector_thread()?;
            self.inner.enabled.store(true, Ordering::Release);
            *lock(&self.inner.collector_state) = CollectorState::Starting;
        } else {
            self.inner.enabled.store(false, Ordering::Release);
            *lock(&self.inner.collector_state) = CollectorState::Disabled;
            self.flush_pending();
            self.emit_paused();
        }
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
        let generation = self.inner.generation.load(Ordering::Acquire);
        if request.to - request.from <= 10 * 60 * 1_000
            && matches!(
                request.interval.unwrap_or(QueryInterval::Auto),
                QueryInterval::Auto | QueryInterval::Second
            )
        {
            return self.query_realtime(request, generation);
        }
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
        Ok(ClearNetworkUsageResult {
            generation,
            deleted_buckets: result.deleted_buckets,
            cleared_at: Utc::now().timestamp_millis(),
            cleared_from: result.cleared_from,
            cleared_to: result.cleared_to,
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
            device: TrafficValues::default(),
            interfaces: Vec::new(),
            applications: Vec::new(),
            proxy_vpn: Default::default(),
            warnings: Vec::new(),
        };
        broadcast_event(&self.inner, event);
    }

    fn query_realtime(
        &self,
        request: NetworkUsageQuery,
        generation: u64,
    ) -> NetworkMonitorResult<NetworkUsageResult> {
        if request.from >= request.to || request.to - request.from > 7 * 24 * 60 * 60 * 1_000 {
            return Err(NetworkMonitorError::new(
                NetworkMonitorErrorCode::InvalidRequest,
                "query range is invalid",
            ));
        }
        request.time_zone.parse::<chrono_tz::Tz>().map_err(|_| {
            NetworkMonitorError::new(
                NetworkMonitorErrorCode::InvalidRequest,
                "timeZone must be a valid IANA time zone",
            )
        })?;
        let limit = request.limit.unwrap_or(1_000);
        if limit == 0 || limit > 5_000 {
            return Err(NetworkMonitorError::new(
                NetworkMonitorErrorCode::InvalidRequest,
                "limit must be between 1 and 5000",
            ));
        }
        let offset = request
            .cursor
            .as_deref()
            .unwrap_or("0")
            .parse::<usize>()
            .map_err(|_| {
                NetworkMonitorError::new(
                    NetworkMonitorErrorCode::InvalidRequest,
                    "cursor must be a non-negative integer",
                )
            })?;
        let group_by = request.group_by.unwrap_or(QueryGroupBy::Time);
        let mut points: BTreeMap<(i64, String, &'static str), NetworkUsagePoint> = BTreeMap::new();
        for event in lock(&self.inner.realtime).iter().filter(|event| {
            event.generation == generation
                && event.sampled_at >= request.from
                && event.sampled_at < request.to
                && event.sample_state == SampleState::Sample
        }) {
            let bucket_start = event.sampled_at.div_euclid(1_000) * 1_000;
            let elapsed_seconds = event.elapsed_ms.unwrap_or(1_000) as f64 / 1_000.0;
            match group_by {
                QueryGroupBy::Time => {
                    add_realtime_point(
                        &mut points,
                        bucket_start,
                        "all",
                        TrafficLayer::Physical,
                        &event.device,
                        elapsed_seconds,
                        &request.layers,
                    );
                    add_realtime_point(
                        &mut points,
                        bucket_start,
                        "all",
                        TrafficLayer::Tunnel,
                        &event.proxy_vpn.traffic,
                        elapsed_seconds,
                        &request.layers,
                    );
                }
                QueryGroupBy::Interface => {
                    for interface in &event.interfaces {
                        if !request.interface_ids.is_empty()
                            && !request.interface_ids.contains(&interface.id)
                        {
                            continue;
                        }
                        add_realtime_point(
                            &mut points,
                            bucket_start,
                            &interface.id,
                            interface.layer,
                            &interface.traffic,
                            elapsed_seconds,
                            &request.layers,
                        );
                    }
                }
                QueryGroupBy::Application => {
                    for application in &event.applications {
                        if !request.application_ids.is_empty()
                            && !request
                                .application_ids
                                .contains(&application.application_id)
                        {
                            continue;
                        }
                        add_realtime_point(
                            &mut points,
                            bucket_start,
                            &application.application_id,
                            TrafficLayer::Application,
                            &application.traffic,
                            elapsed_seconds,
                            &request.layers,
                        );
                    }
                }
                QueryGroupBy::ProxySession => {
                    if event.proxy_vpn.vpn_connected
                        && (request.proxy_session_ids.is_empty()
                            || request.proxy_session_ids.iter().any(|id| id == "vpn"))
                    {
                        add_realtime_point(
                            &mut points,
                            bucket_start,
                            "vpn",
                            TrafficLayer::Tunnel,
                            &event.proxy_vpn.traffic,
                            elapsed_seconds,
                            &request.layers,
                        );
                    }
                }
            }
        }
        let all_points: Vec<_> = points.into_values().collect();
        let total_count = all_points.len() as u64;
        let partial = offset.saturating_add(limit as usize) < all_points.len();
        let result_points = all_points
            .into_iter()
            .skip(offset)
            .take(limit as usize)
            .collect::<Vec<_>>();
        let next_cursor = partial.then(|| (offset + result_points.len()).to_string());
        Ok(NetworkUsageResult {
            generation,
            requested_from: request.from,
            requested_to: request.to,
            actual_from: request.from.div_euclid(1_000) * 1_000,
            actual_to: ((request.to + 999).div_euclid(1_000)) * 1_000,
            interval: QueryInterval::Second,
            points: result_points,
            total_count,
            next_cursor,
            partial,
            warnings: Vec::new(),
        })
    }
}

fn collector_loop(inner: Arc<ManagerInner>) {
    let mut collector = create_platform_collector();
    let mut last_flush = Instant::now();
    let mut retry_count = 0_u32;
    while !inner.shutdown.load(Ordering::Acquire) {
        if !inner.enabled.load(Ordering::Acquire) {
            thread::sleep(Duration::from_millis(200));
            continue;
        }

        let started = Instant::now();
        let sample_interval =
            Duration::from_secs(inner.sample_interval_seconds.load(Ordering::Acquire));
        let sample_interval_revision = inner.sample_interval_revision.load(Ordering::Acquire);
        match collector.collect() {
            Ok(sample) => {
                retry_count = 0;
                let sampled_at = Utc::now().timestamp_millis();
                let generation = inner.generation.load(Ordering::Acquire);
                let sequence = inner.sequence.fetch_add(1, Ordering::AcqRel) + 1;
                let (event, rows) = lock(&inner.accumulator).process(
                    sample,
                    generation,
                    sequence,
                    sampled_at,
                    started,
                    sample_interval,
                );
                lock(&inner.pending_rows).extend(rows);
                *lock(&inner.last_sampled_at) = Some(sampled_at);
                *lock(&inner.last_error) = None;
                *lock(&inner.collector_state) = CollectorState::Running;
                broadcast_event(&inner, event);
                if last_flush.elapsed() >= FLUSH_INTERVAL {
                    flush_pending_rows(&inner);
                    last_flush = Instant::now();
                }
            }
            Err(error) => {
                retry_count = retry_count.saturating_add(1);
                *lock(&inner.collector_state) = CollectorState::Degraded;
                *lock(&inner.last_error) = Some(NetworkMonitorWarning {
                    code: format!("{:?}", error.code),
                    message: Some(error.message),
                });
            }
        }

        let backoff = if retry_count == 0 {
            sample_interval
        } else {
            Duration::from_secs(2_u64.saturating_pow(retry_count.min(3)))
        };
        sleep_interruptibly(
            &inner,
            backoff.saturating_sub(started.elapsed()),
            sample_interval_revision,
        );
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

fn add_realtime_point(
    points: &mut BTreeMap<(i64, String, &'static str), NetworkUsagePoint>,
    bucket_start: i64,
    group_id: &str,
    layer: TrafficLayer,
    traffic: &TrafficValues,
    elapsed_seconds: f64,
    requested_layers: &[TrafficLayer],
) {
    if !requested_layers.is_empty() && !requested_layers.contains(&layer) {
        return;
    }
    let download_bytes = traffic
        .download_bytes_per_second
        .map(|value| (value * elapsed_seconds).round().max(0.0) as u64);
    let upload_bytes = traffic
        .upload_bytes_per_second
        .map(|value| (value * elapsed_seconds).round().max(0.0) as u64);
    if download_bytes.is_none() && upload_bytes.is_none() {
        return;
    }
    let key = (bucket_start, group_id.to_owned(), layer.as_str());
    let point = points.entry(key).or_insert(NetworkUsagePoint {
        from: bucket_start,
        to: bucket_start + 1_000,
        group_id: group_id.to_owned(),
        layer,
        download_bytes: 0,
        upload_bytes: 0,
        quality: AttributionQuality::Exact,
    });
    point.download_bytes = point
        .download_bytes
        .saturating_add(download_bytes.unwrap_or_default());
    point.upload_bytes = point
        .upload_bytes
        .saturating_add(upload_bytes.unwrap_or_default());
}

#[cfg(test)]
mod tests {
    use super::{lock, NetworkMonitorManager};
    use crate::network_monitor::dto::{
        NetworkRealtimeEvent, NetworkUsageQuery, ProxyVpnSnapshot, QueryGroupBy, QueryInterval,
        SampleState, TrafficLayer, TrafficValues,
    };
    use tempfile::tempdir;

    #[test]
    fn manager_construction_does_not_create_the_database() {
        let directory = tempdir().unwrap();
        let database_path = directory.path().join("network-usage.sqlite3");
        let manager = NetworkMonitorManager::new(database_path.clone());
        assert!(!manager.status().database_created);
        assert_eq!(manager.status().sample_interval_seconds, 5);
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
        assert_eq!(manager.status().sample_interval_seconds, 10);
        manager.shutdown();
    }

    #[test]
    fn realtime_query_reports_total_count_before_cursor_pagination() {
        let directory = tempdir().unwrap();
        let manager = NetworkMonitorManager::new(directory.path().join("network-usage.sqlite3"));
        for (sequence, sampled_at) in [(1, 1_000), (2, 2_000)] {
            lock(&manager.inner.realtime).push_back(NetworkRealtimeEvent {
                generation: 0,
                sequence,
                sampled_at,
                elapsed_ms: Some(1_000),
                sample_state: SampleState::Sample,
                device: TrafficValues {
                    download_bytes_per_second: Some(100.0),
                    upload_bytes_per_second: Some(20.0),
                    ..TrafficValues::default()
                },
                interfaces: Vec::new(),
                applications: Vec::new(),
                proxy_vpn: ProxyVpnSnapshot::default(),
                warnings: Vec::new(),
            });
        }
        let request = NetworkUsageQuery {
            from: 0,
            to: 5_000,
            interval: Some(QueryInterval::Second),
            group_by: Some(QueryGroupBy::Time),
            interface_ids: Vec::new(),
            application_ids: Vec::new(),
            proxy_session_ids: Vec::new(),
            layers: vec![TrafficLayer::Physical],
            time_zone: "UTC".to_owned(),
            limit: Some(1),
            cursor: None,
        };

        let first = manager.query(request.clone()).unwrap();
        assert_eq!(first.total_count, 2);
        assert_eq!(first.points.len(), 1);
        assert_eq!(first.next_cursor.as_deref(), Some("1"));

        let second = manager
            .query(NetworkUsageQuery {
                cursor: Some("1".to_owned()),
                ..request
            })
            .unwrap();
        assert_eq!(second.total_count, 2);
        assert_eq!(second.points[0].from, 2_000);
        manager.shutdown();
    }
}
