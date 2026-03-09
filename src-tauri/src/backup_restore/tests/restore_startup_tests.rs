    #[test]
    fn restore_missing_user_store_preserves_local_fallback() {
        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        seed_history_db(&app_data_dir, &history_rows("fallback-source"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("fallback-source", "SOURCE")],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "from-backup" }));
        seed_recordings(&app_data_dir, &[("fallback-source.wav", b"A")]);
        let expected_history = read_history_rows(&app_data_dir);

        let archive_path = make_backup(&app_handle, BackupScope::Complete, "missing-user-store");
        rewrite_archive(&archive_path, |name, bytes| {
            if name == USER_STORE_FILE {
                return None;
            }

            if name == CHECKSUM_FILE {
                let text = String::from_utf8(bytes.to_vec())
                    .expect("checksums payload should be valid UTF-8");
                let filtered = text
                    .lines()
                    .filter(|line| !line.trim_end().ends_with(USER_STORE_FILE))
                    .collect::<Vec<_>>()
                    .join("\n");
                let normalized = if filtered.is_empty() {
                    String::new()
                } else {
                    format!("{filtered}\n")
                };
                return Some(normalized.into_bytes());
            }

            Some(bytes.to_vec())
        });

        seed_history_db(&app_data_dir, &history_rows("fallback-mutated"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("fallback-mutated", "MUTATED")],
        );
        let local_user_store = json!({ "profile": "local-fallback", "marker": true });
        seed_user_store(&app_data_dir, &local_user_store);

        let report = apply_restore(
            &app_handle,
            ApplyRestoreRequest {
                archive_path: archive_path.to_string_lossy().to_string(),
            },
        )
        .expect("apply restore with missing user store payload");

        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.contains("user/user_store.json was missing")),
            "expected missing user_store warning"
        );
        assert_eq!(read_user_store(&app_data_dir), local_user_store);
        assert_eq!(read_history_rows(&app_data_dir), expected_history);
    }

    #[test]
    fn restore_non_object_user_store_preserves_local_fallback() {
        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        seed_history_db(&app_data_dir, &history_rows("shape-fallback-source"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("shape-fallback-source", "SOURCE")],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "from-backup-shape" }));

        let archive_path = make_backup(
            &app_handle,
            BackupScope::Smaller,
            "invalid-user-store-shape",
        );
        tamper_archive_file(&archive_path, USER_STORE_FILE, |_| b"[]".to_vec());

        let user_store_checksum = archive_entry_checksum(&archive_path, USER_STORE_FILE);
        tamper_archive_file(&archive_path, CHECKSUM_FILE, |bytes| {
            let text = String::from_utf8(bytes).expect("checksums payload should be valid UTF-8");
            replace_checksum_line(&text, USER_STORE_FILE, &user_store_checksum).into_bytes()
        });

        let local_user_store = json!({ "profile": "local-shape-fallback", "marker": true });
        seed_user_store(&app_data_dir, &local_user_store);

        let report = apply_restore(
            &app_handle,
            ApplyRestoreRequest {
                archive_path: archive_path.to_string_lossy().to_string(),
            },
        )
        .expect("apply restore with invalid user_store shape");

        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.contains("invalid structure")),
            "expected invalid-structure warning for user_store fallback"
        );
        assert_eq!(read_user_store(&app_data_dir), local_user_store);
    }

    #[cfg(unix)]
    #[test]
    fn restore_rejects_symlinked_local_recordings_root_before_swap() {
        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        seed_history_db(&app_data_dir, &history_rows("symlink-root-source"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("symlink-root-source", "SOURCE")],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "symlink-root-source" }));
        seed_recordings(&app_data_dir, &[("symlink-root-source.wav", b"A")]);
        let archive_path = make_backup(&app_handle, BackupScope::Complete, "symlink-root-backup");

        let recordings = app_data_dir.join(RECORDINGS_DIR);
        fs::remove_dir_all(&recordings).expect("remove active recordings root");
        let outside = app_data_dir.join("outside-recordings");
        fs::create_dir_all(&outside).expect("create outside recordings directory");
        fs::write(outside.join("outside.wav"), b"O").expect("write outside recording fixture");
        symlink(&outside, &recordings).expect("replace recordings root with symlink");
        assert!(
            fs::symlink_metadata(&recordings)
                .expect("inspect recordings root")
                .file_type()
                .is_symlink(),
            "recordings root should be a symlink fixture"
        );

        let expected_state = snapshot_state(&app_data_dir);

        let error = apply_restore(
            &app_handle,
            ApplyRestoreRequest {
                archive_path: archive_path.to_string_lossy().to_string(),
            },
        )
        .expect_err("restore should fail closed for symlinked recordings root");
        assert!(
            error.contains("symbolic link source directory"),
            "unexpected restore failure: {error}"
        );
        assert_eq!(
            snapshot_state(&app_data_dir),
            expected_state,
            "restore failure must keep active data unchanged"
        );
        assert!(
            fs::symlink_metadata(&recordings)
                .expect("inspect recordings root after restore failure")
                .file_type()
                .is_symlink(),
            "symlinked recordings root should remain unchanged after failure"
        );
        assert!(
            outside.join("outside.wav").exists(),
            "outside target should not be mutated by failed restore"
        );
        assert!(
            !marker_path(&app_data_dir).exists(),
            "restore marker must not be written when snapshot creation fails"
        );
    }

    #[test]
    fn restore_failure_during_swap_rolls_back_to_snapshot() {
        let (_guard, env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        seed_history_db(&app_data_dir, &history_rows("rollback-source"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("rollback-source", "SOURCE")],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "rollback-source" }));
        seed_recordings(&app_data_dir, &[("rollback-source.wav", b"A")]);
        let archive_path = make_backup(&app_handle, BackupScope::Complete, "rollback-failpoint");

        seed_history_db(&app_data_dir, &history_rows("rollback-mutated"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("rollback-mutated", "MUTATED")],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "rollback-mutated" }));
        seed_recordings(&app_data_dir, &[("rollback-mutated.wav", b"M")]);
        let expected_state = snapshot_state(&app_data_dir);

        env.set_failpoint("swap_after_displace");
        let result = apply_restore(
            &app_handle,
            ApplyRestoreRequest {
                archive_path: archive_path.to_string_lossy().to_string(),
            },
        );
        env.clear_failpoint();

        let error = result.expect_err("restore should fail at failpoint");
        assert!(error.contains("Injected failpoint: swap_after_displace"));
        assert_eq!(snapshot_state(&app_data_dir), expected_state);
        assert!(
            !marker_path(&app_data_dir).exists(),
            "restore marker must be cleared after rollback"
        );
        assert!(
            list_runtime_dirs_with_prefix(&app_data_dir, "snapshot-").is_empty(),
            "rollback snapshot directories should be cleaned after rollback failure"
        );
    }

    #[test]
    fn restore_failure_after_swap_before_commit_rolls_back_to_snapshot() {
        let (_guard, env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        seed_history_db(&app_data_dir, &history_rows("post-swap-source"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("post-swap-source", "SOURCE")],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "post-swap-source" }));
        seed_recordings(
            &app_data_dir,
            &[("post-swap-source-1.wav", b"A"), ("post-swap-source-2.wav", b"B")],
        );
        let archive_path = make_backup(&app_handle, BackupScope::Complete, "post-swap-failpoint");

        seed_history_db(&app_data_dir, &history_rows("post-swap-mutated"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("post-swap-mutated", "MUTATED")],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "post-swap-mutated" }));
        seed_recordings(&app_data_dir, &[("post-swap-mutated.wav", b"M")]);
        let expected_state = snapshot_state(&app_data_dir);

        env.set_failpoint("restore_after_swap_before_commit");
        let result = apply_restore(
            &app_handle,
            ApplyRestoreRequest {
                archive_path: archive_path.to_string_lossy().to_string(),
            },
        );
        env.clear_failpoint();

        let error = result.expect_err("restore should fail after swap failpoint");
        assert!(error.contains("restore_after_swap_before_commit"));
        assert_eq!(snapshot_state(&app_data_dir), expected_state);
        assert!(
            !marker_path(&app_data_dir).exists(),
            "restore marker must be cleared after rollback"
        );
        assert!(
            list_runtime_dirs_with_prefix(&app_data_dir, "snapshot-").is_empty(),
            "rollback snapshot directories should be cleaned after rollback failure"
        );
    }

    #[test]
    fn startup_reconcile_in_progress_rolls_back() {
        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        seed_history_db(&app_data_dir, &history_rows("reconcile-snapshot"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("reconcile-snapshot", "SNAPSHOT")],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "snapshot" }));
        seed_recordings(&app_data_dir, &[("reconcile-snapshot.wav", b"S")]);
        let expected_state = snapshot_state(&app_data_dir);

        let runtime = runtime_dir(&app_data_dir);
        fs::create_dir_all(&runtime).expect("create runtime dir");
        let snapshot_path = create_snapshot(&app_data_dir, &runtime).expect("create snapshot");

        seed_history_db(&app_data_dir, &history_rows("reconcile-active"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("reconcile-active", "ACTIVE")],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "active" }));
        seed_recordings(&app_data_dir, &[("reconcile-active.wav", b"A")]);

        durable_write_json(
            &marker_path(&app_data_dir),
            &RestoreMarker {
                state: "in_progress".to_string(),
                snapshot_path: snapshot_path.to_string_lossy().to_string(),
                updated_at: now_rfc3339(),
                snapshot_layout: Some(
                    build_restore_marker_snapshot_layout(&snapshot_path)
                        .expect("build restore marker snapshot layout"),
                ),
            },
        )
        .expect("write in-progress marker");

        reconcile_startup(&app_handle);

        assert_eq!(snapshot_state(&app_data_dir), expected_state);
        assert!(
            !marker_path(&app_data_dir).exists(),
            "marker should be removed after startup reconciliation"
        );
        assert!(
            !snapshot_path.exists(),
            "startup rollback snapshot should be removed after reconciliation"
        );
    }

    #[test]
    fn startup_reconcile_in_progress_refreshes_dictionary_state() {
        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        seed_history_db(&app_data_dir, &history_rows("reconcile-dict-snapshot"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("reconcile-dict-snapshot", "SNAPSHOT")],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "snapshot" }));
        let runtime = runtime_dir(&app_data_dir);
        fs::create_dir_all(&runtime).expect("create runtime dir");
        let snapshot_path = create_snapshot(&app_data_dir, &runtime).expect("create snapshot");

        seed_history_db(&app_data_dir, &history_rows("reconcile-dict-active"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("reconcile-dict-active", "ACTIVE")],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "active" }));

        durable_write_json(
            &marker_path(&app_data_dir),
            &RestoreMarker {
                state: "in_progress".to_string(),
                snapshot_path: snapshot_path.to_string_lossy().to_string(),
                updated_at: now_rfc3339(),
                snapshot_layout: Some(
                    build_restore_marker_snapshot_layout(&snapshot_path)
                        .expect("build restore marker snapshot layout"),
                ),
            },
        )
        .expect("write in-progress marker");

        reconcile_startup(&app_handle);

        let dictionary_snapshot = crate::user_dictionary::get_dictionary_snapshot(&app_handle);
        assert_eq!(dictionary_snapshot.len(), 1);
        assert_eq!(dictionary_snapshot[0].input, "reconcile-dict-snapshot");
        assert_eq!(dictionary_snapshot[0].replacement, "SNAPSHOT");
    }

    #[test]
    fn manual_stats_repair_applies_for_known_bad_signature() {
        let (_guard, env, _app, app_data_dir) = setup_test_app();
        seed_history_db(&app_data_dir, &history_rows("manual-repair-known-bad"));
        seed_user_stats(
            &app_data_dir,
            &TestUserStatsSnapshot {
                total_words: MANUAL_STATS_REPAIR_BAD_WORDS,
                total_duration_ms: MANUAL_STATS_REPAIR_BAD_DURATION_MS,
                total_transcriptions: 2_363,
                first_transcription_date: Some(1_701_000_001),
                last_transcription_date: Some(1_701_000_999),
                transcription_dates: vec!["2026-03-03".to_string()],
                restored_streak_days: 0,
                restored_streak_counted_through_date: None,
                restored_streak_restore_date: None,
                total_filler_words_removed: 0,
                total_speech_duration_ms: MANUAL_STATS_REPAIR_BAD_SPEECH_MS,
                duration_stats_semantics_version: 1,
            },
        );

        env.enable_manual_stats_repair();
        let report = maybe_run_manual_stats_repair(&app_data_dir)
            .expect("run manual stats repair")
            .expect("manual repair should be evaluated when env flag is set");
        env.disable_manual_stats_repair();

        assert!(report.applied);
        assert_eq!(report.reason, "guard_match");
        let repaired = read_user_stats(&app_data_dir).expect("read repaired stats row");
        assert_eq!(repaired.total_words, MANUAL_STATS_REPAIR_TARGET_WORDS);
        assert_eq!(
            repaired.total_duration_ms,
            MANUAL_STATS_REPAIR_TARGET_DURATION_MS
        );
        assert_eq!(
            repaired.total_speech_duration_ms,
            MANUAL_STATS_REPAIR_TARGET_SPEECH_MS
        );
        assert_eq!(repaired.duration_stats_semantics_version, 1);
    }

    #[test]
    fn manual_stats_repair_skips_when_guard_does_not_match() {
        let (_guard, env, _app, app_data_dir) = setup_test_app();
        seed_history_db(&app_data_dir, &history_rows("manual-repair-guard-mismatch"));
        seed_user_stats(
            &app_data_dir,
            &TestUserStatsSnapshot {
                total_words: 10_000,
                total_duration_ms: 2_000_000,
                total_transcriptions: 100,
                first_transcription_date: Some(1_701_000_001),
                last_transcription_date: Some(1_701_000_999),
                transcription_dates: vec!["2026-02-01".to_string()],
                restored_streak_days: 0,
                restored_streak_counted_through_date: None,
                restored_streak_restore_date: None,
                total_filler_words_removed: 42,
                total_speech_duration_ms: 1_800_000,
                duration_stats_semantics_version: 1,
            },
        );

        let before = read_user_stats(&app_data_dir).expect("read seeded stats row");
        env.enable_manual_stats_repair();
        let report = maybe_run_manual_stats_repair(&app_data_dir)
            .expect("run manual stats repair")
            .expect("manual repair should be evaluated when env flag is set");
        env.disable_manual_stats_repair();

        assert!(!report.applied);
        assert_eq!(report.reason, "guard_mismatch");
        assert_eq!(read_user_stats(&app_data_dir), Some(before));
    }

    #[test]
    fn manual_stats_repair_skips_when_user_stats_row_missing() {
        let (_guard, env, _app, app_data_dir) = setup_test_app();
        seed_history_db(&app_data_dir, &history_rows("manual-repair-row-missing"));

        let history_db_path = app_data_dir.join(HISTORY_DB_FILE);
        let conn = Connection::open(&history_db_path).expect("open history db");
        conn.execute("DELETE FROM user_stats", [])
            .expect("delete user_stats singleton row");

        env.enable_manual_stats_repair();
        let report = maybe_run_manual_stats_repair(&app_data_dir)
            .expect("run manual stats repair")
            .expect("manual repair should be evaluated when env flag is set");
        env.disable_manual_stats_repair();

        assert!(!report.applied);
        assert_eq!(report.reason, "user_stats_row_missing");
        assert_eq!(read_user_stats(&app_data_dir), None);
    }

    #[test]
    fn startup_reconcile_in_progress_without_layout_fails_closed_and_keeps_marker() {
        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        seed_history_db(&app_data_dir, &history_rows("reconcile-no-layout-snapshot"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("reconcile-no-layout-snapshot", "SNAPSHOT")],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "snapshot" }));
        seed_recordings(&app_data_dir, &[("reconcile-no-layout-snapshot.wav", b"S")]);

        let runtime = runtime_dir(&app_data_dir);
        fs::create_dir_all(&runtime).expect("create runtime dir");
        let snapshot_path = create_snapshot(&app_data_dir, &runtime).expect("create snapshot");

        seed_history_db(&app_data_dir, &history_rows("reconcile-no-layout-active"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("reconcile-no-layout-active", "ACTIVE")],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "active" }));
        seed_recordings(&app_data_dir, &[("reconcile-no-layout-active.wav", b"A")]);
        let expected_active_state = snapshot_state(&app_data_dir);

        durable_write_json(
            &marker_path(&app_data_dir),
            &RestoreMarker {
                state: "in_progress".to_string(),
                snapshot_path: snapshot_path.to_string_lossy().to_string(),
                updated_at: now_rfc3339(),
                snapshot_layout: None,
            },
        )
        .expect("write marker without snapshot layout");

        reconcile_startup(&app_handle);

        assert_eq!(snapshot_state(&app_data_dir), expected_active_state);
        assert!(
            marker_path(&app_data_dir).exists(),
            "marker should be kept when snapshot layout metadata is missing"
        );
    }

    #[test]
    fn startup_reconcile_in_progress_with_incomplete_layout_fails_closed_and_keeps_marker() {
        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        seed_history_db(&app_data_dir, &history_rows("reconcile-layout-snapshot"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("reconcile-layout-snapshot", "SNAPSHOT")],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "snapshot" }));
        seed_recordings(&app_data_dir, &[("reconcile-layout-snapshot.wav", b"S")]);

        let runtime = runtime_dir(&app_data_dir);
        fs::create_dir_all(&runtime).expect("create runtime dir");
        let snapshot_path = create_snapshot(&app_data_dir, &runtime).expect("create snapshot");
        let snapshot_layout = build_restore_marker_snapshot_layout(&snapshot_path)
            .expect("build restore marker snapshot layout");

        fs::remove_file(snapshot_path.join(USER_STORE_DB_FILE))
            .expect("remove snapshot user_store to simulate incomplete snapshot");

        seed_history_db(&app_data_dir, &history_rows("reconcile-layout-active"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("reconcile-layout-active", "ACTIVE")],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "active" }));
        seed_recordings(&app_data_dir, &[("reconcile-layout-active.wav", b"A")]);
        let expected_active_state = snapshot_state(&app_data_dir);

        durable_write_json(
            &marker_path(&app_data_dir),
            &RestoreMarker {
                state: "in_progress".to_string(),
                snapshot_path: snapshot_path.to_string_lossy().to_string(),
                updated_at: now_rfc3339(),
                snapshot_layout: Some(snapshot_layout),
            },
        )
        .expect("write marker with incomplete snapshot layout");

        reconcile_startup(&app_handle);

        assert_eq!(snapshot_state(&app_data_dir), expected_active_state);
        assert!(
            marker_path(&app_data_dir).exists(),
            "marker should be kept when snapshot validation fails"
        );
    }

    #[test]
    fn startup_reconcile_committed_keeps_restored_data() {
        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        seed_history_db(&app_data_dir, &history_rows("reconcile-committed"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("reconcile-committed", "COMMITTED")],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "committed" }));
        seed_recordings(&app_data_dir, &[("reconcile-committed.wav", b"C")]);
        let expected_state = snapshot_state(&app_data_dir);

        let runtime = runtime_dir(&app_data_dir);
        fs::create_dir_all(&runtime).expect("create runtime dir");
        let snapshot_path = runtime.join("unused-snapshot");
        fs::create_dir_all(&snapshot_path).expect("create placeholder snapshot dir");

        durable_write_json(
            &marker_path(&app_data_dir),
            &RestoreMarker {
                state: "committed".to_string(),
                snapshot_path: snapshot_path.to_string_lossy().to_string(),
                updated_at: now_rfc3339(),
                snapshot_layout: None,
            },
        )
        .expect("write committed marker");

        reconcile_startup(&app_handle);

        assert_eq!(snapshot_state(&app_data_dir), expected_state);
        assert!(
            !marker_path(&app_data_dir).exists(),
            "committed marker should be removed after startup reconciliation"
        );
    }

    #[test]
    fn startup_reconcile_ignores_marker_snapshot_outside_runtime() {
        let (_guard, env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        let outside_snapshot = env.root.path().join("outside-snapshot");
        fs::create_dir_all(&outside_snapshot).expect("create outside snapshot dir");
        let sentinel = outside_snapshot.join("sentinel.txt");
        fs::write(&sentinel, b"keep").expect("write outside snapshot sentinel");

        durable_write_json(
            &marker_path(&app_data_dir),
            &RestoreMarker {
                state: "in_progress".to_string(),
                snapshot_path: outside_snapshot.to_string_lossy().to_string(),
                updated_at: now_rfc3339(),
                snapshot_layout: None,
            },
        )
        .expect("write invalid marker");

        reconcile_startup(&app_handle);

        assert!(
            !marker_path(&app_data_dir).exists(),
            "invalid marker should be removed"
        );
        assert!(
            sentinel.exists(),
            "reconcile should not touch snapshot directories outside runtime"
        );
    }
