//! Tracking of distinct client (cluster) IDs.
//!
//! Clusters send their cluster ID in the `id` query parameter of every graph
//! request. This module keeps an in-memory record of when each ID was last
//! seen and exposes the number of distinct IDs seen within a set of rolling
//! windows as a Prometheus gauge.
//!
//! IDs are never exported as metric labels (that would create an unbounded
//! number of time series); only aggregate counts leave the process. IDs are
//! stored in memory only, as parsed 128-bit values, and are evicted once they
//! fall outside the largest window.
//!
//! Each replica tracks independently. Because clusters poll frequently and
//! requests are spread across replicas, `max()` across replicas is a close
//! lower bound of the true number of distinct clients over long windows;
//! `sum()` over-counts.

use parking_lot::Mutex;
use prometheus::core::{Collector, Desc};
use prometheus::proto::MetricFamily;
use prometheus::{IntCounter, IntGaugeVec, Opts};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Name of the query parameter carrying the client ID.
pub const CLIENT_ID_PARAM: &str = "id";

/// Default upper bound on the number of tracked IDs, to cap memory usage.
pub const DEFAULT_MAX_ENTRIES: usize = 1_000_000;

/// Minimum interval between evictions triggered from the request path when the
/// tracker is full. Bounds the work done for rejected IDs; scrapes evict as well.
const FULL_EVICT_INTERVAL: Duration = Duration::from_secs(60);

/// Rolling windows (label value, duration) for which distinct counts are reported.
pub const DEFAULT_WINDOWS: &[(&str, Duration)] = &[
    ("1h", Duration::from_secs(60 * 60)),
    ("24h", Duration::from_secs(24 * 60 * 60)),
];

/// In-memory tracker of distinct client IDs.
pub struct ClientIdTracker {
    state: Mutex<State>,
    windows: Vec<(&'static str, Duration)>,
    retention: Duration,
    max_entries: usize,
    distinct: IntGaugeVec,
    dropped: IntCounter,
}

struct State {
    last_seen: HashMap<u128, Instant>,
    /// Time of the last eviction triggered by a full tracker on the request path.
    last_full_evict: Option<Instant>,
}

impl fmt::Debug for ClientIdTracker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClientIdTracker")
            .field("windows", &self.windows)
            .field("max_entries", &self.max_entries)
            .finish_non_exhaustive()
    }
}

impl ClientIdTracker {
    /// Create a tracker with the default windows and entry limit.
    pub fn new() -> Self {
        Self::with_config(DEFAULT_WINDOWS, DEFAULT_MAX_ENTRIES)
    }

