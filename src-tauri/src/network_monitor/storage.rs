use super::dto::{
    AttributionQuality, ClearNetworkUsageRequest, ClearScope, NetworkMonitorWarning,
    NetworkUsagePoint, NetworkUsageQuery, NetworkUsageResult, QueryGroupBy, QueryInterval,
    TrafficLayer,
};
use super::error::{NetworkMonitorError, NetworkMonitorErrorCode, NetworkMonitorResult};
use chrono::Utc;
use chrono_tz::Tz;
#[cfg(test)]
use rusqlite::OptionalExtension;
use rusqlite::{params, Connection};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const SCHEMA_VERSION: i64 = 1;
const MAX_QUERY_SPAN_MS: i64 = 7 * 24 * 60 * 60 * 1_000;
const RETENTION_MS: i64 = MAX_QUERY_SPAN_MS;
const DEFAULT_LIMIT: u32 = 1_000;
const HARD_LIMIT: u32 = 5_000;

#[derive(Debug, Clone)]
pub struct UsageBucketDelta {
    pub bucket_start: i64,
    pub interval_seconds: u32,
    pub layer: TrafficLayer,
    pub interface_id: String,
    pub interface_name: String,
    pub application_id: String,
    pub proxy_session_id: String,
    pub download_bytes: u64,
    pub upload_bytes: u64,
    pub quality: AttributionQuality,
}

#[derive(Debug, Clone)]
pub struct UsageBatch {
    pub batch_id: String,
    pub rows: Vec<UsageBucketDelta>,
}

pub struct StorageWorker {
    sender: SyncSender<StorageCommand>,
    handle: Option<JoinHandle<()>>,
    path: PathBuf,
}

enum StorageCommand {
    Write {
        batch: UsageBatch,
        reply: mpsc::Sender<NetworkMonitorResult<()>>,
    },
    Query {
        request: NetworkUsageQuery,
        generation: u64,
        reply: mpsc::Sender<NetworkMonitorResult<NetworkUsageResult>>,
    },
    Clear {
        request: ClearNetworkUsageRequest,
        reply: mpsc::Sender<NetworkMonitorResult<StorageClearResult>>,
    },
    Shutdown {
        reply: mpsc::Sender<()>,
    },
}

#[derive(Debug)]
pub struct StorageClearResult {
    pub deleted_buckets: usize,
    pub cleared_from: Option<i64>,
    pub cleared_to: Option<i64>,
}

impl StorageWorker {
    pub fn new(path: PathBuf) -> Self {
        let (sender, receiver) = mpsc::sync_channel(32);
        let worker_path = path.clone();
        let handle = thread::Builder::new()
            .name("network-storage".to_owned())
            .spawn(move || run_storage_actor(worker_path, receiver))
            .expect("failed to spawn network storage thread");
        Self {
            sender,
            handle: Some(handle),
            path,
        }
    }

    pub fn database_exists(&self) -> bool {
        self.path.exists()
    }

    pub fn write(&self, batch: UsageBatch) -> NetworkMonitorResult<()> {
        let (reply, receiver) = mpsc::channel();
        self.sender
            .send(StorageCommand::Write { batch, reply })
            .map_err(|_| storage_worker_stopped())?;
        receiver.recv().map_err(|_| storage_worker_stopped())?
    }

    pub fn query(
        &self,
        request: NetworkUsageQuery,
        generation: u64,
    ) -> NetworkMonitorResult<NetworkUsageResult> {
        let (reply, receiver) = mpsc::channel();
        self.sender
            .send(StorageCommand::Query {
                request,
                generation,
                reply,
            })
            .map_err(|_| storage_worker_stopped())?;
        receiver.recv().map_err(|_| storage_worker_stopped())?
    }

    pub fn clear(
        &self,
        request: ClearNetworkUsageRequest,
    ) -> NetworkMonitorResult<StorageClearResult> {
        let (reply, receiver) = mpsc::channel();
        self.sender
            .send(StorageCommand::Clear { request, reply })
            .map_err(|_| storage_worker_stopped())?;
        receiver.recv().map_err(|_| storage_worker_stopped())?
    }

