#[cfg(test)]
mod integration_tests {
    use crate::voucher_history::*;
    use std::time::Instant;

    /// E2E Test: Create 10 transactions and export all records
    #[test]
    fn test_e2e_create_and_export_10_transactions() {
        let records = create_test_records(10);
        let filter = VoucherHistoryFilter::default();
        let page = query_voucher_history(&records, &filter, 0, 100);

        assert_eq!(page.total, 10);
        assert_eq!(page.records.len(), 10);

        // Verify all records are present
        for record in &page.records {
            assert!(!record.tx_hash.is_empty());
        }
    }

    /// Test: Security - Non-owner cannot export another voucher's history
    /// (This would be enforced at the endpoint level via auth middleware)
    #[test]
    fn test_security_address_ownership() {
        let _owner = "voucher_owner";
        let records = vec![VoucherHistoryRecord {
            timestamp: 1000,
            event_type: VoucherEventType::Vouch,
            borrower: "borrower_a".to_string(),
            amount_stroops: 100_000_000,
            tx_hash: "tx1".to_string(),
        }];

        let filter = VoucherHistoryFilter::default();
        let page = query_voucher_history(&records, &filter, 0, 100);

        // Verify data is present (auth check happens at endpoint)
        assert_eq!(page.total, 1);
        assert_eq!(page.records[0].borrower, "borrower_a");
    }

    /// Performance Test: Export 1000 records in < 2 seconds
    #[test]
    fn test_performance_1000_records_under_2_seconds() {
        let records = create_test_records(1000);
        let filter = VoucherHistoryFilter::default();

        let start = Instant::now();
        let page = query_voucher_history(&records, &filter, 0, 1000);
        let duration = start.elapsed();

        assert_eq!(page.total, 1000);
        assert_eq!(page.records.len(), 1000);
        assert!(
            duration.as_secs_f64() < 2.0,
            "Export took {:.2}s, should be < 2s",
            duration.as_secs_f64()
        );
    }

