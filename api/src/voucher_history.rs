use serde::{Deserialize, Serialize};

/// Voucher history event type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum VoucherEventType {
    Vouch,
    IncreaseStake,
    DecreaseStake,
    WithdrawVouch,
    Slash,
    YieldEarned,
}

impl VoucherEventType {
    pub fn as_str(&self) -> &str {
        match self {
            VoucherEventType::Vouch => "vouch",
            VoucherEventType::IncreaseStake => "increase_stake",
            VoucherEventType::DecreaseStake => "decrease_stake",
            VoucherEventType::WithdrawVouch => "withdraw_vouch",
            VoucherEventType::Slash => "slash",
            VoucherEventType::YieldEarned => "yield_earned",
        }
    }
}

/// Individual voucher history record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoucherHistoryRecord {
    pub timestamp: i64,
    pub event_type: VoucherEventType,
    pub borrower: String,
    pub amount_stroops: i128,
    pub tx_hash: String,
}

impl VoucherHistoryRecord {
    pub fn amount_xlm(&self) -> f64 {
        self.amount_stroops as f64 / 10_000_000.0
    }
}

/// Query result for voucher history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoucherHistoryPage {
    pub records: Vec<VoucherHistoryRecord>,
    pub total: u32,
    pub offset: u32,
    pub limit: u32,
}

/// Filter parameters for voucher history queries
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VoucherHistoryFilter {
    /// Unix timestamp lower bound (inclusive)
    pub start_date: Option<i64>,
    /// Unix timestamp upper bound (inclusive)
    pub end_date: Option<i64>,
    /// Borrower address filter (if provided)
    pub borrower: Option<String>,
    /// Comma-separated event types: "vouch,increase_stake,slash"
    pub transaction_types: Option<String>,
}

/// Aggregate summary of voucher activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoucherActivitySummary {
    pub total_staked: i128,
    pub total_unstaked: i128,
    pub total_yield_earned: i128,
    pub total_slashed: i128,
    pub vouch_count: u32,
    pub slash_count: u32,
}

/// Query and filter voucher history
pub fn query_voucher_history(
    records: &[VoucherHistoryRecord],
    filter: &VoucherHistoryFilter,
    offset: u32,
    limit: u32,
) -> VoucherHistoryPage {
    let filtered: Vec<VoucherHistoryRecord> = records
        .iter()
        .filter(|r| {
            if let Some(start) = filter.start_date {
                if r.timestamp < start {
                    return false;
                }
            }
            if let Some(end) = filter.end_date {
                if r.timestamp > end {
                    return false;
                }
            }
            if let Some(ref borrower) = filter.borrower {
                if r.borrower != *borrower {
                    return false;
                }
            }
            if let Some(ref types) = filter.transaction_types {
                let type_set: Vec<&str> = types.split(',').map(|s| s.trim()).collect();
                if !type_set.contains(&r.event_type.as_str()) {
                    return false;
                }
            }
            true
        })
        .cloned()
        .collect();

    let total = filtered.len() as u32;
    let start = (offset as usize).min(filtered.len());
    let end = ((offset + limit) as usize).min(filtered.len());
    let page_records = filtered[start..end].to_vec();

    VoucherHistoryPage {
        records: page_records,
        total,
        offset,
        limit,
    }
}

/// Compute activity summary from history records
pub fn compute_activity_summary(records: &[VoucherHistoryRecord]) -> VoucherActivitySummary {
    let mut summary = VoucherActivitySummary {
        total_staked: 0,
        total_unstaked: 0,
        total_yield_earned: 0,
        total_slashed: 0,
        vouch_count: 0,
        slash_count: 0,
    };

    for record in records {
        match record.event_type {
            VoucherEventType::Vouch | VoucherEventType::IncreaseStake => {
                summary.total_staked += record.amount_stroops;
                summary.vouch_count += 1;
            }
            VoucherEventType::DecreaseStake | VoucherEventType::WithdrawVouch => {
                summary.total_unstaked += record.amount_stroops;
            }
            VoucherEventType::YieldEarned => {
                summary.total_yield_earned += record.amount_stroops;
            }
            VoucherEventType::Slash => {
                summary.total_slashed += record.amount_stroops;
                summary.slash_count += 1;
            }
        }
    }

    summary
}