    pub fn shutdown(&mut self, timeout: Duration) {
        let (reply, receiver) = mpsc::channel();
        if self.sender.send(StorageCommand::Shutdown { reply }).is_ok() {
            let _ = receiver.recv_timeout(timeout);
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for StorageWorker {
    fn drop(&mut self) {
        self.shutdown(Duration::from_secs(3));
    }
}

fn run_storage_actor(path: PathBuf, receiver: Receiver<StorageCommand>) {
    let mut connection: Option<Connection> = None;
    let mut last_maintenance_at = 0_i64;
    while let Ok(command) = receiver.recv() {
        match command {
            StorageCommand::Write { batch, reply } => {
                let result = connection_for(&path, &mut connection)
                    .and_then(|database| write_batch(database, batch))
                    .and_then(|_| {
                        let now = Utc::now().timestamp_millis();
                        if now - last_maintenance_at >= 24 * 60 * 60 * 1_000 {
                            maintain_history(
                                connection.as_mut().expect("connection initialized"),
                                now,
                            )?;
                            last_maintenance_at = now;
                        }
                        Ok(())
                    });
                let _ = reply.send(result);
            }
            StorageCommand::Query {
                request,
                generation,
                reply,
            } => {
                let result = connection_for(&path, &mut connection)
                    .and_then(|database| query_usage(database, request, generation));
                let _ = reply.send(result);
            }
            StorageCommand::Clear { request, reply } => {
                let result = connection_for(&path, &mut connection)
                    .and_then(|database| clear_usage(database, request));
                let _ = reply.send(result);
            }
            StorageCommand::Shutdown { reply } => {
                if let Some(database) = connection.take() {
                    let _ = database.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
                }
                let _ = reply.send(());
                break;
            }
        }
    }
}

fn connection_for<'a>(
    path: &Path,
    connection: &'a mut Option<Connection>,
) -> NetworkMonitorResult<&'a mut Connection> {
    if connection.is_none() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(NetworkMonitorError::storage)?;
        }
        let database = Connection::open(path).map_err(NetworkMonitorError::storage)?;
        configure_and_migrate(&database)?;
        *connection = Some(database);
    }
    connection.as_mut().ok_or_else(storage_worker_stopped)
}

fn configure_and_migrate(database: &Connection) -> NetworkMonitorResult<()> {
    database
        .busy_timeout(Duration::from_secs(3))
        .map_err(NetworkMonitorError::storage)?;
    database
        .execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA foreign_keys=ON;",
        )
        .map_err(NetworkMonitorError::storage)?;
    let version: i64 = database
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(NetworkMonitorError::storage)?;
    if version > SCHEMA_VERSION {
        return Err(NetworkMonitorError::new(
            NetworkMonitorErrorCode::StorageUnavailable,
            format!("unsupported database schema version {version}"),
        ));
    }
    if version == 0 {
        database
            .execute_batch(
                "BEGIN IMMEDIATE;
                 CREATE TABLE application_identity (
                   id TEXT PRIMARY KEY NOT NULL,
                   display_name TEXT NOT NULL,
                   created_at INTEGER NOT NULL
                 );
                 CREATE TABLE network_interface (
                   id TEXT PRIMARY KEY NOT NULL,
                   display_name TEXT NOT NULL,
                   created_at INTEGER NOT NULL
                 );
                 CREATE TABLE proxy_session (
                   id TEXT PRIMARY KEY NOT NULL,
                   kind TEXT NOT NULL,
                   created_at INTEGER NOT NULL
                 );
                 CREATE TABLE usage_bucket (
                   bucket_start INTEGER NOT NULL,
                   interval_seconds INTEGER NOT NULL,
                   layer TEXT NOT NULL,
                   interface_id TEXT NOT NULL,
                   application_id TEXT NOT NULL,
                   proxy_session_id TEXT NOT NULL,
                   download_bytes INTEGER NOT NULL,
                   upload_bytes INTEGER NOT NULL,
                   quality TEXT NOT NULL,
                   PRIMARY KEY (
                     bucket_start, interval_seconds, layer, interface_id,
                     application_id, proxy_session_id
                   ),
                   FOREIGN KEY(interface_id) REFERENCES network_interface(id),
                   FOREIGN KEY(application_id) REFERENCES application_identity(id),
                   FOREIGN KEY(proxy_session_id) REFERENCES proxy_session(id)
                 );
                 CREATE INDEX usage_bucket_range_idx
                   ON usage_bucket(bucket_start, interval_seconds, layer);
                 CREATE TABLE committed_batch (
                   batch_id TEXT PRIMARY KEY NOT NULL,
                   committed_at INTEGER NOT NULL
                 );
                 INSERT INTO application_identity(id, display_name, created_at)
                   VALUES ('0', 'Unavailable', 0);
                 INSERT INTO network_interface(id, display_name, created_at)
                   VALUES ('0', 'Unspecified', 0);
                 INSERT INTO proxy_session(id, kind, created_at)
                   VALUES ('0', 'none', 0);
                 PRAGMA user_version=1;
                 COMMIT;",
            )
            .map_err(NetworkMonitorError::storage)?;
    }
    Ok(())
}