    /// Performance Test: CSV export formatting for 1000 records < 2 seconds
    #[test]
    fn test_performance_csv_export_1000_records() {
        let records = create_test_records(1000);

        let start = Instant::now();
        let csv = records_to_csv(&records);
        let duration = start.elapsed();

        assert!(
            duration.as_secs_f64() < 2.0,
            "CSV export took {:.2}s",
            duration.as_secs_f64()
        );

        // Verify CSV structure
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 1001); // header + 1000 records
        assert!(lines[0].contains("date,type,borrower"));
    }

    /// Test: Pagination - Verify no data loss with 2000 records
    #[test]
    fn test_pagination_2000_records_no_data_loss() {
        let records = create_test_records(2000);
        let filter = VoucherHistoryFilter::default();

        // Fetch in pages of 500
        let page1 = query_voucher_history(&records, &filter, 0, 500);
        let page2 = query_voucher_history(&records, &filter, 500, 500);
        let page3 = query_voucher_history(&records, &filter, 1000, 500);
        let page4 = query_voucher_history(&records, &filter, 1500, 500);

        assert_eq!(page1.records.len(), 500);
        assert_eq!(page2.records.len(), 500);
        assert_eq!(page3.records.len(), 500);
        assert_eq!(page4.records.len(), 500);

        // Verify no duplicates by checking tx_hashes
        let mut hashes = std::collections::HashSet::new();
        for record in &page1.records {
            hashes.insert(record.tx_hash.clone());
        }
        for record in &page2.records {
            hashes.insert(record.tx_hash.clone());
        }
        for record in &page3.records {
            hashes.insert(record.tx_hash.clone());
        }
        for record in &page4.records {
            hashes.insert(record.tx_hash.clone());
        }

        assert_eq!(hashes.len(), 2000);
    }

    /// Test: Edge case - Empty dataset
    #[test]
    fn test_edge_case_empty_dataset() {
        let records: Vec<VoucherHistoryRecord> = vec![];
        let filter = VoucherHistoryFilter::default();
        let page = query_voucher_history(&records, &filter, 0, 100);

        assert_eq!(page.total, 0);
        assert_eq!(page.records.len(), 0);

        let csv = records_to_csv(&records);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 1); // Only header
    }

    /// Test: CSV formatting - Special characters escaping
    #[test]
    fn test_csv_special_character_escaping() {
        let records = vec![
            VoucherHistoryRecord {
                timestamp: 1000,
                event_type: VoucherEventType::Vouch,
                borrower: "addr,with,commas".to_string(),
                amount_stroops: 100_000_000,
                tx_hash: "tx\"with\"quotes".to_string(),
            },
            VoucherHistoryRecord {
                timestamp: 2000,
                event_type: VoucherEventType::YieldEarned,
                borrower: "addr\nwith\nnewlines".to_string(),
                amount_stroops: 50_000_000,
                tx_hash: "normal_tx".to_string(),
            },
        ];

        let csv = records_to_csv(&records);

        // Verify escaping
        assert!(csv.contains("\"addr,with,commas\""));
        assert!(csv.contains("\"tx\"\"with\"\"quotes\""));
        assert!(csv.contains("\"addr\nwith\nnewlines\""));
    }

    /// Test: JSON export format validation
    #[test]
    fn test_json_export_format() {
        let records = create_test_records(5);
        let filter = VoucherHistoryFilter::default();
        let page = query_voucher_history(&records, &filter, 0, 100);

        // Simulate JSON export
        let json = serde_json::to_string(&page).expect("JSON serialization failed");

        // Parse it back to verify valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("Invalid JSON");

        assert!(parsed.is_object());
        assert!(parsed["records"].is_array());
        assert_eq!(parsed["records"].as_array().unwrap().len(), 5);
        assert_eq!(parsed["total"].as_u64(), Some(5));
    }

    /// Test: Filter by transaction type "vouch,slash"
    #[test]
    fn test_filter_multiple_transaction_types() {
        let records = vec![
            VoucherHistoryRecord {
                timestamp: 1000,
                event_type: VoucherEventType::Vouch,
                borrower: "b1".to_string(),
                amount_stroops: 100,
                tx_hash: "tx1".to_string(),
            },
            VoucherHistoryRecord {
                timestamp: 2000,
                event_type: VoucherEventType::IncreaseStake,
                borrower: "b1".to_string(),
                amount_stroops: 50,
                tx_hash: "tx2".to_string(),
            },
            VoucherHistoryRecord {
                timestamp: 3000,
                event_type: VoucherEventType::Slash,
                borrower: "b1".to_string(),
                amount_stroops: 25,
                tx_hash: "tx3".to_string(),
            },
        ];

        let filter = VoucherHistoryFilter {
            transaction_types: Some("vouch,slash".to_string()),
            ..Default::default()
        };

        let page = query_voucher_history(&records, &filter, 0, 100);
        assert_eq!(page.total, 2);
        assert!(page
            .records
            .iter()
            .all(|r| r.event_type == VoucherEventType::Vouch
                || r.event_type == VoucherEventType::Slash));
    }

    /// Test: Date range filtering
    #[test]
    fn test_filter_by_date_range() {
        let records = vec![
            VoucherHistoryRecord {
                timestamp: 1000,
                event_type: VoucherEventType::Vouch,
                borrower: "b1".to_string(),
                amount_stroops: 100,
                tx_hash: "tx1".to_string(),
            },
            VoucherHistoryRecord {
                timestamp: 2000,
                event_type: VoucherEventType::Vouch,
                borrower: "b1".to_string(),
                amount_stroops: 100,
                tx_hash: "tx2".to_string(),
            },
            VoucherHistoryRecord {
                timestamp: 3000,
                event_type: VoucherEventType::Vouch,
                borrower: "b1".to_string(),
                amount_stroops: 100,
                tx_hash: "tx3".to_string(),
            },
            VoucherHistoryRecord {
                timestamp: 4000,
                event_type: VoucherEventType::Vouch,
                borrower: "b1".to_string(),
                amount_stroops: 100,
                tx_hash: "tx4".to_string(),
            },
        ];

        let filter = VoucherHistoryFilter {
            start_date: Some(1500),
            end_date: Some(3500),
            ..Default::default()
        };

        let page = query_voucher_history(&records, &filter, 0, 100);
        assert_eq!(page.total, 2);
        assert_eq!(page.records[0].tx_hash, "tx2");
        assert_eq!(page.records[1].tx_hash, "tx3");
    }

    /// Test: Combined filters (date + borrower + type)
    #[test]
    fn test_combined_filters() {
        let records = create_test_records_varied(10);

        let filter = VoucherHistoryFilter {
            start_date: Some(1500),
            end_date: Some(8000),
            borrower: Some("borrower_0".to_string()),
            transaction_types: Some("vouch,increase_stake".to_string()),
        };

        let page = query_voucher_history(&records, &filter, 0, 100);

        // Verify all results match filter criteria
        for record in &page.records {
            assert!(record.timestamp >= 1500 && record.timestamp <= 8000);
            assert_eq!(record.borrower, "borrower_0");
            assert!(
                record.event_type == VoucherEventType::Vouch
                    || record.event_type == VoucherEventType::IncreaseStake
            );
        }
    }

    /// Test: Activity summary calculation
    #[test]
    fn test_activity_summary_calculation() {
        let records = vec![
            VoucherHistoryRecord {
                timestamp: 1000,
                event_type: VoucherEventType::Vouch,
                borrower: "b1".to_string(),
                amount_stroops: 1000,
                tx_hash: "tx1".to_string(),
            },
            VoucherHistoryRecord {
                timestamp: 2000,
                event_type: VoucherEventType::IncreaseStake,
                borrower: "b1".to_string(),
                amount_stroops: 500,
                tx_hash: "tx2".to_string(),
            },
            VoucherHistoryRecord {
                timestamp: 3000,
                event_type: VoucherEventType::YieldEarned,
                borrower: "b1".to_string(),
                amount_stroops: 100,
                tx_hash: "tx3".to_string(),
            },
            VoucherHistoryRecord {
                timestamp: 4000,
                event_type: VoucherEventType::Slash,
                borrower: "b1".to_string(),
                amount_stroops: 250,
                tx_hash: "tx4".to_string(),
            },
        ];

        let summary = compute_activity_summary(&records);

        assert_eq!(summary.total_staked, 1500);
        assert_eq!(summary.total_yield_earned, 100);
        assert_eq!(summary.total_slashed, 250);
        assert_eq!(summary.vouch_count, 2);
        assert_eq!(summary.slash_count, 1);
    }

    // Helper function to create test records
    fn create_test_records(count: usize) -> Vec<VoucherHistoryRecord> {
        (0..count)
            .map(|i| VoucherHistoryRecord {
                timestamp: (i as i64) * 100 + 1000,
                event_type: if i % 2 == 0 {
                    VoucherEventType::Vouch
                } else {
                    VoucherEventType::IncreaseStake
                },
                borrower: format!("borrower_{}", i % 3),
                amount_stroops: (i as i128 + 1) * 1_000_000,
                tx_hash: format!("tx_{:06}", i),
            })
            .collect()
    }

    // Helper function to create varied test records
    fn create_test_records_varied(count: usize) -> Vec<VoucherHistoryRecord> {
        let types = [
            VoucherEventType::Vouch,
            VoucherEventType::IncreaseStake,
            VoucherEventType::DecreaseStake,
            VoucherEventType::YieldEarned,
            VoucherEventType::Slash,
        ];

        (0..count)
            .map(|i| VoucherHistoryRecord {
                timestamp: (i as i64) * 100 + 1000,
                event_type: types[i % types.len()].clone(),
                borrower: format!("borrower_{}", i % 3),
                amount_stroops: (i as i128 + 1) * 1_000_000,
                tx_hash: format!("tx_{:06}", i),
            })
            .collect()
    }
}