/// Format records as CSV with proper escaping
pub fn records_to_csv(records: &[VoucherHistoryRecord]) -> String {
    let mut out = String::from("date,type,borrower,amount_stroops,amount_xlm,tx_hash\n");

    for r in records {
        let date_str = chrono::DateTime::from_timestamp(r.timestamp, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
            .unwrap_or_else(|| format!("{}", r.timestamp));

        let borrower = escape_csv(&r.borrower);
        let tx_hash = escape_csv(&r.tx_hash);

        out.push_str(&format!(
            "{},{},{},{},{:.7},{}\n",
            date_str,
            r.event_type.as_str(),
            borrower,
            r.amount_stroops,
            r.amount_xlm(),
            tx_hash,
        ));
    }

    out
}

/// Escape CSV field value (quote if contains comma, quote, or newline)
fn escape_csv(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_records() -> Vec<VoucherHistoryRecord> {
        vec![
            VoucherHistoryRecord {
                timestamp: 1000,
                event_type: VoucherEventType::Vouch,
                borrower: "borrower_a".to_string(),
                amount_stroops: 1_000_000_000,
                tx_hash: "tx1".to_string(),
            },
            VoucherHistoryRecord {
                timestamp: 2000,
                event_type: VoucherEventType::IncreaseStake,
                borrower: "borrower_a".to_string(),
                amount_stroops: 500_000_000,
                tx_hash: "tx2".to_string(),
            },
            VoucherHistoryRecord {
                timestamp: 3000,
                event_type: VoucherEventType::YieldEarned,
                borrower: "borrower_a".to_string(),
                amount_stroops: 30_000_000,
                tx_hash: "tx3".to_string(),
            },
            VoucherHistoryRecord {
                timestamp: 4000,
                event_type: VoucherEventType::Slash,
                borrower: "borrower_b".to_string(),
                amount_stroops: 250_000_000,
                tx_hash: "tx4".to_string(),
            },
        ]
    }

    #[test]
    fn test_query_voucher_history_all() {
        let records = sample_records();
        let filter = VoucherHistoryFilter::default();
        let page = query_voucher_history(&records, &filter, 0, 10);
        assert_eq!(page.total, 4);
        assert_eq!(page.records.len(), 4);
    }

    #[test]
    fn test_query_voucher_history_date_filter() {
        let records = sample_records();
        let filter = VoucherHistoryFilter {
            start_date: Some(2000),
            end_date: Some(3500),
            ..Default::default()
        };
        let page = query_voucher_history(&records, &filter, 0, 10);
        assert_eq!(page.total, 2);
    }

    #[test]
    fn test_query_voucher_history_borrower_filter() {
        let records = sample_records();
        let filter = VoucherHistoryFilter {
            borrower: Some("borrower_a".to_string()),
            ..Default::default()
        };
        let page = query_voucher_history(&records, &filter, 0, 10);
        assert_eq!(page.total, 3);
    }

    #[test]
    fn test_query_voucher_history_type_filter() {
        let records = sample_records();
        let filter = VoucherHistoryFilter {
            transaction_types: Some("vouch,increase_stake".to_string()),
            ..Default::default()
        };
        let page = query_voucher_history(&records, &filter, 0, 10);
        assert_eq!(page.total, 2);
    }

    #[test]
    fn test_query_voucher_history_pagination() {
        let records = sample_records();
        let filter = VoucherHistoryFilter::default();
        let page = query_voucher_history(&records, &filter, 1, 2);
        assert_eq!(page.total, 4);
        assert_eq!(page.records.len(), 2);
        assert_eq!(page.offset, 1);
        assert_eq!(page.limit, 2);
    }

    #[test]
    fn test_compute_activity_summary() {
        let records = sample_records();
        let summary = compute_activity_summary(&records);
        assert_eq!(summary.total_staked, 1_500_000_000);
        assert_eq!(summary.total_yield_earned, 30_000_000);
        assert_eq!(summary.total_slashed, 250_000_000);
        assert_eq!(summary.vouch_count, 2);
        assert_eq!(summary.slash_count, 1);
    }

    #[test]
    fn test_records_to_csv_escaping() {
        let records = vec![VoucherHistoryRecord {
            timestamp: 1000,
            event_type: VoucherEventType::Vouch,
            borrower: "addr,with,comma".to_string(),
            amount_stroops: 1_000_000_000,
            tx_hash: "hash\"with\"quote".to_string(),
        }];
        let csv = records_to_csv(&records);
        assert!(csv.contains("\"addr,with,comma\""));
        assert!(csv.contains("\"hash\"\"with\"\"quote\""));
    }

    #[test]
    fn test_records_to_csv_format() {
        let records = sample_records();
        let csv = records_to_csv(&records);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 5); // header + 4 records
        assert!(lines[0].contains("date,type,borrower"));
    }

    #[test]
    fn test_amount_xlm_conversion() {
        let record = VoucherHistoryRecord {
            timestamp: 1000,
            event_type: VoucherEventType::Vouch,
            borrower: "addr".to_string(),
            amount_stroops: 10_000_000,
            tx_hash: "tx".to_string(),
        };
        assert!((record.amount_xlm() - 1.0).abs() < 1e-7);
    }
}