    /// Create a tracker with custom windows and entry limit.
    pub fn with_config(windows: &[(&'static str, Duration)], max_entries: usize) -> Self {
        let retention = windows.iter().map(|(_, d)| *d).max().unwrap_or_default();
        let distinct = IntGaugeVec::new(
            Opts::new(
                "graph_distinct_client_ids",
                "Number of distinct client IDs (the 'id' query parameter) seen by this replica within a rolling window",
            ),
            &["window"],
        )
        .expect("valid gauge definition");
        let dropped = IntCounter::new(
            "graph_client_ids_dropped_total",
            "Number of new client IDs not tracked because the tracker was full",
        )
        .expect("valid counter definition");

        Self {
            state: Mutex::new(State {
                last_seen: HashMap::new(),
                last_full_evict: None,
            }),
            windows: windows.to_vec(),
            retention,
            max_entries,
            distinct,
            dropped,
        }
    }

    /// Record a request from `raw_id`. Values which are not valid UUIDs are ignored.
    pub fn record(&self, raw_id: &str) {
        self.record_at(raw_id, Instant::now())
    }

    fn record_at(&self, raw_id: &str, now: Instant) {
        let id = match parse_uuid(raw_id) {
            Some(id) => id,
            None => return,
        };
        let mut state = self.state.lock();
        if let Some(seen) = state.last_seen.get_mut(&id) {
            *seen = now;
            return;
        }
        if state.last_seen.len() >= self.max_entries {
            // Evicting scans the whole map, so do it at most once per interval;
            // otherwise a stream of new IDs against a full tracker would make
            // every request pay for a full scan while holding the lock.
            let may_evict = match state.last_full_evict {
                None => true,
                Some(t) => now.saturating_duration_since(t) >= FULL_EVICT_INTERVAL,
            };
            if may_evict {
                state.last_full_evict = Some(now);
                Self::evict(&mut state.last_seen, now, self.retention);
            }
            if state.last_seen.len() >= self.max_entries {
                self.dropped.inc();
                return;
            }
        }
        state.last_seen.insert(id, now);
    }

    fn evict(map: &mut HashMap<u128, Instant>, now: Instant, retention: Duration) {
        map.retain(|_, seen| now.saturating_duration_since(*seen) <= retention);
    }

    /// Evict expired entries and return the distinct counts for each window.
    fn refresh_at(&self, now: Instant) -> Vec<(&'static str, i64)> {
        let mut state = self.state.lock();
        let map = &mut state.last_seen;
        Self::evict(map, now, self.retention);
        self.windows
            .iter()
            .map(|(label, window)| {
                let count = map
                    .values()
                    .filter(|seen| now.saturating_duration_since(**seen) <= *window)
                    .count();
                (*label, count as i64)
            })
            .collect()
    }
}

impl Default for ClientIdTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Prometheus collector computing distinct counts at scrape time.
pub struct ClientIdCollector(pub Arc<ClientIdTracker>);

impl Collector for ClientIdCollector {
    fn desc(&self) -> Vec<&Desc> {
        let mut descs = self.0.distinct.desc();
        descs.extend(self.0.dropped.desc());
        descs
    }

    fn collect(&self) -> Vec<MetricFamily> {
        for (label, count) in self.0.refresh_at(Instant::now()) {
            self.0.distinct.with_label_values(&[label]).set(count);
        }
        let mut families = self.0.distinct.collect();
        families.extend(self.0.dropped.collect());
        families
    }
}

/// Parse a canonical (8-4-4-4-12, hyphenated) UUID string into its 128-bit value.
fn parse_uuid(s: &str) -> Option<u128> {
    let bytes = s.as_bytes();
    if bytes.len() != 36 {
        return None;
    }
    let mut value: u128 = 0;
    for (i, b) in bytes.iter().enumerate() {
        if matches!(i, 8 | 13 | 18 | 23) {
            if *b != b'-' {
                return None;
            }
            continue;
        }
        let nibble = (*b as char).to_digit(16)?;
        value = (value << 4) | nibble as u128;
    }
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID_A: &str = "01234567-89ab-cdef-0123-456789abcdef";
    const ID_A_UPPER: &str = "01234567-89AB-CDEF-0123-456789ABCDEF";
    const ID_B: &str = "fedcba98-7654-3210-fedc-ba9876543210";
    const ID_C: &str = "00000000-0000-0000-0000-000000000001";

    fn tracker(max_entries: usize) -> ClientIdTracker {
        ClientIdTracker::with_config(
            &[
                ("1h", Duration::from_secs(3600)),
                ("24h", Duration::from_secs(86400)),
            ],
            max_entries,
        )
    }

    #[test]
    fn parse_uuid_valid_and_invalid() {
        assert_eq!(parse_uuid(ID_A), parse_uuid(ID_A_UPPER));
        assert_eq!(parse_uuid(ID_C), Some(1));
        assert!(parse_uuid("").is_none());
        assert!(parse_uuid("not-a-uuid").is_none());
        assert!(parse_uuid("0123456789abcdef0123456789abcdef").is_none());
        assert!(parse_uuid("01234567-89ab-cdef-0123-456789abcdeg").is_none());
        assert!(parse_uuid("01234567_89ab-cdef-0123-456789abcdef").is_none());
        assert!(parse_uuid("+1234567-89ab-cdef-0123-456789abcdef").is_none());
    }

    #[test]
    fn counts_distinct_ids_per_window() {
        let t = tracker(100);
        let start = Instant::now();

        t.record_at(ID_A, start);
        t.record_at(ID_A_UPPER, start); // same ID, different case
        t.record_at(ID_B, start);
        t.record_at("garbage", start);
        assert_eq!(t.refresh_at(start), vec![("1h", 2), ("24h", 2)]);

        // Two hours later only a new ID has been seen within the last hour.
        let later = start + Duration::from_secs(2 * 3600);
        t.record_at(ID_C, later);
        assert_eq!(t.refresh_at(later), vec![("1h", 1), ("24h", 3)]);

        // A repeat request refreshes the last-seen time.
        t.record_at(ID_A, later);
        assert_eq!(t.refresh_at(later), vec![("1h", 2), ("24h", 3)]);

        // After the retention period, stale IDs are evicted.
        let much_later = later + Duration::from_secs(86400 + 1);
        assert_eq!(t.refresh_at(much_later), vec![("1h", 0), ("24h", 0)]);
        assert!(t.state.lock().last_seen.is_empty());
    }

    #[test]
    fn respects_max_entries() {
        let t = tracker(2);
        let start = Instant::now();

        t.record_at(ID_A, start);
        t.record_at(ID_B, start);
        t.record_at(ID_C, start);
        assert_eq!(t.dropped.get(), 1);
        assert_eq!(t.refresh_at(start), vec![("1h", 2), ("24h", 2)]);

        // Known IDs are still refreshed when full.
        t.record_at(ID_A, start);
        assert_eq!(t.dropped.get(), 1);

        // Once old entries expire there is room again.
        let later = start + Duration::from_secs(86400 + 1);
        t.record_at(ID_C, later);
        assert_eq!(t.dropped.get(), 1);
        assert_eq!(t.refresh_at(later), vec![("1h", 1), ("24h", 1)]);
    }

    #[test]
    fn collector_exports_gauge() {
        let registry = prometheus::Registry::new_custom(Some("test".to_string()), None).unwrap();
        let t = Arc::new(tracker(100));
        registry
            .register(Box::new(ClientIdCollector(t.clone())))
            .unwrap();
        t.record(ID_A);
        t.record(ID_B);

        let families = registry.gather();
        let gauge = families
            .iter()
            .find(|f| f.get_name() == "test_graph_distinct_client_ids")
            .expect("gauge family present");
        assert_eq!(gauge.get_metric().len(), 2);
        for m in gauge.get_metric() {
            assert_eq!(m.get_gauge().get_value(), 2.0);
        }
        assert!(families
            .iter()
            .any(|f| f.get_name() == "test_graph_client_ids_dropped_total"));
    }
}
