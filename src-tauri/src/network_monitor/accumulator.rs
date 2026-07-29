use super::application::{ApplicationTrafficCollector, UnavailableApplicationTrafficCollector};
use super::dto::{
    AttributionQuality, InterfaceKind, InterfaceState, InterfaceTrafficSnapshot,
    NetworkMonitorWarning, NetworkRealtimeEvent, ProxyVpnSnapshot, RawInterfaceCounters,
    RawNetworkSample, SampleState, TrafficLayer, TrafficValues,
};
use super::storage::UsageBucketDelta;
use std::collections::HashMap;
use std::time::{Duration, Instant};

const MIN_MAX_CONTIGUOUS_GAP: Duration = Duration::from_secs(15);
const MAX_CONTIGUOUS_GAP_MULTIPLIER: u32 = 3;

#[derive(Debug, Clone)]
struct Baseline {
    received_bytes: u64,
    transmitted_bytes: u64,
    sampled_at: Instant,
    session_download_bytes: u64,
    session_upload_bytes: u64,
}

pub struct TrafficAccumulator {
    baselines: HashMap<String, Baseline>,
    application_collector: Box<dyn ApplicationTrafficCollector>,
}

impl Default for TrafficAccumulator {
    fn default() -> Self {
        Self {
            baselines: HashMap::new(),
            application_collector: Box::new(UnavailableApplicationTrafficCollector),
        }
    }
}

impl TrafficAccumulator {
    pub fn reset(&mut self) {
        self.baselines.clear();
    }

    pub fn process(
        &mut self,
        sample: RawNetworkSample,
        generation: u64,
        sequence: u64,
        sampled_at: i64,
        now: Instant,
        expected_interval: Duration,
    ) -> (NetworkRealtimeEvent, Vec<UsageBucketDelta>) {
        let mut warnings = sample.warnings;
        let mut interfaces = Vec::with_capacity(sample.interfaces.len());
        let mut deltas = Vec::new();
        let mut event_state = SampleState::Sample;
        let mut event_elapsed_ms = None;

        for counters in sample.interfaces {
            let layer = layer_for(&counters);
            let (traffic, state, interface_deltas, elapsed_ms) = self.process_interface(
                &counters,
                sampled_at,
                now,
                expected_interval,
                &mut warnings,
            );
            if let Some(elapsed_ms) = elapsed_ms {
                event_elapsed_ms = Some(event_elapsed_ms.unwrap_or(0).max(elapsed_ms));
            }
            if state == SampleState::Gap {
                event_state = SampleState::Gap;
            }
            deltas.extend(interface_deltas);
            interfaces.push(InterfaceTrafficSnapshot {
                id: counters.stable_id,
                name: counters.name,
                kind: counters.kind,
                state: counters.state,
                layer,
                is_virtual: counters.is_virtual,
                tunnel_type: counters.tunnel_type,
                traffic,
                quality: AttributionQuality::Exact,
            });
        }

        self.baselines
            .retain(|id, _| interfaces.iter().any(|interface| &interface.id == id));

        let device = sum_traffic(interfaces.iter().filter(|interface| {
            interface.layer == TrafficLayer::Physical
                && interface.state == InterfaceState::Up
                && interface.kind != InterfaceKind::Loopback
                && !interface.is_virtual
        }));
        let tunnel_interfaces: Vec<_> = interfaces
            .iter()
            .filter(|interface| {
                interface.layer == TrafficLayer::Tunnel && interface.state == InterfaceState::Up
            })
            .collect();
        let tunnel_traffic = sum_traffic(tunnel_interfaces.iter().copied());
        let vpn_connected = !tunnel_interfaces.is_empty();
        let virtual_interface_ids = tunnel_interfaces
            .iter()
            .map(|interface| interface.id.clone())
            .collect();
        let application_quality = self.application_collector.quality();
        let applications = self.application_collector.collect();
        if application_quality == AttributionQuality::Unavailable {
            warnings.push(NetworkMonitorWarning::new(
                "applicationAttributionUnavailable",
            ));
        }

        (
            NetworkRealtimeEvent {
                generation,
                sequence,
                sampled_at,
                elapsed_ms: event_elapsed_ms,
                sample_state: event_state,
                device,
                interfaces,
                applications,
                proxy_vpn: ProxyVpnSnapshot {
                    proxy_configured: sample.proxy.configured,
                    proxy_kinds: sample.proxy.kinds,
                    pac_enabled: sample.proxy.pac_enabled,
                    vpn_connected,
                    route_mode: sample.route_mode,
                    virtual_interface_ids,
                    traffic: tunnel_traffic,
                    quality: if vpn_connected {
                        AttributionQuality::Exact
                    } else {
                        AttributionQuality::Unavailable
                    },
                },
                warnings,
            },
            deltas,
        )
    }

