use super::dto::{
    ApplicationTrafficSnapshot, AttributionQuality, NetworkPath, NetworkRealtimeEvent,
    RawApplicationSample, SampleState, TrafficValues,
};
use super::storage::UsageBucketDelta;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone)]
struct SessionTotals {
    display_name: String,
    download_bytes: u64,
    upload_bytes: u64,
    quality: AttributionQuality,
}

#[derive(Debug, Clone)]
struct CurrentDelta {
    display_name: String,
    download_bytes: u64,
    upload_bytes: u64,
    quality: AttributionQuality,
}

#[derive(Default)]
pub struct TrafficAccumulator {
    session_totals: HashMap<(String, NetworkPath), SessionTotals>,
}

impl TrafficAccumulator {
    pub fn reset(&mut self) {
        self.session_totals.clear();
    }

    pub fn process(
        &mut self,
        sample: RawApplicationSample,
        generation: u64,
        sequence: u64,
        sampled_at: i64,
        elapsed: Duration,
    ) -> (NetworkRealtimeEvent, Vec<UsageBucketDelta>) {
        let seconds = elapsed.as_secs_f64().max(0.001);
        let partial = sample.lost_events > 0 || sample.unresolved_events > 0;
        let sample_state = if partial {
            SampleState::Gap
        } else {
            SampleState::Sample
        };
        let mut current = HashMap::<(String, NetworkPath), CurrentDelta>::new();

        for delta in sample.applications {
            let key = (delta.application_id, delta.network_path);
            let aggregate = current.entry(key).or_insert_with(|| CurrentDelta {
                display_name: delta.display_name.clone(),
                download_bytes: 0,
                upload_bytes: 0,
                quality: delta.quality,
            });
            aggregate.display_name = delta.display_name;
            aggregate.download_bytes = aggregate
                .download_bytes
                .saturating_add(delta.download_bytes);
            aggregate.upload_bytes = aggregate.upload_bytes.saturating_add(delta.upload_bytes);
            if delta.quality == AttributionQuality::Partial {
                aggregate.quality = AttributionQuality::Partial;
            }
        }

        for (key, delta) in &current {
            let totals = self
                .session_totals
                .entry(key.clone())
                .or_insert_with(|| SessionTotals {
                    display_name: delta.display_name.clone(),
                    download_bytes: 0,
                    upload_bytes: 0,
                    quality: delta.quality,
                });
            totals.display_name = delta.display_name.clone();
            totals.download_bytes = totals.download_bytes.saturating_add(delta.download_bytes);
            totals.upload_bytes = totals.upload_bytes.saturating_add(delta.upload_bytes);
            if delta.quality == AttributionQuality::Partial {
                totals.quality = AttributionQuality::Partial;
            }
        }

        let mut applications = Vec::with_capacity(self.session_totals.len());
        for ((application_id, network_path), totals) in &self.session_totals {
            let delta = current.get(&(application_id.clone(), *network_path));
            applications.push(ApplicationTrafficSnapshot {
                application_id: application_id.clone(),
                display_name: totals.display_name.clone(),
                network_path: *network_path,
                traffic: TrafficValues {
                    download_bytes_per_second: Some(
                        delta.map_or(0, |value| value.download_bytes) as f64 / seconds,
                    ),
                    upload_bytes_per_second: Some(
                        delta.map_or(0, |value| value.upload_bytes) as f64 / seconds,
                    ),
                    session_download_bytes: totals.download_bytes,
                    session_upload_bytes: totals.upload_bytes,
                },
                quality: if partial {
                    AttributionQuality::Partial
                } else {
                    totals.quality
                },
            });
        }

        let mut rows = Vec::with_capacity(current.len());
        for ((application_id, network_path), delta) in current {
            rows.push(UsageBucketDelta {
                bucket_start: sampled_at.div_euclid(60_000) * 60_000,
                interval_seconds: 60,
                application_id,
                display_name: delta.display_name,
                network_path,
                download_bytes: delta.download_bytes,
                upload_bytes: delta.upload_bytes,
                quality: if partial {
                    AttributionQuality::Partial
                } else {
                    delta.quality
                },
            });
        }

        applications.sort_by(|left, right| {
            let left_rate = left.traffic.download_bytes_per_second.unwrap_or_default()
                + left.traffic.upload_bytes_per_second.unwrap_or_default();
            let right_rate = right.traffic.download_bytes_per_second.unwrap_or_default()
                + right.traffic.upload_bytes_per_second.unwrap_or_default();
            right_rate
                .total_cmp(&left_rate)
                .then_with(|| left.application_id.cmp(&right.application_id))
                .then_with(|| left.network_path.as_str().cmp(right.network_path.as_str()))
        });

        let unknown_traffic = sum_unknown_traffic(&applications);
        (
            NetworkRealtimeEvent {
                generation,
                sequence,
                sampled_at,
                elapsed_ms: Some(elapsed.as_millis().min(u128::from(u64::MAX)) as u64),
                sample_state,
                applications,
                unknown_traffic,
                lost_events: sample.lost_events,
                unresolved_events: sample.unresolved_events,
                warnings: sample.warnings,
            },
            rows,
        )
    }
}

