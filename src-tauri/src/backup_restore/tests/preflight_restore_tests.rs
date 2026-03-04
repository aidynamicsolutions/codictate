    #[test]
    fn preflight_rejects_checksum_mismatch() {
        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        seed_history_db(&app_data_dir, &history_rows("checksum-source"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("checksum-alpha", "ALPHA")],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "checksum-source" }));
        seed_recordings(&app_data_dir, &[("checksum-source.wav", b"A")]);

        let archive_path = make_backup(&app_handle, BackupScope::Complete, "checksum-mismatch");
        tamper_archive_file(&archive_path, HISTORY_FILE, |mut bytes| {
            bytes.push(b' ');
            bytes
        });

        let report = preflight_restore(
            &app_handle,
            PreflightRestoreRequest {
                archive_path: archive_path.to_string_lossy().to_string(),
            },
        )
        .expect("preflight restore");

        assert!(!report.can_apply);
        assert!(
            report
                .blocking_findings
                .iter()
                .any(|finding| finding.code == "checksum_mismatch"),
            "expected checksum mismatch finding"
        );
    }

    #[test]
    fn apply_restore_skips_extension_only_recoverable_warning() {
        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        seed_history_db(&app_data_dir, &history_rows("extension-warning-source"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("extension-warning-alpha", "ALPHA")],
        );
        seed_user_store(
            &app_data_dir,
            &json!({ "profile": "extension-warning-source" }),
        );
        seed_recordings(&app_data_dir, &[("extension-warning-source-1.wav", b"A")]);

        let archive_path = make_backup(
            &app_handle,
            BackupScope::Complete,
            "extension-warning-backup",
        );
        let renamed_archive_path = archive_path.with_extension("zip");
        if renamed_archive_path.exists() {
            fs::remove_file(&renamed_archive_path).expect("remove stale renamed backup");
        }
        fs::rename(&archive_path, &renamed_archive_path)
            .expect("rename backup with non-standard extension");

        let preflight = preflight_restore(
            &app_handle,
            PreflightRestoreRequest {
                archive_path: renamed_archive_path.to_string_lossy().to_string(),
            },
        )
        .expect("preflight restore for renamed archive");
        assert!(
            preflight.can_apply,
            "extension-only mismatch should remain recoverable"
        );
        assert!(
            preflight
                .recoverable_findings
                .iter()
                .any(|finding| finding.code == "archive_extension_unexpected"),
            "preflight should record extension mismatch finding"
        );

        let report = apply_restore(
            &app_handle,
            ApplyRestoreRequest {
                archive_path: renamed_archive_path.to_string_lossy().to_string(),
            },
        )
        .expect("apply restore for renamed archive");
        assert!(
            report.warnings.is_empty(),
            "extension-only mismatch should not surface as apply warning"
        );
    }

    #[test]
    fn preflight_returns_blocking_finding_for_non_zip_archive() {
        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        let archive_path = app_data_dir.join("not-a-zip.codictatebackup");
        fs::write(&archive_path, b"this is not a zip archive").expect("write invalid archive bytes");

        let report = preflight_restore(
            &app_handle,
            PreflightRestoreRequest {
                archive_path: archive_path.to_string_lossy().to_string(),
            },
        )
        .expect("preflight restore should return structured report");

        assert!(!report.can_apply);
        assert!(
            report
                .blocking_findings
                .iter()
                .any(|finding| finding.code == "archive_parse_failed"),
            "expected archive_parse_failed blocking finding"
        );
    }

    #[test]
    fn preflight_fails_fast_when_archive_entry_limit_is_exceeded() {
        let (_guard, env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        seed_history_db(&app_data_dir, &history_rows("entry-limit-source"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("entry-limit-source", "SOURCE")],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "entry-limit-source" }));
        seed_recordings(&app_data_dir, &[("entry-limit-source.wav", b"A")]);

        let archive_path = make_backup(&app_handle, BackupScope::Complete, "entry-limit-backup");

        env.set_archive_entries_limit(3);
        let report = preflight_restore(
            &app_handle,
            PreflightRestoreRequest {
                archive_path: archive_path.to_string_lossy().to_string(),
            },
        )
        .expect("preflight restore should return structured report");
        env.clear_archive_entries_limit();

        assert!(!report.can_apply, "entry-count overflow must block restore");
        assert_eq!(
            report.blocking_findings.len(),
            1,
            "preflight should fail fast once entry-count limit is breached"
        );
        assert_eq!(report.blocking_findings[0].code, "archive_entry_limit");
        assert!(
            report.summary.is_none(),
            "summary should stay unset on fast-fail path"
        );
    }

    #[test]
    fn preflight_rejects_archive_path_prefix_conflicts() {
        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        seed_history_db(&app_data_dir, &history_rows("prefix-conflict-source"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("prefix-conflict-alpha", "ALPHA")],
        );
        seed_user_store(
            &app_data_dir,
            &json!({ "profile": "prefix-conflict-source" }),
        );
        seed_recordings(
            &app_data_dir,
            &[
                ("prefix-conflict-source-1.wav", b"A"),
                ("prefix-conflict-source-2.wav", b"B"),
            ],
        );

        let archive_path = make_backup(&app_handle, BackupScope::Complete, "prefix-conflict");

        let original_recording_path = format!("{RECORDINGS_DIR}/prefix-conflict-source-1.wav");
        rename_archive_entry(&archive_path, &original_recording_path, RECORDINGS_DIR);

        let renamed_checksum = archive_entry_checksum(&archive_path, RECORDINGS_DIR);
        tamper_archive_file(&archive_path, CHECKSUM_FILE, |bytes| {
            let text = String::from_utf8(bytes).expect("checksums payload should be valid UTF-8");
            replace_checksum_path_line(
                &text,
                &original_recording_path,
                RECORDINGS_DIR,
                &renamed_checksum,
            )
            .into_bytes()
        });

        let report = preflight_restore(
            &app_handle,
            PreflightRestoreRequest {
                archive_path: archive_path.to_string_lossy().to_string(),
            },
        )
        .expect("preflight restore");

        assert!(!report.can_apply, "path-prefix conflicts must be blocking");
        assert!(
            report
                .blocking_findings
                .iter()
                .any(|finding| finding.code == "archive_path_conflict"),
            "expected archive_path_conflict blocking finding"
        );
    }

    #[test]
    fn preflight_rejects_archive_paths_with_control_characters() {
        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        let rows = history_rows("archive-control-char-source");
        seed_history_db(&app_data_dir, &rows);
        seed_dictionary(
            &app_handle,
            vec![custom_word("archive-control-char-alpha", "ALPHA")],
        );
        seed_user_store(
            &app_data_dir,
            &json!({ "profile": "archive-control-char-source" }),
        );
        seed_recordings(&app_data_dir, &[("archive-control-char-source-1.wav", b"A")]);

        let archive_path = make_backup(
            &app_handle,
            BackupScope::Complete,
            "archive-control-char-path",
        );
        let original_recording_path = format!("{RECORDINGS_DIR}/{}", rows[0].file_name);
        let control_char_path = format!("{RECORDINGS_DIR}/archive-\ncontrol.wav");
        rename_archive_entry(&archive_path, &original_recording_path, &control_char_path);

        let report = preflight_restore(
            &app_handle,
            PreflightRestoreRequest {
                archive_path: archive_path.to_string_lossy().to_string(),
            },
        )
        .expect("preflight restore");

        assert!(!report.can_apply, "control-char archive entry must block restore");
        assert!(
            report
                .blocking_findings
                .iter()
                .any(|finding| finding.code == "archive_unsafe_path"),
            "expected archive_unsafe_path blocking finding"
        );
    }

    #[test]
    fn preflight_blocks_when_local_history_data_is_corrupted() {
        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        seed_history_db(&app_data_dir, &history_rows("local-integrity-source"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("local-integrity-alpha", "ALPHA")],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "local-integrity-source" }));
        seed_recordings(&app_data_dir, &[("local-integrity-source.wav", b"A")]);

        let archive_path = make_backup(
            &app_handle,
            BackupScope::Complete,
            "local-history-integrity-block",
        );

        fs::write(app_data_dir.join(HISTORY_DB_FILE), b"not-a-sqlite-db")
            .expect("corrupt local history db");

        let report = preflight_restore(
            &app_handle,
            PreflightRestoreRequest {
                archive_path: archive_path.to_string_lossy().to_string(),
            },
        )
        .expect("preflight restore");

        assert!(!report.can_apply);
        assert!(
            report
                .blocking_findings
                .iter()
                .any(|finding| finding.code == "local_history_data_corrupted"),
            "expected local_history_data_corrupted blocking finding"
        );
    }

    #[test]
    fn preflight_and_extract_use_already_open_archive_handle() {
        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        seed_history_db(&app_data_dir, &history_rows("handle-source"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("handle-source-alpha", "ALPHA")],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "handle-source" }));
        seed_recordings(&app_data_dir, &[("handle-source.wav", b"A")]);

        let archive_path = make_backup(&app_handle, BackupScope::Complete, "handle-backup");
        let mut archive_handle = File::open(&archive_path).expect("open archive handle");

        // Replace the archive at its path after opening the original handle.
        // Preflight and extraction should continue to use the already-open file.
        let replacement_path = app_data_dir.join("handle-replaced.codictatebackup");
        fs::write(&replacement_path, b"not a zip archive")
            .expect("write replacement archive payload");
        fs::rename(&replacement_path, &archive_path)
            .expect("replace archive path with unrelated file");

        let preflight =
            build_preflight_context_with_open_archive(&app_handle, &archive_path, &mut archive_handle)
                .expect("preflight with open handle");
        assert!(
            preflight.report.can_apply,
            "preflight should succeed because it uses the opened archive handle"
        );

        let extract_dir = app_data_dir.join("extract-with-open-handle");
        if extract_dir.exists() {
            fs::remove_dir_all(&extract_dir).expect("remove stale extraction directory");
        }
        fs::create_dir_all(&extract_dir).expect("create extraction directory");

        extract_archive(&app_handle, &mut archive_handle, &extract_dir)
            .expect("extract should use opened archive handle");
        assert!(
            extract_dir.join(MANIFEST_FILE).exists(),
            "expected extracted manifest from original archive handle"
        );
    }

    #[test]
    fn extract_entry_bounded_honors_cancel_during_chunk_stream() {
        struct SlowReader {
            remaining: usize,
            chunk_size: usize,
            sleep_per_chunk: StdDuration,
        }

        impl Read for SlowReader {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                if self.remaining == 0 {
                    return Ok(0);
                }

                std::thread::sleep(self.sleep_per_chunk);
                let take = self.remaining.min(self.chunk_size).min(buf.len());
                buf[..take].fill(b'R');
                self.remaining -= take;
                Ok(take)
            }
        }

        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();
        set_cancel_requested(&app_handle, false);

        let cancel_handle = app_handle.clone();
        let cancel_thread = std::thread::spawn(move || {
            std::thread::sleep(StdDuration::from_millis(20));
            set_cancel_requested(&cancel_handle, true);
        });

        let mut reader = SlowReader {
            remaining: 8 * 1024 * 1024,
            chunk_size: 256 * 1024,
            sleep_per_chunk: StdDuration::from_millis(5),
        };
        let output_path = app_data_dir.join("extract-cancelled.bin");
        let mut total_extracted_bytes = 0_u64;

        let error = extract_entry_bounded(
            &app_handle,
            &mut reader,
            &output_path,
            "recordings/extract-cancelled.wav",
            &mut total_extracted_bytes,
        )
        .expect_err("extract should stop once cancellation is requested");

        cancel_thread.join().expect("join cancel helper thread");
        assert!(
            error.contains("cancelled safely"),
            "expected cancellation failure from extract loop, got: {error}"
        );
        let written_len = fs::metadata(&output_path)
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        assert!(
            written_len < (8 * 1024 * 1024) as u64,
            "cancelled extraction should not write full payload"
        );
        assert!(
            total_extracted_bytes < (8 * 1024 * 1024) as u64,
            "cancelled extraction should stop before the full stream is consumed"
        );

        set_cancel_requested(&app_handle, false);
    }

    #[test]
    fn restore_cancel_during_recordings_import_keeps_active_data_unchanged() {
        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();
        set_cancel_requested(&app_handle, false);

        seed_history_db(&app_data_dir, &history_rows("restore-recordings-cancel-source"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("restore-recordings-cancel-source", "SOURCE")],
        );
        seed_user_store(
            &app_data_dir,
            &json!({ "profile": "restore-recordings-cancel-source" }),
        );

        let source_recordings_dir = app_data_dir.join(RECORDINGS_DIR);
        fs::create_dir_all(&source_recordings_dir).expect("create source recordings directory");
        write_large_file(
            &source_recordings_dir.join("restore-recordings-cancel-source-1.wav"),
            48 * 1024 * 1024,
        );
        fs::write(
            source_recordings_dir.join("restore-recordings-cancel-source-2.wav"),
            b"S",
        )
        .expect("write secondary source recording");

        let archive_path = make_backup(
            &app_handle,
            BackupScope::Complete,
            "restore-recordings-cancel-backup",
        );

        seed_history_db(&app_data_dir, &history_rows("restore-recordings-cancel-active"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("restore-recordings-cancel-active", "ACTIVE")],
        );
        seed_user_store(
            &app_data_dir,
            &json!({ "profile": "restore-recordings-cancel-active" }),
        );
        seed_recordings(
            &app_data_dir,
            &[("restore-recordings-cancel-active.wav", b"A")],
        );
        let expected_state = snapshot_state(&app_data_dir);

        let runtime_path = runtime_dir(&app_data_dir);
        let cancel_handle = app_handle.clone();
        let cancel_thread = std::thread::spawn(move || {
            let deadline = std::time::Instant::now() + StdDuration::from_secs(5);
            loop {
                let mut detected_partial_recordings_copy = false;
                if let Ok(work_dirs) = fs::read_dir(&runtime_path) {
                    for work_dir in work_dirs.flatten() {
                        if !work_dir
                            .file_name()
                            .to_string_lossy()
                            .starts_with("restore-work-")
                        {
                            continue;
                        }

                        let copied_recording = work_dir.path().join(format!(
                            "new-data/{RECORDINGS_DIR}/restore-recordings-cancel-source-1.wav"
                        ));
                        if copied_recording
                            .metadata()
                            .map(|metadata| metadata.len() > 0)
                            .unwrap_or(false)
                        {
                            detected_partial_recordings_copy = true;
                            break;
                        }
                    }
                }

                if detected_partial_recordings_copy || std::time::Instant::now() >= deadline {
                    set_cancel_requested(&cancel_handle, true);
                    break;
                }

                std::thread::sleep(StdDuration::from_millis(2));
            }
        });

        let error = apply_restore(
            &app_handle,
            ApplyRestoreRequest {
                archive_path: archive_path.to_string_lossy().to_string(),
            },
        )
        .expect_err("restore should stop once cancellation is requested during recordings import");

        cancel_thread.join().expect("join restore cancel helper thread");
        assert!(
            error.contains("cancelled safely"),
            "expected cancellation error from restore recordings import, got: {error}"
        );
        assert_eq!(
            snapshot_state(&app_data_dir),
            expected_state,
            "restore cancellation before swap should keep active data unchanged"
        );
        assert!(
            !marker_path(&app_data_dir).exists(),
            "restore marker must not be written when cancellation occurs before swap"
        );

        set_cancel_requested(&app_handle, false);
    }

    #[test]
    fn restore_extract_enforces_stream_entry_limit() {
        let (_guard, env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        seed_history_db(&app_data_dir, &history_rows("extract-entry-source"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("extract-entry-source", "SOURCE")],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "extract-entry-source" }));
        seed_recordings(
            &app_data_dir,
            &[
                ("extract-entry-source-1.wav", b"A"),
                ("extract-entry-source-2.wav", b"B"),
            ],
        );

        let archive_path = make_backup(
            &app_handle,
            BackupScope::Complete,
            "extract-entry-limit-backup",
        );

        seed_history_db(&app_data_dir, &history_rows("extract-entry-mutated"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("extract-entry-mutated", "MUTATED")],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "extract-entry-mutated" }));
        seed_recordings(&app_data_dir, &[("extract-entry-mutated.wav", b"M")]);
        let expected_state = snapshot_state(&app_data_dir);

        env.set_extract_payload_limit_bytes(64);
        let error = apply_restore(
            &app_handle,
            ApplyRestoreRequest {
                archive_path: archive_path.to_string_lossy().to_string(),
            },
        )
        .expect_err("restore should fail when extracted payload exceeds stream entry limit");
        env.clear_extract_payload_limit_bytes();

        assert!(
            error.contains("archive_payload_size_limit_extracted:"),
            "expected extraction entry size-limit failure, got: {error}"
        );
        assert_eq!(snapshot_state(&app_data_dir), expected_state);
        assert!(
            !marker_path(&app_data_dir).exists(),
            "restore marker must not be written when extraction fails"
        );
    }

    #[test]
    fn restore_extract_enforces_stream_total_limit() {
        let (_guard, env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        seed_history_db(&app_data_dir, &history_rows("extract-total-source"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("extract-total-source", "SOURCE")],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "extract-total-source" }));
        seed_recordings(
            &app_data_dir,
            &[
                ("extract-total-source-1.wav", b"A"),
                ("extract-total-source-2.wav", b"B"),
            ],
        );

        let archive_path = make_backup(
            &app_handle,
            BackupScope::Complete,
            "extract-total-limit-backup",
        );

        seed_history_db(&app_data_dir, &history_rows("extract-total-mutated"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("extract-total-mutated", "MUTATED")],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "extract-total-mutated" }));
        seed_recordings(&app_data_dir, &[("extract-total-mutated.wav", b"M")]);
        let expected_state = snapshot_state(&app_data_dir);

        env.set_extract_total_limit_bytes(128);
        let error = apply_restore(
            &app_handle,
            ApplyRestoreRequest {
                archive_path: archive_path.to_string_lossy().to_string(),
            },
        )
        .expect_err("restore should fail when extracted payload exceeds stream total limit");
        env.clear_extract_total_limit_bytes();

        assert!(
            error.contains("archive_uncompressed_limit_extracted:"),
            "expected extraction total size-limit failure, got: {error}"
        );
        assert_eq!(snapshot_state(&app_data_dir), expected_state);
        assert!(
            !marker_path(&app_data_dir).exists(),
            "restore marker must not be written when extraction fails"
        );
    }

    #[test]
    fn preflight_rejects_missing_payload_checksum() {
        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        seed_history_db(&app_data_dir, &history_rows("checksum-missing-source"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("checksum-missing-alpha", "ALPHA")],
        );
        seed_user_store(
            &app_data_dir,
            &json!({ "profile": "checksum-missing-source" }),
        );
        seed_recordings(&app_data_dir, &[("checksum-missing-source.wav", b"A")]);

        let archive_path = make_backup(
            &app_handle,
            BackupScope::Complete,
            "checksum-missing-payload",
        );

        rewrite_archive(&archive_path, |name, bytes| {
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

        let report = preflight_restore(
            &app_handle,
            PreflightRestoreRequest {
                archive_path: archive_path.to_string_lossy().to_string(),
            },
        )
        .expect("preflight restore");

        assert!(!report.can_apply);
        assert!(
            report
                .blocking_findings
                .iter()
                .any(|finding| finding.code == "checksum_missing_payload"),
            "expected checksum_missing_payload finding"
        );
    }

    #[test]
    fn preflight_rejects_checksum_paths_with_control_characters() {
        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        seed_history_db(&app_data_dir, &history_rows("checksum-control-char-source"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("checksum-control-char-alpha", "ALPHA")],
        );
        seed_user_store(
            &app_data_dir,
            &json!({ "profile": "checksum-control-char-source" }),
        );
        seed_recordings(
            &app_data_dir,
            &[("checksum-control-char-source.wav", b"A")],
        );

        let archive_path = make_backup(
            &app_handle,
            BackupScope::Complete,
            "checksum-control-char-path",
        );
        tamper_archive_file(&archive_path, CHECKSUM_FILE, |bytes| {
            let text = String::from_utf8(bytes).expect("checksums payload should be valid UTF-8");
            replace_checksum_path_line(
                &text,
                HISTORY_FILE,
                "history/\u{007f}history.jsonl",
                "0".repeat(64).as_str(),
            )
            .into_bytes()
        });

        let report = preflight_restore(
            &app_handle,
            PreflightRestoreRequest {
                archive_path: archive_path.to_string_lossy().to_string(),
            },
        )
        .expect("preflight restore");

        assert!(!report.can_apply, "control-char checksum path must block restore");
        assert!(
            report
                .blocking_findings
                .iter()
                .any(|finding| finding.code == "checksum_parse_failed"),
            "expected checksum_parse_failed finding"
        );
    }

    #[test]
    fn preflight_rejects_history_file_name_traversal() {
        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        seed_history_db(&app_data_dir, &history_rows("traversal-source"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("traversal-alpha", "ALPHA")],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "traversal-source" }));
        seed_recordings(
            &app_data_dir,
            &[("traversal-source-1.wav", b"A"), ("traversal-source-2.wav", b"B")],
        );

        let archive_path = make_backup(&app_handle, BackupScope::Complete, "traversal-backup");

        tamper_archive_file(&archive_path, HISTORY_FILE, |bytes| {
            let text = String::from_utf8(bytes).expect("history payload should be valid UTF-8");
            let mut rows = text
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(|line| serde_json::from_str::<HistoryRowV1>(line).expect("valid history row"))
                .collect::<Vec<_>>();
            rows[0].file_name = "../outside.wav".to_string();
            let body = rows
                .into_iter()
                .map(|row| serde_json::to_string(&row).expect("serialize history row"))
                .collect::<Vec<_>>()
                .join("\n");
            format!("{body}\n").into_bytes()
        });

        let history_checksum = archive_entry_checksum(&archive_path, HISTORY_FILE);
        tamper_archive_file(&archive_path, CHECKSUM_FILE, |bytes| {
            let text = String::from_utf8(bytes).expect("checksums payload should be valid UTF-8");
            replace_checksum_line(&text, HISTORY_FILE, &history_checksum).into_bytes()
        });

        let report = preflight_restore(
            &app_handle,
            PreflightRestoreRequest {
                archive_path: archive_path.to_string_lossy().to_string(),
            },
        )
        .expect("preflight restore");

        assert!(!report.can_apply);
        assert!(
            report
                .blocking_findings
                .iter()
                .any(|finding| finding.code == "history_payload_invalid"),
            "expected history payload validation finding"
        );
    }

    #[test]
    fn preflight_rejects_history_line_size_limit() {
        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        seed_history_db(&app_data_dir, &history_rows("line-limit-source"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("line-limit-source", "SOURCE")],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "line-limit-source" }));

        let archive_path = make_backup(&app_handle, BackupScope::Smaller, "line-limit-backup");

        tamper_archive_file(&archive_path, HISTORY_FILE, |bytes| {
            let text = String::from_utf8(bytes).expect("history payload should be valid UTF-8");
            let mut rows = text
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(|line| serde_json::from_str::<HistoryRowV1>(line).expect("valid history row"))
                .collect::<Vec<_>>();
            rows[0].transcription_text =
                "X".repeat(MAX_HISTORY_JSONL_LINE_BYTES.saturating_add(1));
            let body = rows
                .into_iter()
                .map(|row| serde_json::to_string(&row).expect("serialize history row"))
                .collect::<Vec<_>>()
                .join("\n");
            format!("{body}\n").into_bytes()
        });

        let history_checksum = archive_entry_checksum(&archive_path, HISTORY_FILE);
        tamper_archive_file(&archive_path, CHECKSUM_FILE, |bytes| {
            let text = String::from_utf8(bytes).expect("checksums payload should be valid UTF-8");
            replace_checksum_line(&text, HISTORY_FILE, &history_checksum).into_bytes()
        });

        let report = preflight_restore(
            &app_handle,
            PreflightRestoreRequest {
                archive_path: archive_path.to_string_lossy().to_string(),
            },
        )
        .expect("preflight restore");

        assert!(!report.can_apply);
        assert!(
            report
                .blocking_findings
                .iter()
                .any(|finding| finding.code == "history_line_size_limit"),
            "expected history_line_size_limit finding"
        );
    }

    #[test]
    fn restore_rejects_history_line_size_limit_before_swap() {
        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        seed_history_db(&app_data_dir, &history_rows("restore-line-source"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("restore-line-source", "SOURCE")],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "restore-line-source" }));
        seed_recordings(&app_data_dir, &[("restore-line-source-1.wav", b"A")]);
        let archive_path = make_backup(&app_handle, BackupScope::Complete, "restore-line-backup");

        tamper_archive_file(&archive_path, HISTORY_FILE, |bytes| {
            let text = String::from_utf8(bytes).expect("history payload should be valid UTF-8");
            let mut rows = text
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(|line| serde_json::from_str::<HistoryRowV1>(line).expect("valid history row"))
                .collect::<Vec<_>>();
            rows[0].transcription_text =
                "Y".repeat(MAX_HISTORY_JSONL_LINE_BYTES.saturating_add(1));
            let body = rows
                .into_iter()
                .map(|row| serde_json::to_string(&row).expect("serialize history row"))
                .collect::<Vec<_>>()
                .join("\n");
            format!("{body}\n").into_bytes()
        });

        let history_checksum = archive_entry_checksum(&archive_path, HISTORY_FILE);
        tamper_archive_file(&archive_path, CHECKSUM_FILE, |bytes| {
            let text = String::from_utf8(bytes).expect("checksums payload should be valid UTF-8");
            replace_checksum_line(&text, HISTORY_FILE, &history_checksum).into_bytes()
        });

        seed_history_db(&app_data_dir, &history_rows("restore-line-mutated"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("restore-line-mutated", "MUTATED")],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "restore-line-mutated" }));
        seed_recordings(&app_data_dir, &[("restore-line-mutated-1.wav", b"M")]);
        let expected_state = snapshot_state(&app_data_dir);

        let error = apply_restore(
            &app_handle,
            ApplyRestoreRequest {
                archive_path: archive_path.to_string_lossy().to_string(),
            },
        )
        .expect_err("restore should reject oversized history line before data swap");

        assert!(
            error.contains("history_jsonl_line_too_large:"),
            "expected oversized history-line restore error, got: {error}"
        );
        assert_eq!(snapshot_state(&app_data_dir), expected_state);
    }

    #[test]
    fn backup_rejects_invalid_history_file_name() {
        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        seed_history_db(&app_data_dir, &history_rows("invalid-file-name"));
        let db_path = app_data_dir.join(HISTORY_DB_FILE);
        let conn = Connection::open(&db_path).expect("open history db for invalid file name");
        conn.execute(
            "UPDATE transcription_history SET file_name = ?1 WHERE id = 1",
            params!["../outside.wav"],
        )
        .expect("mutate history row file_name");

        seed_dictionary(
            &app_handle,
            vec![custom_word("invalid-file-name", "SAFE")],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "invalid-file-name" }));

        let archive_path = app_data_dir.join("invalid-file-name-backup.codictatebackup");
        let result = create_backup(
            &app_handle,
            CreateBackupRequest {
                scope: BackupScope::Smaller,
                output_path: archive_path.to_string_lossy().to_string(),
            },
        );

        let error = result.expect_err("backup should reject invalid history file_name");
        assert!(
            error.contains("Invalid history row file_name"),
            "unexpected backup failure: {error}"
        );
    }

    #[test]
    fn backup_rejects_control_char_history_file_name() {
        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        seed_history_db(&app_data_dir, &history_rows("invalid-control-char-file-name"));
        let db_path = app_data_dir.join(HISTORY_DB_FILE);
        let conn = Connection::open(&db_path).expect("open history db for invalid file name");
        conn.execute(
            "UPDATE transcription_history SET file_name = ?1 WHERE id = 1",
            params!["bad\nname.wav"],
        )
        .expect("mutate history row file_name");

        seed_dictionary(
            &app_handle,
            vec![custom_word("invalid-control-char-file-name", "SAFE")],
        );
        seed_user_store(
            &app_data_dir,
            &json!({ "profile": "invalid-control-char-file-name" }),
        );

        let archive_path = app_data_dir.join("invalid-control-char-file-name.codictatebackup");
        let result = create_backup(
            &app_handle,
            CreateBackupRequest {
                scope: BackupScope::Smaller,
                output_path: archive_path.to_string_lossy().to_string(),
            },
        );

        let error = result.expect_err("backup should reject control-char history file_name");
        assert!(
            error.contains("control characters are not allowed"),
            "unexpected backup failure: {error}"
        );
    }

    #[test]
    fn preflight_treats_non_object_user_store_as_recoverable() {
        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        seed_history_db(&app_data_dir, &history_rows("invalid-user-store-source"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("invalid-user-store-source", "SOURCE")],
        );
        seed_user_store(
            &app_data_dir,
            &json!({ "profile": "invalid-user-store-source" }),
        );

        let archive_path = make_backup(
            &app_handle,
            BackupScope::Smaller,
            "invalid-user-store-preflight",
        );

        tamper_archive_file(&archive_path, USER_STORE_FILE, |_| b"[]".to_vec());
        let user_store_checksum = archive_entry_checksum(&archive_path, USER_STORE_FILE);
        tamper_archive_file(&archive_path, CHECKSUM_FILE, |bytes| {
            let text = String::from_utf8(bytes).expect("checksums payload should be valid UTF-8");
            replace_checksum_line(&text, USER_STORE_FILE, &user_store_checksum).into_bytes()
        });

        let report = preflight_restore(
            &app_handle,
            PreflightRestoreRequest {
                archive_path: archive_path.to_string_lossy().to_string(),
            },
        )
        .expect("preflight restore");

        assert!(report.can_apply, "invalid user_store shape should be recoverable");
        assert_eq!(
            report.compatibility_note_code,
            PreflightCompatibilityNoteCode::V1MacosGuaranteedCrossPlatformBestEffort
        );
        assert!(
            !report.compatibility_note.trim().is_empty(),
            "compatibility note fallback text should be populated"
        );
        assert!(
            report
                .recoverable_findings
                .iter()
                .any(|finding| finding.code == "user_store_payload_recoverable"
                    && finding.message.contains("invalid structure")),
            "expected recoverable invalid-structure finding"
        );
    }

    #[test]
    fn missing_user_stats_payload_is_recoverable_and_restore_warns() {
        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        seed_history_db(&app_data_dir, &history_rows("missing-user-stats-source"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("missing-user-stats-source", "SOURCE")],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "missing-user-stats-source" }));

        let archive_path = make_backup(
            &app_handle,
            BackupScope::Smaller,
            "missing-user-stats-preflight",
        );
        rewrite_archive(&archive_path, |name, bytes| {
            if name == HISTORY_USER_STATS_FILE {
                return None;
            }

            if name == CHECKSUM_FILE {
                let text = String::from_utf8(bytes.to_vec())
                    .expect("checksums payload should be valid UTF-8");
                let filtered = text
                    .lines()
                    .filter(|line| !line.trim_end().ends_with(HISTORY_USER_STATS_FILE))
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

        let preflight = preflight_restore(
            &app_handle,
            PreflightRestoreRequest {
                archive_path: archive_path.to_string_lossy().to_string(),
            },
        )
        .expect("preflight restore with missing user_stats payload");
        assert!(preflight.can_apply, "missing user_stats payload should be recoverable");
        assert!(
            preflight
                .recoverable_findings
                .iter()
                .any(|finding| finding.code == "user_stats_missing_recoverable"),
            "expected missing user_stats recoverable finding"
        );

        let report = apply_restore(
            &app_handle,
            ApplyRestoreRequest {
                archive_path: archive_path.to_string_lossy().to_string(),
            },
        )
        .expect("apply restore with missing user_stats payload");
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.contains(HISTORY_USER_STATS_FILE)),
            "expected restore warning that canonical user_stats payload was missing"
        );
    }

