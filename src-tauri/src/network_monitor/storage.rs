use super::dto::{
    AttributionQuality, ClearNetworkUsageRequest, ClearScope, NetworkMonitorWarning, NetworkPath,
    NetworkUsagePoint, NetworkUsageQuery, NetworkUsageResult, NetworkUsageSortBy, SortDirection,
};
use super::error::{NetworkMonitorError, NetworkMonitorErrorCode, NetworkMonitorResult};
use chrono::Utc;
use chrono_tz::Tz;
use rusqlite::{params, Connection};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

// Version 3 clears ETW-derived rows so no result combines different collection methods.
const SCHEMA_VERSION: i64 = 3;
const MAX_QUERY_SPAN_MS: i64 = 7 * 24 * 60 * 60 * 1_000;
const RETENTION_MS: i64 = MAX_QUERY_SPAN_MS;
const DEFAULT_LIMIT: u32 = 1_000;
const HARD_LIMIT: u32 = 5_000;

#[derive(Debug, Clone)]
pub struct UsageBucketDelta {
    pub bucket_start: i64,
    pub interval_seconds: u32,
    pub application_id: String,
    pub display_name: String,
    pub network_path: NetworkPath,
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
    if version == SCHEMA_VERSION {
        return Ok(());
    }

    database
        .execute_batch(
            "BEGIN IMMEDIATE;
             DROP TABLE IF EXISTS usage_bucket;
             DROP TABLE IF EXISTS network_interface;
             DROP TABLE IF EXISTS proxy_session;
             DROP TABLE IF EXISTS application_identity;
             DROP TABLE IF EXISTS committed_batch;
             CREATE TABLE application_identity (
               id TEXT PRIMARY KEY NOT NULL,
               display_name TEXT NOT NULL,
               created_at INTEGER NOT NULL,
               last_seen_at INTEGER NOT NULL
             );
             CREATE TABLE usage_bucket (
               bucket_start INTEGER NOT NULL,
               interval_seconds INTEGER NOT NULL,
               application_id TEXT NOT NULL,
               network_path TEXT NOT NULL CHECK (
                 network_path IN ('proxy', 'direct', 'unknown')
               ),
               download_bytes INTEGER NOT NULL,
               upload_bytes INTEGER NOT NULL,
               quality TEXT NOT NULL,
               PRIMARY KEY (
                 bucket_start, interval_seconds, application_id, network_path
               ),
               FOREIGN KEY(application_id) REFERENCES application_identity(id)
             );
             CREATE INDEX usage_bucket_range_path_idx
               ON usage_bucket(bucket_start, network_path, application_id);
             CREATE TABLE committed_batch (
               batch_id TEXT PRIMARY KEY NOT NULL,
               committed_at INTEGER NOT NULL
             );
             PRAGMA user_version=3;
             COMMIT;",
        )
        .map_err(NetworkMonitorError::storage)
}

fn write_batch(database: &mut Connection, batch: UsageBatch) -> NetworkMonitorResult<()> {
    let transaction = database
        .transaction()
        .map_err(NetworkMonitorError::storage)?;
    let now = Utc::now().timestamp_millis();
    let inserted = transaction
        .execute(
            "INSERT OR IGNORE INTO committed_batch(batch_id, committed_at) VALUES (?1, ?2)",
            params![batch.batch_id, now],
        )
        .map_err(NetworkMonitorError::storage)?;
    if inserted == 0 {
        transaction.commit().map_err(NetworkMonitorError::storage)?;
        return Ok(());
    }

    for row in batch.rows {
        transaction
            .execute(
                "INSERT INTO application_identity(id, display_name, created_at, last_seen_at)
                 VALUES (?1, ?2, ?3, ?3)
                 ON CONFLICT(id) DO UPDATE SET
                   display_name = excluded.display_name,
                   last_seen_at = excluded.last_seen_at",
                params![row.application_id, row.display_name, now],
            )
            .map_err(NetworkMonitorError::storage)?;
        transaction
            .execute(
                "INSERT INTO usage_bucket(
                   bucket_start, interval_seconds, application_id, network_path,
                   download_bytes, upload_bytes, quality
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(
                   bucket_start, interval_seconds, application_id, network_path
                 ) DO UPDATE SET
                   download_bytes = download_bytes + excluded.download_bytes,
                   upload_bytes = upload_bytes + excluded.upload_bytes,
                   quality = CASE
                     WHEN quality = 'partial' OR excluded.quality = 'partial'
                       THEN 'partial'
                     ELSE 'exact'
                   END",
                params![
                    row.bucket_start,
                    row.interval_seconds,
                    row.application_id,
                    row.network_path.as_str(),
                    to_sql_integer(row.download_bytes),
                    to_sql_integer(row.upload_bytes),
                    row.quality.as_str(),
                ],
            )
            .map_err(NetworkMonitorError::storage)?;
    }
    transaction.commit().map_err(NetworkMonitorError::storage)
}