    fn process_interface(
        &mut self,
        counters: &RawInterfaceCounters,
        sampled_at: i64,
        now: Instant,
        expected_interval: Duration,
        warnings: &mut Vec<NetworkMonitorWarning>,
    ) -> (
        TrafficValues,
        SampleState,
        Vec<UsageBucketDelta>,
        Option<u64>,
    ) {
        let Some(previous) = self.baselines.get_mut(&counters.stable_id) else {
            self.baselines.insert(
                counters.stable_id.clone(),
                Baseline {
                    received_bytes: counters.received_bytes,
                    transmitted_bytes: counters.transmitted_bytes,
                    sampled_at: now,
                    session_download_bytes: 0,
                    session_upload_bytes: 0,
                },
            );
            return (TrafficValues::default(), SampleState::Gap, Vec::new(), None);
        };

        let elapsed = now.saturating_duration_since(previous.sampled_at);
        let max_contiguous_gap = expected_interval
            .saturating_mul(MAX_CONTIGUOUS_GAP_MULTIPLIER)
            .max(MIN_MAX_CONTIGUOUS_GAP);
        let counters_rolled_back = counters.received_bytes < previous.received_bytes
            || counters.transmitted_bytes < previous.transmitted_bytes;
        if counters_rolled_back || elapsed > max_contiguous_gap || elapsed.is_zero() {
            previous.received_bytes = counters.received_bytes;
            previous.transmitted_bytes = counters.transmitted_bytes;
            previous.sampled_at = now;
            warnings.push(NetworkMonitorWarning::new(if counters_rolled_back {
                "counterReset"
            } else {
                "samplingGap"
            }));
            return (
                TrafficValues {
                    session_download_bytes: previous.session_download_bytes,
                    session_upload_bytes: previous.session_upload_bytes,
                    ..TrafficValues::default()
                },
                SampleState::Gap,
                Vec::new(),
                None,
            );
        }

        let download_delta = counters.received_bytes - previous.received_bytes;
        let upload_delta = counters.transmitted_bytes - previous.transmitted_bytes;
        previous.received_bytes = counters.received_bytes;
        previous.transmitted_bytes = counters.transmitted_bytes;
        previous.sampled_at = now;
        previous.session_download_bytes = previous
            .session_download_bytes
            .saturating_add(download_delta);
        previous.session_upload_bytes = previous.session_upload_bytes.saturating_add(upload_delta);
        let seconds = elapsed.as_secs_f64();
        let layer = layer_for(counters);

        (
            TrafficValues {
                download_bytes_per_second: Some(download_delta as f64 / seconds),
                upload_bytes_per_second: Some(upload_delta as f64 / seconds),
                session_download_bytes: previous.session_download_bytes,
                session_upload_bytes: previous.session_upload_bytes,
            },
            SampleState::Sample,
            vec![UsageBucketDelta {
                bucket_start: sampled_at.div_euclid(60_000) * 60_000,
                interval_seconds: 60,
                layer,
                interface_id: counters.stable_id.clone(),
                interface_name: counters.name.clone(),
                application_id: "0".to_owned(),
                proxy_session_id: "0".to_owned(),
                download_bytes: download_delta,
                upload_bytes: upload_delta,
                quality: AttributionQuality::Exact,
            }],
            Some(elapsed.as_millis().min(u128::from(u64::MAX)) as u64),
        )
    }
}

fn layer_for(counters: &RawInterfaceCounters) -> TrafficLayer {
    if counters.kind == InterfaceKind::Tunnel || counters.tunnel_type.is_some() {
        TrafficLayer::Tunnel
    } else {
        TrafficLayer::Physical
    }
}

