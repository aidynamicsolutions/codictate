    #[test]
    fn complete_backup_roundtrip_restores_all_components() {
        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        let source_rows = history_rows("complete-source");
        seed_history_db(&app_data_dir, &source_rows);
        seed_dictionary(
            &app_handle,
            vec![
                custom_word("complete-alpha", "ALPHA"),
                custom_word("complete-beta", "BETA"),
            ],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "source", "count": 1 }));
        seed_recordings(
            &app_data_dir,
            &[("complete-source-1.wav", b"A"), ("complete-source-2.wav", b"B")],
        );
        let expected_state = snapshot_state(&app_data_dir);

        let archive_path = make_backup(&app_handle, BackupScope::Complete, "complete-roundtrip");

        seed_history_db(&app_data_dir, &history_rows("complete-mutated"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("complete-mutated", "MUTATED")],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "mutated", "count": 99 }));
        seed_recordings(&app_data_dir, &[("complete-mutated-1.wav", b"Z")]);

        apply_restore(
            &app_handle,
            ApplyRestoreRequest {
                archive_path: archive_path.to_string_lossy().to_string(),
            },
        )
        .expect("apply complete restore");

        assert_eq!(snapshot_state(&app_data_dir), expected_state);
    }

    #[test]
    fn complete_backup_roundtrip_restores_canonical_user_stats_payload() {
        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        seed_history_db(&app_data_dir, &history_rows("canonical-stats-source"));
        let source_stats = TestUserStatsSnapshot {
            total_words: 88_888,
            total_duration_ms: 9_999_000,
            total_transcriptions: 1_234,
            first_transcription_date: Some(1_701_000_001),
            last_transcription_date: Some(1_701_000_999),
            transcription_dates: vec![
                "2026-02-28".to_string(),
                "2026-03-01".to_string(),
                "2026-03-02".to_string(),
            ],
            total_filler_words_removed: 777,
            total_speech_duration_ms: 8_888_000,
            duration_stats_semantics_version: 1,
        };
        seed_user_stats(&app_data_dir, &source_stats);

        let archive_path =
            make_backup(&app_handle, BackupScope::Complete, "canonical-stats-roundtrip");

        seed_history_db(&app_data_dir, &history_rows("canonical-stats-mutated"));
        seed_user_stats(
            &app_data_dir,
            &TestUserStatsSnapshot {
                total_words: 10,
                total_duration_ms: 5_000,
                total_transcriptions: 5,
                first_transcription_date: Some(1_700_000_000),
                last_transcription_date: Some(1_700_000_000),
                transcription_dates: vec!["2026-01-01".to_string()],
                total_filler_words_removed: 1,
                total_speech_duration_ms: 4_000,
                duration_stats_semantics_version: 1,
            },
        );

        apply_restore(
            &app_handle,
            ApplyRestoreRequest {
                archive_path: archive_path.to_string_lossy().to_string(),
            },
        )
        .expect("apply restore with canonical user_stats payload");

        assert_eq!(read_user_stats(&app_data_dir), Some(source_stats));
    }

    #[test]
    fn smaller_backup_roundtrip_restores_without_recordings() {
        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        seed_history_db(&app_data_dir, &history_rows("smaller-source"));
        seed_dictionary(
            &app_handle,
            vec![
                custom_word("smaller-alpha", "ALPHA"),
                custom_word("smaller-beta", "BETA"),
            ],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "smaller-source" }));
        seed_recordings(
            &app_data_dir,
            &[("smaller-source-1.wav", b"A"), ("smaller-source-2.wav", b"B")],
        );
        let mut expected_state = snapshot_state(&app_data_dir);
        expected_state.recordings.clear();

        let archive_path = make_backup(&app_handle, BackupScope::Smaller, "smaller-roundtrip");

        seed_history_db(&app_data_dir, &history_rows("smaller-mutated"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("smaller-mutated", "MUTATED")],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "smaller-mutated" }));
        seed_recordings(&app_data_dir, &[("smaller-mutated.wav", b"Q")]);

        let report = apply_restore(
            &app_handle,
            ApplyRestoreRequest {
                archive_path: archive_path.to_string_lossy().to_string(),
            },
        )
        .expect("apply smaller restore");

        assert_eq!(report.counts.recording_files, 0);
        assert_eq!(snapshot_state(&app_data_dir), expected_state);
    }

    #[test]
    fn import_dictionary_payload_writes_entries_and_returns_count() {
        let (_guard, _env, _app, app_data_dir) = setup_test_app();

        let payload_path = app_data_dir.join("dictionary-import-source.json");
        let destination_path = app_data_dir.join("dictionary-import-destination.json");
        let payload = DictionaryPayload {
            version: DICTIONARY_PAYLOAD_VERSION,
            entries: vec![
                custom_word("dict-import-alpha", "ALPHA"),
                custom_word("dict-import-beta", "BETA"),
                custom_word("dict-import-gamma", "GAMMA"),
            ],
        };
        write_json_file_atomically(&payload_path, &payload).expect("write dictionary payload source");

        let count = import_dictionary_payload(&payload_path, &destination_path)
            .expect("import dictionary payload");

        assert_eq!(count, 3);
        let written = read_json_file::<DictionaryPayload>(&destination_path)
            .expect("read imported dictionary payload");
        assert_eq!(written.version, DICTIONARY_PAYLOAD_VERSION);
        assert_eq!(written.entries.len(), 3);
        assert_eq!(written.entries[0].input, "dict-import-alpha");
        assert_eq!(written.entries[1].input, "dict-import-beta");
        assert_eq!(written.entries[2].input, "dict-import-gamma");
    }

    #[test]
    fn import_history_jsonl_fallback_stats_match_runtime_semantics() {
        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();
        let staged_db = app_data_dir.join("fallback-staged-history.db");
        initialize_history_db(&staged_db).expect("initialize staged history db");

        let jsonl_path = app_data_dir.join("fallback-history.jsonl");
        let rows = vec![
            HistoryRowV1 {
                id: 1,
                file_name: "fallback-1.wav".to_string(),
                timestamp: 1_700_100_001,
                saved: false,
                title: "Fallback 1".to_string(),
                transcription_text: "raw one".to_string(),
                post_processed_text: Some("post one".to_string()),
                inserted_text: Some("inserted text should not affect stats".to_string()),
                post_process_prompt: None,
                duration_ms: 1_000,
                speech_duration_ms: 0,
            },
            HistoryRowV1 {
                id: 2,
                file_name: "fallback-2.wav".to_string(),
                timestamp: 1_700_100_002,
                saved: false,
                title: "Fallback 2".to_string(),
                transcription_text: "raw two".to_string(),
                post_processed_text: None,
                inserted_text: Some("ignored inserted text".to_string()),
                post_process_prompt: None,
                duration_ms: 1_500,
                speech_duration_ms: 600,
            },
        ];
        let body = rows
            .iter()
            .map(|row| serde_json::to_string(row).expect("serialize history row"))
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(&jsonl_path, format!("{body}\n")).expect("write fallback history jsonl");

        let summary = import_history_jsonl(&jsonl_path, &staged_db, &app_handle)
            .expect("import history jsonl for fallback semantics");
        assert_eq!(summary.row_count, 2);
        assert_eq!(summary.zero_speech_duration_rows, 1);
        assert_eq!(summary.recomputed_stats.total_words, 4);
        assert_eq!(summary.recomputed_stats.total_duration_ms, 2_500);
        assert_eq!(summary.recomputed_stats.total_speech_duration_ms, 1_600);

        let persisted = read_user_stats_payload_from_history_db(&staged_db)
            .expect("read persisted fallback stats");
        assert_eq!(persisted.total_words, 4);
        assert_eq!(persisted.total_duration_ms, 2_500);
        assert_eq!(persisted.total_speech_duration_ms, 1_600);
    }

    #[test]
    fn backup_export_fails_when_history_payload_exceeds_limit() {
        let (_guard, env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        env.set_payload_limit_bytes(256);

        let long_text = "history-payload-limit ".repeat(40);
        let rows = vec![TestHistoryRow {
            id: 1,
            file_name: "history-limit.wav".to_string(),
            title: "History payload limit".to_string(),
            transcription_text: long_text,
            timestamp: 1_700_000_101,
        }];
        seed_history_db(&app_data_dir, &rows);
        seed_dictionary(&app_handle, vec![custom_word("history-limit", "LIMIT")]);
        seed_user_store(&app_data_dir, &json!({ "profile": "history-limit" }));

        let output = app_data_dir.join("history-payload-limit.codictatebackup");
        let error = create_backup(
            &app_handle,
            CreateBackupRequest {
                scope: BackupScope::Smaller,
                output_path: output.to_string_lossy().to_string(),
            },
        )
        .expect_err("backup export should fail when history payload exceeds export limit");

        env.clear_payload_limit_bytes();

        assert!(
            error.contains("export_payload_size_limit:history/history.jsonl"),
            "expected history payload size-limit failure, got: {error}"
        );
    }

    #[test]
    fn backup_export_fails_when_history_row_exceeds_line_limit() {
        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        let oversized_text = "X".repeat(MAX_HISTORY_JSONL_LINE_BYTES.saturating_add(1024));
        let rows = vec![TestHistoryRow {
            id: 1,
            file_name: "history-line-limit.wav".to_string(),
            title: "History line limit".to_string(),
            transcription_text: oversized_text,
            timestamp: 1_700_100_001,
        }];
        seed_history_db(&app_data_dir, &rows);
        seed_dictionary(&app_handle, vec![custom_word("history-line-limit", "LIMIT")]);
        seed_user_store(&app_data_dir, &json!({ "profile": "history-line-limit" }));

        let output = app_data_dir.join("history-line-limit.codictatebackup");
        let error = create_backup(
            &app_handle,
            CreateBackupRequest {
                scope: BackupScope::Smaller,
                output_path: output.to_string_lossy().to_string(),
            },
        )
        .expect_err("backup export should fail when a history row exceeds line-size limit");

        assert!(
            error.contains("history_jsonl_line_too_large:"),
            "expected oversized history-line failure, got: {error}"
        );
    }

    #[test]
    fn export_history_jsonl_does_not_create_missing_history_db() {
        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        let missing_history_db = app_data_dir.join("missing-history.db");
        let payload_path = app_data_dir.join("workspace/history/history.jsonl");
        let mut referenced_recordings = BTreeSet::new();

        assert!(
            !missing_history_db.exists(),
            "test precondition: missing history db should not exist"
        );

        let error = export_history_jsonl(
            &missing_history_db,
            &payload_path,
            false,
            &mut referenced_recordings,
            &app_handle,
            |_processed, _total| {},
        )
        .expect_err("export should fail when history DB is missing");

        assert!(
            error.contains("Failed to open history database for backup"),
            "expected history open failure, got: {error}"
        );
        assert!(
            !missing_history_db.exists(),
            "history export should not create missing DB files"
        );
        assert!(
            !payload_path.exists(),
            "history payload should not be created when DB open fails"
        );
    }

    #[test]
    fn backup_export_fails_when_recording_payload_exceeds_limit() {
        let (_guard, env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        env.set_payload_limit_bytes(1024);

        seed_history_db(&app_data_dir, &history_rows("recording-limit"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("recording-limit-source", "SOURCE")],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "recording-limit-source" }));
        seed_recordings(
            &app_data_dir,
            &[
                ("recording-limit-1.wav", &[b'X'; 2048]),
                ("recording-limit-2.wav", b"A"),
            ],
        );

        let output = app_data_dir.join("recording-payload-limit.codictatebackup");
        let error = create_backup(
            &app_handle,
            CreateBackupRequest {
                scope: BackupScope::Complete,
                output_path: output.to_string_lossy().to_string(),
            },
        )
        .expect_err("backup export should fail when recording payload exceeds export limit");

        env.clear_payload_limit_bytes();

        assert!(
            error.contains("export_payload_size_limit:recordings/recording-limit-1.wav"),
            "expected recording payload size-limit failure, got: {error}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn complete_backup_rejects_symlinked_recordings_root() {
        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        seed_history_db(&app_data_dir, &history_rows("symlink-root"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("symlink-root-source", "SOURCE")],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "symlink-root-source" }));

        let recordings = app_data_dir.join(RECORDINGS_DIR);
        if recordings.exists() {
            fs::remove_dir_all(&recordings).expect("reset recordings root fixture");
        }

        let outside = app_data_dir.join("outside-recordings");
        fs::create_dir_all(&outside).expect("create outside recordings fixture");
        fs::write(outside.join("symlink-root-1.wav"), b"O")
            .expect("write outside recording fixture");
        symlink(&outside, &recordings).expect("replace recordings root with symlink");

        let output = app_data_dir.join("symlink-root-backup.codictatebackup");
        let error = create_backup(
            &app_handle,
            CreateBackupRequest {
                scope: BackupScope::Complete,
                output_path: output.to_string_lossy().to_string(),
            },
        )
        .expect_err("backup should fail closed for symlinked recordings root");

        assert!(
            error.contains("recordings root") && error.contains("symbolic link"),
            "expected symlinked recordings root failure, got: {error}"
        );
        assert!(
            !output.exists(),
            "backup archive should not be produced for unsafe recordings root"
        );
    }

    #[cfg(unix)]
    #[test]
    fn backup_estimate_rejects_symlinked_recordings_root() {
        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        seed_history_db(&app_data_dir, &history_rows("estimate-symlink-root"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("estimate-symlink-root-source", "SOURCE")],
        );
        seed_user_store(
            &app_data_dir,
            &json!({ "profile": "estimate-symlink-root-source" }),
        );

        let recordings = app_data_dir.join(RECORDINGS_DIR);
        if recordings.exists() {
            fs::remove_dir_all(&recordings).expect("reset recordings root fixture");
        }

        let outside = app_data_dir.join("estimate-outside-recordings");
        fs::create_dir_all(&outside).expect("create outside recordings fixture");
        fs::write(outside.join("estimate-symlink-root-1.wav"), b"O")
            .expect("write outside recording fixture");
        symlink(&outside, &recordings).expect("replace recordings root with symlink");

        let error = get_backup_estimate(&app_handle)
            .expect_err("backup estimate should fail closed for symlinked recordings root");

        assert!(
            error.contains("recordings root") && error.contains("symbolic link"),
            "expected symlinked recordings root estimate failure, got: {error}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn complete_backup_skips_symlink_recordings_with_warning() {
        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        seed_history_db(&app_data_dir, &history_rows("symlink-source"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("symlink-source", "SOURCE")],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "symlink-source" }));

        let recordings = app_data_dir.join(RECORDINGS_DIR);
        fs::create_dir_all(&recordings).expect("create recordings directory");

        let target = app_data_dir.join("symlink-target.wav");
        fs::write(&target, b"T").expect("write symlink target fixture");
        fs::write(recordings.join("symlink-source-2.wav"), b"B")
            .expect("write regular recording fixture");
        symlink(&target, recordings.join("symlink-source-1.wav"))
            .expect("create symlink recording fixture");

        let output = app_data_dir.join("symlink-backup.codictatebackup");
        let report = create_backup(
            &app_handle,
            CreateBackupRequest {
                scope: BackupScope::Complete,
                output_path: output.to_string_lossy().to_string(),
            },
        )
        .expect("create complete backup with symlink recording");

        assert_eq!(report.counts.recording_files, 1);
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.contains("symbolic link")),
            "expected warning for symlink recording file"
        );

        let source = File::open(&output).expect("open backup archive");
        let mut archive = ZipArchive::new(source).expect("parse backup archive");

        let mut files = Vec::new();
        for index in 0..archive.len() {
            let entry = archive.by_index(index).expect("read archive entry");
            if !entry.is_dir() {
                files.push(entry.name().to_string());
            }
        }

        assert!(
            files.contains(&format!("{RECORDINGS_DIR}/symlink-source-2.wav")),
            "regular recording should be included"
        );
        assert!(
            !files.contains(&format!("{RECORDINGS_DIR}/symlink-source-1.wav")),
            "symlink recording should be skipped"
        );
    }

    #[cfg(unix)]
    #[test]
    fn copy_dir_recursive_chunked_rejects_symlink_entries() {
        let root = tempfile::tempdir().expect("create temp root");
        let source = root.path().join("source");
        let destination = root.path().join("destination");
        fs::create_dir_all(&source).expect("create source directory");

        let outside = root.path().join("outside");
        fs::create_dir_all(&outside).expect("create outside directory");
        symlink(&outside, source.join("linked-dir")).expect("create symlinked directory entry");

        let error = copy_dir_recursive_chunked(&source, &destination)
            .expect_err("copy should reject symlink entry");
        assert!(
            error.contains("symbolic link"),
            "unexpected copy failure: {error}"
        );
        assert!(
            !destination.join("linked-dir").exists(),
            "symlink destination should not be materialized"
        );
    }

    #[cfg(unix)]
    #[test]
    fn copy_dir_recursive_chunked_rejects_symlink_source_root() {
        let root = tempfile::tempdir().expect("create temp root");
        let real_source = root.path().join("real-source");
        let source_link = root.path().join("source-link");
        let destination = root.path().join("destination");

        fs::create_dir_all(&real_source).expect("create real source directory");
        fs::write(real_source.join("recording.wav"), b"A").expect("write real source fixture");
        symlink(&real_source, &source_link).expect("create symlinked source root");

        let error = copy_dir_recursive_chunked(&source_link, &destination)
            .expect_err("copy should reject symlink source root");
        assert!(
            error.contains("symbolic link source directory"),
            "unexpected copy failure: {error}"
        );
        assert!(
            !destination.exists(),
            "destination should not be created for symlink source root"
        );
    }

    #[test]
    fn export_recordings_payload_honors_cancel_and_stops_before_all_files() {
        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();
        set_cancel_requested(&app_handle, false);

        let recordings = app_data_dir.join(RECORDINGS_DIR);
        fs::create_dir_all(&recordings).expect("create recordings fixture dir");
        write_large_file(&recordings.join("cancel-first.wav"), 48 * 1024 * 1024);
        fs::write(recordings.join("cancel-second.wav"), b"S").expect("write second recording");

        let workspace = app_data_dir.join("cancel-export-workspace");
        fs::create_dir_all(&workspace).expect("create export workspace");
        let destination_recordings = workspace.join(RECORDINGS_DIR);
        let first_destination = destination_recordings.join("cancel-first.wav");

        let cancel_handle = app_handle.clone();
        let cancel_thread = std::thread::spawn(move || {
            let deadline = std::time::Instant::now() + StdDuration::from_secs(2);
            loop {
                if first_destination
                    .metadata()
                    .map(|meta| meta.len() > 0)
                    .unwrap_or(false)
                {
                    set_cancel_requested(&cancel_handle, true);
                    break;
                }

                if std::time::Instant::now() >= deadline {
                    set_cancel_requested(&cancel_handle, true);
                    break;
                }

                std::thread::sleep(StdDuration::from_millis(2));
            }
        });

        let mut referenced = BTreeSet::new();
        referenced.insert("cancel-first.wav".to_string());
        referenced.insert("cancel-second.wav".to_string());
        let mut warnings = Vec::new();
        let error = export_recordings_payload(
            &app_handle,
            &app_data_dir,
            &workspace,
            &referenced,
            &mut warnings,
            |_processed, _total| {},
        )
        .expect_err("recordings export should be cancelled while copying first file");

        cancel_thread.join().expect("join cancel helper thread");
        assert!(
            error.contains("cancelled safely"),
            "expected cancellation error, got: {error}"
        );
        assert!(
            !destination_recordings.join("cancel-second.wav").exists(),
            "export should stop before copying all referenced recordings"
        );

        set_cancel_requested(&app_handle, false);
    }

    #[test]
    fn package_workspace_to_archive_honors_cancel_and_removes_temp_output() {
        let (_guard, _env, _app, app_data_dir) = setup_test_app();

        let workspace = app_data_dir.join("package-cancel-workspace");
        fs::create_dir_all(workspace.join("history")).expect("create package workspace history dir");
        let history_path = workspace.join(HISTORY_FILE);
        write_large_file(&history_path, 2 * 1024 * 1024);

        let history_checksum = checksum_path(&history_path).expect("checksum history payload");
        fs::write(
            workspace.join(CHECKSUM_FILE),
            format!("{history_checksum}  {HISTORY_FILE}\n"),
        )
        .expect("write checksum payload");

        let output_path = app_data_dir.join("package-cancel-output.codictatebackup");
        if output_path.exists() {
            fs::remove_file(&output_path).expect("remove stale package-cancel output");
        }
        let temp_output = output_path.with_extension(format!("{}.tmp", BACKUP_FILE_EXTENSION));
        if temp_output.exists() {
            fs::remove_file(&temp_output).expect("remove stale package-cancel temp output");
        }

        let mut cancel_check_calls = 0_u32;
        let error = package_workspace_to_archive_with_cancel(
            &workspace,
            &output_path,
            || {
                cancel_check_calls = cancel_check_calls.saturating_add(1);
                if cancel_check_calls >= 4 {
                    Err("Backup/restore was cancelled safely.".to_string())
                } else {
                    Ok(())
                }
            },
            |_bytes_written, _total_bytes| {},
        )
        .expect_err("package should stop once cancellation is requested");

        assert!(
            error.contains("cancelled safely"),
            "expected cancellation error from package step, got: {error}"
        );
        assert!(
            !output_path.exists(),
            "final archive should not exist when packaging is cancelled"
        );
        assert!(
            !temp_output.exists(),
            "temporary archive should be removed when packaging is cancelled"
        );
    }

    #[test]
    fn package_workspace_to_archive_reports_monotonic_progress() {
        let (_guard, _env, _app, app_data_dir) = setup_test_app();

        let workspace = app_data_dir.join("package-progress-workspace");
        fs::create_dir_all(workspace.join("history")).expect("create package workspace history dir");
        let history_path = workspace.join(HISTORY_FILE);
        write_large_file(&history_path, 3 * 1024 * 1024);

        let history_checksum = checksum_path(&history_path).expect("checksum history payload");
        fs::write(
            workspace.join(CHECKSUM_FILE),
            format!("{history_checksum}  {HISTORY_FILE}\n"),
        )
        .expect("write checksum payload");

        let output_path = app_data_dir.join("package-progress-output.codictatebackup");
        if output_path.exists() {
            fs::remove_file(&output_path).expect("remove stale package-progress output");
        }
        let temp_output = output_path.with_extension(format!("{}.tmp", BACKUP_FILE_EXTENSION));
        if temp_output.exists() {
            fs::remove_file(&temp_output).expect("remove stale package-progress temp output");
        }

        let mut progress_events = Vec::<(u64, u64)>::new();
        package_workspace_to_archive_with_cancel(
            &workspace,
            &output_path,
            || Ok(()),
            |bytes_written, total_bytes| {
                progress_events.push((bytes_written, total_bytes));
            },
        )
        .expect("package should succeed");

        assert!(output_path.exists(), "packaged archive should exist");
        assert!(!progress_events.is_empty(), "progress callback should be invoked");

        let total_bytes = progress_events[0].1;
        let mut previous = 0_u64;
        for (bytes_written, event_total) in &progress_events {
            assert_eq!(*event_total, total_bytes, "total bytes should remain stable");
            assert!(
                *bytes_written >= previous,
                "progress bytes must be monotonic"
            );
            assert!(
                *bytes_written <= total_bytes,
                "progress bytes must not exceed total bytes"
            );
            previous = *bytes_written;
        }

        assert_eq!(
            progress_events
                .last()
                .expect("progress events should include a final update")
                .0,
            total_bytes,
            "final progress event must report completion"
        );
    }

    #[test]
    fn stage_progress_units_are_bounded_and_monotonic() {
        let start = 100_u64;
        let end = 1_200_u64;
        let total = 10_u64;

        assert_eq!(map_stage_progress_units(start, end, 0, total), start);
        assert_eq!(map_stage_progress_units(start, end, total, total), end);
        assert_eq!(
            map_stage_progress_units(start, end, total.saturating_mul(2), total),
            end
        );

        let mut previous = start;
        for processed in 0..=total {
            let current = map_stage_progress_units(start, end, processed, total);
            assert!(current >= previous, "stage progress must be monotonic");
            assert!(
                current >= start && current <= end,
                "stage progress should stay within stage span"
            );
            previous = current;
        }
    }

    #[test]
    fn package_progress_units_follow_new_package_span() {
        let total = 1_000_u64;
        assert_eq!(package_progress_units(0, total), 4_500);
        assert_eq!(package_progress_units(total, total), 9_900);
        assert_eq!(package_progress_units(total.saturating_mul(2), total), 9_900);

        let mut previous = package_progress_units(0, total);
        for written in [100_u64, 250, 500, 750, 1_000] {
            let current = package_progress_units(written, total);
            assert!(current >= previous, "package progress should be monotonic");
            assert!(
                current >= 4_500 && current <= 9_900,
                "package progress should stay within package stage span"
            );
            previous = current;
        }
    }