fn write_batch(database: &mut Connection, batch: UsageBatch) -> NetworkMonitorResult<()> {
    let transaction = database
        .transaction()
        .map_err(NetworkMonitorError::storage)?;
    let inserted = transaction
        .execute(
            "INSERT OR IGNORE INTO committed_batch(batch_id, committed_at) VALUES (?1, ?2)",
            params![batch.batch_id, Utc::now().timestamp_millis()],
        )
        .map_err(NetworkMonitorError::storage)?;
    if inserted == 0 {
        transaction.commit().map_err(NetworkMonitorError::storage)?;
        return Ok(());
    }

    for row in batch.rows {
        transaction
            .execute(
                "INSERT OR IGNORE INTO network_interface(id, display_name, created_at)
                 VALUES (?1, ?2, ?3)",
                params![
                    row.interface_id,
                    row.interface_name,
                    Utc::now().timestamp_millis()
                ],
            )
            .map_err(NetworkMonitorError::storage)?;
        transaction
            .execute(
                "INSERT INTO usage_bucket(
                   bucket_start, interval_seconds, layer, interface_id, application_id,
                   proxy_session_id, download_bytes, upload_bytes, quality
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(
                   bucket_start, interval_seconds, layer, interface_id,
                   application_id, proxy_session_id
                 ) DO UPDATE SET
                   download_bytes = download_bytes + excluded.download_bytes,
                   upload_bytes = upload_bytes + excluded.upload_bytes,
                   quality = excluded.quality",
                params![
                    row.bucket_start,
                    row.interval_seconds,
                    row.layer.as_str(),
                    row.interface_id,
                    row.application_id,
                    row.proxy_session_id,
                    to_sql_integer(row.download_bytes),
                    to_sql_integer(row.upload_bytes),
                    row.quality.as_str(),
                ],
            )
            .map_err(NetworkMonitorError::storage)?;
    }
    transaction.commit().map_err(NetworkMonitorError::storage)
}