fn sum_traffic<'a>(
    interfaces: impl Iterator<Item = &'a InterfaceTrafficSnapshot>,
) -> TrafficValues {
    let mut result = TrafficValues::default();
    let mut download_speed = 0.0;
    let mut upload_speed = 0.0;
    let mut has_download_speed = false;
    let mut has_upload_speed = false;
    for interface in interfaces {
        if let Some(value) = interface.traffic.download_bytes_per_second {
            download_speed += value;
            has_download_speed = true;
        }
        if let Some(value) = interface.traffic.upload_bytes_per_second {
            upload_speed += value;
            has_upload_speed = true;
        }
        result.session_download_bytes = result
            .session_download_bytes
            .saturating_add(interface.traffic.session_download_bytes);
        result.session_upload_bytes = result
            .session_upload_bytes
            .saturating_add(interface.traffic.session_upload_bytes);
    }
    result.download_bytes_per_second = has_download_speed.then_some(download_speed);
    result.upload_bytes_per_second = has_upload_speed.then_some(upload_speed);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network_monitor::dto::{RawNetworkSample, RawProxyState};

    fn interface(received: u64, transmitted: u64, kind: InterfaceKind) -> RawInterfaceCounters {
        RawInterfaceCounters {
            stable_id: "id".to_owned(),
            interface_index: 1,
            name: "adapter".to_owned(),
            kind,
            state: InterfaceState::Up,
            is_virtual: kind == InterfaceKind::Tunnel,
            tunnel_type: (kind == InterfaceKind::Tunnel).then(|| "test".to_owned()),
            received_bytes: received,
            transmitted_bytes: transmitted,
        }
    }

    fn sample(interface: RawInterfaceCounters) -> RawNetworkSample {
        RawNetworkSample {
            interfaces: vec![interface],
            proxy: RawProxyState::default(),
            route_mode: None,
            warnings: Vec::new(),
        }
    }

    #[test]
    fn first_sample_and_counter_reset_create_gaps_without_fake_zero_rates() {
        let start = Instant::now();
        let mut accumulator = TrafficAccumulator::default();
        let (first, _) = accumulator.process(
            sample(interface(100, 200, InterfaceKind::Ethernet)),
            0,
            1,
            0,
            start,
            Duration::from_secs(1),
        );
        assert_eq!(first.sample_state, SampleState::Gap);
        assert_eq!(first.device.download_bytes_per_second, None);

        let (second, deltas) = accumulator.process(
            sample(interface(160, 240, InterfaceKind::Ethernet)),
            0,
            2,
            1_000,
            start + Duration::from_secs(1),
            Duration::from_secs(1),
        );
        assert_eq!(second.device.download_bytes_per_second, Some(60.0));
        assert_eq!(deltas[0].download_bytes, 60);

        let (reset, reset_deltas) = accumulator.process(
            sample(interface(10, 20, InterfaceKind::Ethernet)),
            0,
            3,
            2_000,
            start + Duration::from_secs(2),
            Duration::from_secs(1),
        );
        assert_eq!(reset.sample_state, SampleState::Gap);
        assert!(reset_deltas.is_empty());
    }

    #[test]
    fn tunnel_traffic_is_not_added_to_physical_device_total() {
        let start = Instant::now();
        let mut accumulator = TrafficAccumulator::default();
        accumulator.process(
            sample(interface(100, 100, InterfaceKind::Tunnel)),
            0,
            1,
            0,
            start,
            Duration::from_secs(1),
        );
        let (event, _) = accumulator.process(
            sample(interface(200, 300, InterfaceKind::Tunnel)),
            0,
            2,
            1_000,
            start + Duration::from_secs(1),
            Duration::from_secs(1),
        );
        assert_eq!(event.device.session_download_bytes, 0);
        assert_eq!(
            event.proxy_vpn.traffic.download_bytes_per_second,
            Some(100.0)
        );
    }

    #[test]
    fn expected_interval_scales_the_sleep_gap_threshold() {
        let start = Instant::now();
        let mut accumulator = TrafficAccumulator::default();
        accumulator.process(
            sample(interface(100, 100, InterfaceKind::Ethernet)),
            0,
            1,
            0,
            start,
            Duration::from_secs(10),
        );
        let (regular, _) = accumulator.process(
            sample(interface(200, 200, InterfaceKind::Ethernet)),
            0,
            2,
            20_000,
            start + Duration::from_secs(20),
            Duration::from_secs(10),
        );
        assert_eq!(regular.sample_state, SampleState::Sample);

        let (gap, deltas) = accumulator.process(
            sample(interface(300, 300, InterfaceKind::Ethernet)),
            0,
            3,
            55_000,
            start + Duration::from_secs(55),
            Duration::from_secs(10),
        );
        assert_eq!(gap.sample_state, SampleState::Gap);
        assert!(deltas.is_empty());
    }
}
