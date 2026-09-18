use serde::Serialize;
use std::sync::RwLock;

#[derive(Debug, Clone, Default)]
pub struct StageMetric {
    samples: Vec<u64>,
}

impl StageMetric {
    fn push(&mut self, us: u64) {
        self.samples.push(us);
    }

    fn avg_ms(&self) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        let sum: u64 = self.samples.iter().sum();
        (sum as f64 / self.samples.len() as f64) / 1000.0
    }

    fn p95_ms(&self) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        let mut sorted = self.samples.clone();
        sorted.sort_unstable();
        let idx = (sorted.len() as f64 * 0.95).ceil() as usize;
        sorted[idx.min(sorted.len() - 1)] as f64 / 1000.0
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct StageMetricJson {
    pub count: u64,
    pub avg_ms: f64,
    pub p95_ms: f64,
}

impl From<&StageMetric> for StageMetricJson {
    fn from(m: &StageMetric) -> Self {
        Self {
            count: m.samples.len() as u64,
            avg_ms: m.avg_ms(),
            p95_ms: m.p95_ms(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct OrderMetricsJson {
    pub order_total: StageMetricJson,
    pub tx_begin: StageMetricJson,
    pub reserve_balance: StageMetricJson,
    pub orderbook_lock: StageMetricJson,
    pub matching: StageMetricJson,
    pub settle_trades: StageMetricJson,
    pub tx_commit: StageMetricJson,
}

#[derive(Debug, Clone, Default)]
struct OrderMetrics {
    order_total: StageMetric,
    tx_begin: StageMetric,
    reserve_balance: StageMetric,
    orderbook_lock: StageMetric,
    matching: StageMetric,
    settle_trades: StageMetric,
    tx_commit: StageMetric,
}

impl From<&OrderMetrics> for OrderMetricsJson {
    fn from(m: &OrderMetrics) -> Self {
        Self {
            order_total: StageMetricJson {
                ..StageMetricJson::from(&m.order_total)
            },
            tx_begin: StageMetricJson {
                ..StageMetricJson::from(&m.tx_begin)
            },
            reserve_balance: StageMetricJson {
                ..StageMetricJson::from(&m.reserve_balance)
            },
            orderbook_lock: StageMetricJson {
                ..StageMetricJson::from(&m.orderbook_lock)
            },
            matching: StageMetricJson {
                ..StageMetricJson::from(&m.matching)
            },
            settle_trades: StageMetricJson {
                ..StageMetricJson::from(&m.settle_trades)
            },
            tx_commit: StageMetricJson {
                ..StageMetricJson::from(&m.tx_commit)
            },
        }
    }
}

pub struct MetricsCollector {
    inner: RwLock<OrderMetrics>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(OrderMetrics::default()),
        }
    }

    fn add(&self, stage: impl FnOnce(&mut OrderMetrics) -> &mut StageMetric, us: u64) {
        let mut inner = self.inner.write().unwrap();
        stage(&mut inner).push(us);
    }

    pub fn record_order_total(&self, us: u64) {
        self.add(|m| &mut m.order_total, us);
    }

    pub fn record_tx_begin(&self, us: u64) {
        self.add(|m| &mut m.tx_begin, us);
    }

    pub fn record_reserve_balance(&self, us: u64) {
        self.add(|m| &mut m.reserve_balance, us);
    }

    pub fn record_orderbook_lock(&self, us: u64) {
        self.add(|m| &mut m.orderbook_lock, us);
    }

    pub fn record_matching(&self, us: u64) {
        self.add(|m| &mut m.matching, us);
    }

    pub fn record_settle_trades(&self, us: u64) {
        self.add(|m| &mut m.settle_trades, us);
    }

    pub fn record_tx_commit(&self, us: u64) {
        self.add(|m| &mut m.tx_commit, us);
    }

    pub fn snapshot(&self) -> OrderMetricsJson {
        OrderMetricsJson::from(&*self.inner.read().unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avg_ms_zero_count() {
        let m = StageMetric::default();
        let j = StageMetricJson::from(&m);
        assert_eq!(j.avg_ms, 0.0);
        assert_eq!(j.p95_ms, 0.0);
    }

    #[test]
    fn avg_ms_single() {
        let mut m = StageMetric::default();
        m.push(5000);
        let j = StageMetricJson::from(&m);
        assert!((j.avg_ms - 5.0).abs() < 0.01);
        assert!((j.p95_ms - 5.0).abs() < 0.01);
    }

    #[test]
    fn avg_ms_multiple() {
        let mut m = StageMetric::default();
        m.push(1000);
        m.push(2000);
        m.push(3000);
        let j = StageMetricJson::from(&m);
        assert!((j.avg_ms - 2.0).abs() < 0.01);
    }

    #[test]
    fn p95_correctness() {
        let mut m = StageMetric::default();
        for i in 1..=100 {
            m.push(i * 100); // 100us to 10000us
        }
        let j = StageMetricJson::from(&m);
        assert_eq!(j.count, 100);
        // p95: idx = ceil(100*0.95) = 95, index 95 (0-based) = 96th value = 9600us = 9.6ms
        assert!((j.p95_ms - 9.6).abs() < 0.01);
    }

    #[test]
    fn collector_records_and_snapshots() {
        let c = MetricsCollector::new();
        c.record_order_total(1000);
        c.record_order_total(3000);
        c.record_tx_begin(500);

        let snap = c.snapshot();
        assert_eq!(snap.order_total.count, 2);
        assert!((snap.order_total.avg_ms - 2.0).abs() < 0.01);

        assert_eq!(snap.tx_begin.count, 1);
        assert!((snap.tx_begin.avg_ms - 0.5).abs() < 0.01);

        assert_eq!(snap.reserve_balance.count, 0);
    }
}