fn sum_unknown_traffic(applications: &[ApplicationTrafficSnapshot]) -> TrafficValues {
    let mut result = TrafficValues::default();
    let mut download_rate = 0.0;
    let mut upload_rate = 0.0;
    let mut has_rate = false;
    for application in applications
        .iter()
        .filter(|application| application.network_path == NetworkPath::Unknown)
    {
        if let Some(value) = application.traffic.download_bytes_per_second {
            download_rate += value;
            has_rate = true;
        }
        if let Some(value) = application.traffic.upload_bytes_per_second {
            upload_rate += value;
            has_rate = true;
        }
        result.session_download_bytes = result
            .session_download_bytes
            .saturating_add(application.traffic.session_download_bytes);
        result.session_upload_bytes = result
            .session_upload_bytes
            .saturating_add(application.traffic.session_upload_bytes);
    }
    if has_rate {
        result.download_bytes_per_second = Some(download_rate);
        result.upload_bytes_per_second = Some(upload_rate);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network_monitor::dto::{NetworkMonitorWarning, RawApplicationDelta};

    fn sample(path: NetworkPath, download: u64, upload: u64) -> RawApplicationSample {
        RawApplicationSample {
            applications: vec![RawApplicationDelta {
                application_id: "app".to_owned(),
                display_name: "app.exe".to_owned(),
                network_path: path,
                download_bytes: download,
                upload_bytes: upload,
                quality: AttributionQuality::Exact,
            }],
            lost_events: 0,
            unresolved_events: 0,
            warnings: Vec::<NetworkMonitorWarning>::new(),
        }
    }

    #[test]
    fn accumulates_each_application_path_without_cross_path_totals() {
        let mut accumulator = TrafficAccumulator::default();
        let (proxy, _) = accumulator.process(
            sample(NetworkPath::Proxy, 100, 20),
            0,
            1,
            60_000,
            Duration::from_secs(2),
        );
        let (direct, _) = accumulator.process(
            sample(NetworkPath::Direct, 40, 10),
            0,
            2,
            62_000,
            Duration::from_secs(2),
        );
        assert_eq!(
            proxy.applications[0].traffic.download_bytes_per_second,
            Some(50.0)
        );
        assert_eq!(proxy.applications[0].traffic.session_download_bytes, 100);
        assert_eq!(direct.applications[0].traffic.session_download_bytes, 40);
        let inactive_proxy = direct
            .applications
            .iter()
            .find(|application| application.network_path == NetworkPath::Proxy)
            .unwrap();
        assert_eq!(inactive_proxy.traffic.download_bytes_per_second, Some(0.0));
        assert_eq!(inactive_proxy.traffic.session_download_bytes, 100);
    }

    #[test]
    fn lost_events_mark_the_sample_partial_and_preserve_deltas() {
        let mut accumulator = TrafficAccumulator::default();
        let mut raw = sample(NetworkPath::Unknown, 80, 10);
        raw.lost_events = 2;
        let (event, rows) = accumulator.process(raw, 0, 1, 60_000, Duration::from_secs(1));
        assert_eq!(event.sample_state, SampleState::Gap);
        assert_eq!(event.applications[0].quality, AttributionQuality::Partial);
        assert_eq!(event.unknown_traffic.session_download_bytes, 80);
        assert_eq!(rows[0].download_bytes, 80);

        let (next, _) = accumulator.process(
            sample(NetworkPath::Direct, 20, 5),
            0,
            2,
            61_000,
            Duration::from_secs(1),
        );
        assert_eq!(next.unknown_traffic.download_bytes_per_second, Some(0.0));
        assert_eq!(next.unknown_traffic.session_download_bytes, 80);
    }
}