fn query_usage(
    database: &Connection,
    request: NetworkUsageQuery,
    generation: u64,
) -> NetworkMonitorResult<NetworkUsageResult> {
    validate_query(&request)?;
    let now = Utc::now().timestamp_millis();
    let interval = resolve_interval(
        request.interval.unwrap_or(QueryInterval::Auto),
        request.to - request.from,
    );
    let interval_ms = interval_milliseconds(interval);
    let actual_from = request.from.div_euclid(interval_ms) * interval_ms;
    let actual_to = ceil_to(request.to, interval_ms);
    let limit = request.limit.unwrap_or(DEFAULT_LIMIT).min(HARD_LIMIT) as usize;
    let offset = request
        .cursor
        .as_deref()
        .unwrap_or("0")
        .parse::<usize>()
        .map_err(|_| invalid_request("cursor must be a non-negative integer"))?;

    let mut statement = database
        .prepare(
            "SELECT bucket_start, interval_seconds, layer, interface_id,
                    application_id, proxy_session_id, download_bytes,
                    upload_bytes, quality
             FROM usage_bucket
             WHERE bucket_start >= ?1 AND bucket_start < ?2
             ORDER BY bucket_start ASC",
        )
        .map_err(NetworkMonitorError::storage)?;
    let rows = statement
        .query_map(params![actual_from, actual_to], |row| {
            Ok(StoredRow {
                bucket_start: row.get(0)?,
                interval_seconds: row.get(1)?,
                layer: parse_layer(row.get::<_, String>(2)?.as_str()),
                interface_id: row.get(3)?,
                application_id: row.get(4)?,
                proxy_session_id: row.get(5)?,
                download_bytes: row.get::<_, i64>(6)?.max(0) as u64,
                upload_bytes: row.get::<_, i64>(7)?.max(0) as u64,
                quality: parse_quality(row.get::<_, String>(8)?.as_str()),
            })
        })
        .map_err(NetworkMonitorError::storage)?;

    let group_by = request.group_by.unwrap_or(QueryGroupBy::Time);
    let mut grouped: BTreeMap<(i64, String, &'static str), NetworkUsagePoint> = BTreeMap::new();
    for row in rows {
        let row = row.map_err(NetworkMonitorError::storage)?;
        if !request.layers.is_empty() && !request.layers.contains(&row.layer) {
            continue;
        }
        if !request.interface_ids.is_empty() && !request.interface_ids.contains(&row.interface_id) {
            continue;
        }
        if !request.application_ids.is_empty()
            && !request.application_ids.contains(&row.application_id)
        {
            continue;
        }
        if !request.proxy_session_ids.is_empty()
            && !request.proxy_session_ids.contains(&row.proxy_session_id)
        {
            continue;
        }
        let bucket_start = row.bucket_start.div_euclid(interval_ms) * interval_ms;
        let group_id = match group_by {
            QueryGroupBy::Time => "all".to_owned(),
            QueryGroupBy::Interface => row.interface_id,
            QueryGroupBy::Application => row.application_id,
            QueryGroupBy::ProxySession => row.proxy_session_id,
        };
        let key = (bucket_start, group_id.clone(), row.layer.as_str());
        let point = grouped.entry(key).or_insert(NetworkUsagePoint {
            from: bucket_start,
            to: bucket_start + interval_ms,
            group_id,
            layer: row.layer,
            download_bytes: 0,
            upload_bytes: 0,
            quality: row.quality,
        });
        point.download_bytes = point.download_bytes.saturating_add(row.download_bytes);
        point.upload_bytes = point.upload_bytes.saturating_add(row.upload_bytes);
    }

    let all_points: Vec<_> = grouped.into_values().collect();
    let total_count = all_points.len() as u64;
    let partial = offset.saturating_add(limit) < all_points.len();
    let points = all_points
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect::<Vec<_>>();
    let next_cursor = partial.then(|| (offset + points.len()).to_string());
    let mut warnings = Vec::new();
    if request.from < now - RETENTION_MS {
        warnings.push(NetworkMonitorWarning::new("queryBeforeRetentionWindow"));
    }

    Ok(NetworkUsageResult {
        generation,
        requested_from: request.from,
        requested_to: request.to,
        actual_from,
        actual_to,
        interval,
        points,
        total_count,
        next_cursor,
        partial,
        warnings,
    })
}

fn clear_usage(
    database: &mut Connection,
    request: ClearNetworkUsageRequest,
) -> NetworkMonitorResult<StorageClearResult> {
    let transaction = database
        .transaction()
        .map_err(NetworkMonitorError::storage)?;
    let (deleted_buckets, cleared_from, cleared_to) = match request.scope {
        ClearScope::All => (
            transaction
                .execute("DELETE FROM usage_bucket", [])
                .map_err(NetworkMonitorError::storage)?,
            None,
            None,
        ),
        ClearScope::Application => {
            let application_id = request
                .application_id
                .as_deref()
                .ok_or_else(|| invalid_request("applicationId is required"))?;
            (
                transaction
                    .execute(
                        "DELETE FROM usage_bucket WHERE application_id = ?1",
                        params![application_id],
                    )
                    .map_err(NetworkMonitorError::storage)?,
                None,
                None,
            )
        }
        ClearScope::TimeRange => {
            let from = request
                .from
                .ok_or_else(|| invalid_request("from is required"))?;
            let to = request
                .to
                .ok_or_else(|| invalid_request("to is required"))?;
            if from >= to {
                return Err(invalid_request("from must be before to"));
            }
            let normalized_from = from.div_euclid(60_000) * 60_000;
            let normalized_to = ceil_to(to, 60_000);
            (
                transaction
                    .execute(
                        "DELETE FROM usage_bucket
                         WHERE bucket_start >= ?1 AND bucket_start < ?2",
                        params![normalized_from, normalized_to],
                    )
                    .map_err(NetworkMonitorError::storage)?,
                Some(normalized_from),
                Some(normalized_to),
            )
        }
    };
    transaction.commit().map_err(NetworkMonitorError::storage)?;
    Ok(StorageClearResult {
        deleted_buckets,
        cleared_from,
        cleared_to,
    })
}

fn maintain_history(database: &mut Connection, now: i64) -> NetworkMonitorResult<()> {
    let transaction = database
        .transaction()
        .map_err(NetworkMonitorError::storage)?;
    let five_minute_cutoff = now - 24 * 60 * 60 * 1_000;
    let retention_cutoff = now - RETENTION_MS;
    transaction
        .execute(
            "INSERT INTO usage_bucket(
               bucket_start, interval_seconds, layer, interface_id, application_id,
               proxy_session_id, download_bytes, upload_bytes, quality
             )
             SELECT (bucket_start / 300000) * 300000, 300, layer, interface_id,
                    application_id, proxy_session_id, SUM(download_bytes),
                    SUM(upload_bytes), MIN(quality)
             FROM usage_bucket
             WHERE interval_seconds = 60 AND bucket_start >= ?1 AND bucket_start < ?2
             GROUP BY (bucket_start / 300000), layer, interface_id,
                      application_id, proxy_session_id
             ON CONFLICT(
               bucket_start, interval_seconds, layer, interface_id,
               application_id, proxy_session_id
             ) DO UPDATE SET
               download_bytes = excluded.download_bytes,
               upload_bytes = excluded.upload_bytes,
               quality = excluded.quality",
            params![retention_cutoff, five_minute_cutoff],
        )
        .map_err(NetworkMonitorError::storage)?;
    transaction
        .execute(
            "DELETE FROM usage_bucket
             WHERE interval_seconds = 60 AND bucket_start < ?1",
            params![five_minute_cutoff],
        )
        .map_err(NetworkMonitorError::storage)?;
    transaction
        .execute(
            "DELETE FROM usage_bucket WHERE bucket_start < ?1",
            params![retention_cutoff],
        )
        .map_err(NetworkMonitorError::storage)?;
    transaction
        .execute(
            "DELETE FROM committed_batch WHERE committed_at < ?1",
            params![retention_cutoff],
        )
        .map_err(NetworkMonitorError::storage)?;
    transaction.commit().map_err(NetworkMonitorError::storage)
}