#[derive(Debug)]
struct UsageAggregate {
    display_name: String,
    download_bytes: u64,
    upload_bytes: u64,
    includes_unknown: bool,
    partial: bool,
}

fn query_usage(
    database: &Connection,
    request: NetworkUsageQuery,
    generation: u64,
) -> NetworkMonitorResult<NetworkUsageResult> {
    validate_query(&request)?;
    let actual_from = request.from.div_euclid(60_000) * 60_000;
    let actual_to = ceil_to(request.to, 60_000);
    let limit = request.limit.unwrap_or(DEFAULT_LIMIT).min(HARD_LIMIT) as usize;
    let offset = request
        .cursor
        .as_deref()
        .unwrap_or("0")
        .parse::<usize>()
        .map_err(|_| invalid_request("cursor must be a non-negative integer"))?;

    let mut statement = database
        .prepare(
            "SELECT usage_bucket.application_id, application_identity.display_name,
                    usage_bucket.network_path, usage_bucket.download_bytes,
                    usage_bucket.upload_bytes, usage_bucket.quality
             FROM usage_bucket
             JOIN application_identity
               ON application_identity.id = usage_bucket.application_id
             WHERE usage_bucket.bucket_start >= ?1
               AND usage_bucket.bucket_start < ?2",
        )
        .map_err(NetworkMonitorError::storage)?;
    let rows = statement
        .query_map(params![actual_from, actual_to], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                parse_network_path(&row.get::<_, String>(2)?),
                row.get::<_, i64>(3)?.max(0) as u64,
                row.get::<_, i64>(4)?.max(0) as u64,
                row.get::<_, String>(5)?,
            ))
        })
        .map_err(NetworkMonitorError::storage)?;

    let mut grouped = BTreeMap::<String, UsageAggregate>::new();
    for row in rows {
        let (application_id, display_name, path, download, upload, quality) =
            row.map_err(NetworkMonitorError::storage)?;
        if !request.network_path.includes(path) {
            continue;
        }
        let aggregate = grouped.entry(application_id).or_insert(UsageAggregate {
            display_name,
            download_bytes: 0,
            upload_bytes: 0,
            includes_unknown: false,
            partial: false,
        });
        aggregate.download_bytes = aggregate.download_bytes.saturating_add(download);
        aggregate.upload_bytes = aggregate.upload_bytes.saturating_add(upload);
        aggregate.includes_unknown |= path == NetworkPath::Unknown;
        aggregate.partial |= quality == "partial";
    }

    let mut all_points = grouped
        .into_iter()
        .map(|(application_id, aggregate)| {
            let total_bytes = aggregate
                .download_bytes
                .saturating_add(aggregate.upload_bytes);
            NetworkUsagePoint {
                application_id,
                display_name: aggregate.display_name,
                download_bytes: aggregate.download_bytes,
                upload_bytes: aggregate.upload_bytes,
                total_bytes,
                includes_unknown: aggregate.includes_unknown,
                quality: if aggregate.partial || aggregate.includes_unknown {
                    AttributionQuality::Partial
                } else {
                    AttributionQuality::Exact
                },
            }
        })
        .collect::<Vec<_>>();
    all_points.sort_by(|left, right| {
        let comparison = match request.sort_by {
            NetworkUsageSortBy::Application => left.display_name.cmp(&right.display_name),
            NetworkUsageSortBy::Download => left.download_bytes.cmp(&right.download_bytes),
            NetworkUsageSortBy::Upload => left.upload_bytes.cmp(&right.upload_bytes),
            NetworkUsageSortBy::Total => left.total_bytes.cmp(&right.total_bytes),
        };
        let comparison = match request.sort_direction {
            SortDirection::Asc => comparison,
            SortDirection::Desc => comparison.reverse(),
        };
        comparison.then_with(|| left.application_id.cmp(&right.application_id))
    });

    let partial = all_points
        .iter()
        .any(|point| point.quality == AttributionQuality::Partial);
    let total_count = all_points.len() as u64;
    let has_more = offset.saturating_add(limit) < all_points.len();
    let points = all_points
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect::<Vec<_>>();
    let next_cursor = has_more.then(|| (offset + points.len()).to_string());
    let mut warnings = Vec::new();
    if request.from < Utc::now().timestamp_millis() - RETENTION_MS {
        warnings.push(NetworkMonitorWarning::new("queryBeforeRetentionWindow"));
    }
    if partial {
        warnings.push(NetworkMonitorWarning::new("partialApplicationData"));
    }

    Ok(NetworkUsageResult {
        generation,
        requested_from: request.from,
        requested_to: request.to,
        actual_from,
        actual_to,
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
    match request.scope {
        ClearScope::All => {}
    }
    let transaction = database
        .transaction()
        .map_err(NetworkMonitorError::storage)?;
    let deleted_buckets = transaction
        .execute("DELETE FROM usage_bucket", [])
        .map_err(NetworkMonitorError::storage)?;
    transaction
        .execute("DELETE FROM application_identity", [])
        .map_err(NetworkMonitorError::storage)?;
    transaction
        .execute("DELETE FROM committed_batch", [])
        .map_err(NetworkMonitorError::storage)?;
    transaction.commit().map_err(NetworkMonitorError::storage)?;
    Ok(StorageClearResult { deleted_buckets })
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
               bucket_start, interval_seconds, application_id, network_path,
               download_bytes, upload_bytes, quality
             )
             SELECT (bucket_start / 300000) * 300000, 300, application_id,
                    network_path, SUM(download_bytes), SUM(upload_bytes),
                    CASE WHEN SUM(quality = 'partial') > 0 THEN 'partial' ELSE 'exact' END
             FROM usage_bucket
             WHERE interval_seconds = 60 AND bucket_start >= ?1 AND bucket_start < ?2
             GROUP BY (bucket_start / 300000), application_id, network_path
             ON CONFLICT(
               bucket_start, interval_seconds, application_id, network_path
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

fn parse_network_path(value: &str) -> NetworkPath {
    match value {
        "proxy" => NetworkPath::Proxy,
        "direct" => NetworkPath::Direct,
        _ => NetworkPath::Unknown,
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
    use crate::network_monitor::dto::{
        ClearScope, NetworkPathFilter, NetworkUsageSortBy, SortDirection,
    };
    use tempfile::tempdir;

    fn row(
        application_id: &str,
        display_name: &str,
        path: NetworkPath,
        download_bytes: u64,
    ) -> UsageBucketDelta {
        UsageBucketDelta {
            bucket_start: 60_000,
            interval_seconds: 60,
            application_id: application_id.to_owned(),
            display_name: display_name.to_owned(),
            network_path: path,
            download_bytes,
            upload_bytes: 20,
            quality: AttributionQuality::Exact,
        }
    }

    fn query(path: NetworkPathFilter) -> NetworkUsageQuery {
        NetworkUsageQuery {
            from: 0,
            to: 120_000,
            network_path: path,
            time_zone: "Asia/Shanghai".to_owned(),
            limit: None,
            cursor: None,
            sort_by: Default::default(),
            sort_direction: Default::default(),
        }
    }

    #[test]
    fn v1_migration_discards_incompatible_interface_history() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("usage.sqlite3");
        let database = Connection::open(path).unwrap();
        database
            .execute_batch(
                "CREATE TABLE usage_bucket(value INTEGER);
                 INSERT INTO usage_bucket(value) VALUES (1);
                 PRAGMA user_version=1;",
            )
            .unwrap();
        configure_and_migrate(&database).unwrap();
        let version: i64 = database
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        let count: i64 = database
            .query_row("SELECT COUNT(*) FROM usage_bucket", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 3);
        assert_eq!(count, 0);
    }

    #[test]
    fn query_filters_paths_merges_applications_and_sorts_by_total() {
        let mut database = Connection::open_in_memory().unwrap();
        configure_and_migrate(&database).unwrap();
        write_batch(
            &mut database,
            UsageBatch {
                batch_id: "batch".to_owned(),
                rows: vec![
                    row("browser", "browser.exe", NetworkPath::Direct, 100),
                    row("browser", "browser.exe", NetworkPath::Proxy, 200),
                    row("sync", "sync.exe", NetworkPath::Unknown, 500),
                ],
            },
        )
        .unwrap();

        let all = query_usage(&database, query(NetworkPathFilter::All), 3).unwrap();
        assert_eq!(all.points[0].application_id, "sync");
        assert_eq!(all.points[1].download_bytes, 300);
        assert!(all.points[0].includes_unknown);

        let proxy = query_usage(&database, query(NetworkPathFilter::Proxy), 3).unwrap();
        assert_eq!(proxy.points.len(), 1);
        assert_eq!(proxy.points[0].download_bytes, 200);
        assert!(!proxy.points[0].includes_unknown);
    }

    #[test]
    fn query_sorts_the_full_aggregate_before_paging() {
        let mut database = Connection::open_in_memory().unwrap();
        configure_and_migrate(&database).unwrap();
        write_batch(
            &mut database,
            UsageBatch {
                batch_id: "sort".to_owned(),
                rows: vec![
                    row("alpha", "alpha.exe", NetworkPath::Direct, 300),
                    row("bravo", "bravo.exe", NetworkPath::Direct, 100),
                    row("charlie", "charlie.exe", NetworkPath::Direct, 200),
                ],
            },
        )
        .unwrap();

        let mut request = query(NetworkPathFilter::All);
        request.limit = Some(1);
        request.sort_by = NetworkUsageSortBy::Application;
        request.sort_direction = SortDirection::Asc;
        request.cursor = Some("1".to_owned());
        let result = query_usage(&database, request, 0).unwrap();

        assert_eq!(result.total_count, 3);
        assert_eq!(result.points[0].application_id, "bravo");
    }

    #[test]
    fn committed_batches_are_idempotent_and_clear_removes_identities() {
        let mut database = Connection::open_in_memory().unwrap();
        configure_and_migrate(&database).unwrap();
        let batch = UsageBatch {
            batch_id: "stable".to_owned(),
            rows: vec![row("app", "app.exe", NetworkPath::Direct, 100)],
        };
        write_batch(&mut database, batch.clone()).unwrap();
        write_batch(&mut database, batch).unwrap();
        let result = query_usage(&database, query(NetworkPathFilter::All), 0).unwrap();
        assert_eq!(result.points[0].download_bytes, 100);

        let cleared = clear_usage(
            &mut database,
            ClearNetworkUsageRequest {
                scope: ClearScope::All,
            },
        )
        .unwrap();
        assert_eq!(cleared.deleted_buckets, 1);
        let identity_count: i64 = database
            .query_row("SELECT COUNT(*) FROM application_identity", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(identity_count, 0);
    }

    #[test]
    fn maintenance_compresses_after_one_day_and_removes_after_seven_days() {
        const DAY: i64 = 24 * 60 * 60 * 1_000;
        let now = 10 * DAY;
        let mut database = Connection::open_in_memory().unwrap();
        configure_and_migrate(&database).unwrap();
        let mut expired = row("app", "app.exe", NetworkPath::Direct, 10);
        expired.bucket_start = now - 8 * DAY;
        let mut compressed_one = row("app", "app.exe", NetworkPath::Direct, 20);
        compressed_one.bucket_start = now - 2 * DAY;
        let mut compressed_two = row("app", "app.exe", NetworkPath::Direct, 30);
        compressed_two.bucket_start = now - 2 * DAY + 60_000;
        let mut recent = row("app", "app.exe", NetworkPath::Direct, 40);
        recent.bucket_start = now - 60 * 60 * 1_000;
        write_batch(
            &mut database,
            UsageBatch {
                batch_id: "maintenance".to_owned(),
                rows: vec![expired, compressed_one, compressed_two, recent],
            },
        )
        .unwrap();

        maintain_history(&mut database, now).unwrap();

        let one_minute: i64 = database
            .query_row(
                "SELECT COUNT(*) FROM usage_bucket WHERE interval_seconds = 60",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let five_minute: (i64, i64) = database
            .query_row(
                "SELECT COUNT(*), SUM(download_bytes)
                 FROM usage_bucket WHERE interval_seconds = 300",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        let total: i64 = database
            .query_row("SELECT COUNT(*) FROM usage_bucket", [], |row| row.get(0))
            .unwrap();
        assert_eq!(one_minute, 1);
        assert_eq!(five_minute, (1, 50));
        assert_eq!(total, 2);
    }
}