#[derive(Debug)]
struct StoredRow {
    bucket_start: i64,
    #[allow(dead_code)]
    interval_seconds: u32,
    layer: TrafficLayer,
    interface_id: String,
    application_id: String,
    proxy_session_id: String,
    download_bytes: u64,
    upload_bytes: u64,
    quality: AttributionQuality,
}

fn validate_query(request: &NetworkUsageQuery) -> NetworkMonitorResult<()> {
    if request.from >= request.to {
        return Err(invalid_request("from must be before to"));
    }
    if request.to - request.from > MAX_QUERY_SPAN_MS {
        return Err(invalid_request("query span cannot exceed seven days"));
    }
    if request
        .limit
        .is_some_and(|limit| limit == 0 || limit > HARD_LIMIT)
    {
        return Err(invalid_request("limit must be between 1 and 5000"));
    }
    request
        .time_zone
        .parse::<Tz>()
        .map_err(|_| invalid_request("timeZone must be a valid IANA time zone"))?;
    Ok(())
}

fn resolve_interval(requested: QueryInterval, span: i64) -> QueryInterval {
    match requested {
        QueryInterval::Auto if span <= 10 * 60 * 1_000 => QueryInterval::Second,
        QueryInterval::Auto if span <= 24 * 60 * 60 * 1_000 => QueryInterval::Minute,
        QueryInterval::Auto => QueryInterval::FiveMinutes,
        value => value,
    }
}

fn interval_milliseconds(interval: QueryInterval) -> i64 {
    match interval {
        QueryInterval::Second => 1_000,
        QueryInterval::Minute | QueryInterval::Auto => 60_000,
        QueryInterval::FiveMinutes => 300_000,
    }
}

fn ceil_to(value: i64, interval: i64) -> i64 {
    value.div_euclid(interval) * interval
        + if value.rem_euclid(interval) == 0 {
            0
        } else {
            interval
        }
}

fn parse_layer(value: &str) -> TrafficLayer {
    match value {
        "tunnel" => TrafficLayer::Tunnel,
        "application" => TrafficLayer::Application,
        _ => TrafficLayer::Physical,
    }
}

fn parse_quality(value: &str) -> AttributionQuality {
    match value {
        "exact" => AttributionQuality::Exact,
        "interfaceOnly" => AttributionQuality::InterfaceOnly,
        _ => AttributionQuality::Unavailable,
    }
}

fn invalid_request(message: impl Into<String>) -> NetworkMonitorError {
    NetworkMonitorError::new(NetworkMonitorErrorCode::InvalidRequest, message)
}

fn storage_worker_stopped() -> NetworkMonitorError {
    NetworkMonitorError::new(
        NetworkMonitorErrorCode::StorageUnavailable,
        "network storage worker stopped",
    )
}

fn to_sql_integer(value: u64) -> i64 {
    value.min(i64::MAX as u64) as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn test_batch(id: &str) -> UsageBatch {
        UsageBatch {
            batch_id: id.to_owned(),
            rows: vec![UsageBucketDelta {
                bucket_start: 60_000,
                interval_seconds: 60,
                layer: TrafficLayer::Physical,
                interface_id: "interface".to_owned(),
                interface_name: "Test interface".to_owned(),
                application_id: "0".to_owned(),
                proxy_session_id: "0".to_owned(),
                download_bytes: 100,
                upload_bytes: 20,
                quality: AttributionQuality::Exact,
            }],
        }
    }

    fn query() -> NetworkUsageQuery {
        NetworkUsageQuery {
            from: 0,
            to: 120_000,
            interval: Some(QueryInterval::Minute),
            group_by: Some(QueryGroupBy::Time),
            interface_ids: Vec::new(),
            application_ids: Vec::new(),
            proxy_session_ids: Vec::new(),
            layers: Vec::new(),
            time_zone: "Asia/Shanghai".to_owned(),
            limit: None,
            cursor: None,
        }
    }

    #[test]
    fn migration_creates_sentinel_dimensions_and_idempotent_batches() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("usage.sqlite3");
        let mut database = Connection::open(path).unwrap();
        configure_and_migrate(&database).unwrap();
        write_batch(&mut database, test_batch("stable-batch")).unwrap();
        write_batch(&mut database, test_batch("stable-batch")).unwrap();
        let result = query_usage(&database, query(), 3).unwrap();
        assert_eq!(result.points.len(), 1);
        assert_eq!(result.points[0].download_bytes, 100);
        let sentinel: Option<String> = database
            .query_row(
                "SELECT id FROM application_identity WHERE id = '0'",
                [],
                |row| row.get(0),
            )
            .optional()
            .unwrap();
        assert_eq!(sentinel.as_deref(), Some("0"));
    }

    #[test]
    fn clear_time_range_returns_normalized_bucket_boundaries() {
        let database = Connection::open_in_memory().unwrap();
        configure_and_migrate(&database).unwrap();
        let mut database = database;
        write_batch(&mut database, test_batch("clear-batch")).unwrap();
        let result = clear_usage(
            &mut database,
            ClearNetworkUsageRequest {
                scope: ClearScope::TimeRange,
                application_id: None,
                from: Some(60_001),
                to: Some(61_000),
            },
        )
        .unwrap();
        assert_eq!(result.cleared_from, Some(60_000));
        assert_eq!(result.cleared_to, Some(120_000));
        assert_eq!(result.deleted_buckets, 1);
    }

    #[test]
    fn invalid_timezone_and_excessive_span_are_rejected() {
        let database = Connection::open_in_memory().unwrap();
        configure_and_migrate(&database).unwrap();
        let mut request = query();
        request.time_zone = "not-a-zone".to_owned();
        assert!(query_usage(&database, request, 0).is_err());

        let mut request = query();
        request.to = MAX_QUERY_SPAN_MS + 1;
        assert!(query_usage(&database, request, 0).is_err());
    }

    #[test]
    fn query_reports_total_count_before_cursor_pagination() {
        let database = Connection::open_in_memory().unwrap();
        configure_and_migrate(&database).unwrap();
        let mut database = database;
        write_batch(&mut database, test_batch("page-one")).unwrap();
        let mut second_batch = test_batch("page-two");
        second_batch.rows[0].bucket_start = 120_000;
        write_batch(&mut database, second_batch).unwrap();

        let mut first_request = query();
        first_request.to = 180_000;
        first_request.limit = Some(1);
        let first = query_usage(&database, first_request.clone(), 0).unwrap();
        assert_eq!(first.total_count, 2);
        assert_eq!(first.points.len(), 1);
        assert_eq!(first.next_cursor.as_deref(), Some("1"));

        first_request.cursor = Some("1".to_owned());
        let second = query_usage(&database, first_request, 0).unwrap();
        assert_eq!(second.total_count, 2);
        assert_eq!(second.points[0].from, 120_000);
    }
}
