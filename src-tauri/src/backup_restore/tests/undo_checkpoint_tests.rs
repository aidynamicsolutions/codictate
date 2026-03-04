    #[test]
    fn undo_availability_rejects_checkpoint_outside_runtime() {
        let (_guard, env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        let outside_snapshot = env.root.path().join("outside-undo");
        fs::create_dir_all(&outside_snapshot).expect("create outside undo snapshot dir");
        let sentinel = outside_snapshot.join("sentinel.txt");
        fs::write(&sentinel, b"keep").expect("write outside undo snapshot sentinel");

        durable_write_json(
            &undo_checkpoint_meta_path(&app_data_dir),
            &UndoCheckpointMeta {
                snapshot_path: outside_snapshot.to_string_lossy().to_string(),
                created_at: now_rfc3339(),
                expires_at: (Utc::now() + Duration::days(1)).to_rfc3339(),
                snapshot_layout: Some(UndoCheckpointSnapshotLayout {
                    history_db: true,
                    user_dictionary: true,
                    user_store: true,
                    recordings_dir: true,
                }),
            },
        )
        .expect("write invalid undo checkpoint metadata");

        let availability =
            undo_last_restore_availability(&app_handle).expect("query undo availability");
        assert!(!availability.available);
        assert!(
            !undo_checkpoint_meta_path(&app_data_dir).exists(),
            "invalid checkpoint metadata should be removed"
        );
        assert!(
            sentinel.exists(),
            "availability check should not touch snapshot directories outside runtime"
        );
    }

    #[test]
    fn undo_last_restore_rejects_invalid_checkpoint_without_data_change() {
        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        seed_history_db(&app_data_dir, &history_rows("undo-invalid-active"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("undo-invalid-active", "ACTIVE")],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "undo-invalid-active" }));
        seed_recordings(&app_data_dir, &[("undo-invalid-active.wav", b"A")]);
        let expected_state = snapshot_state(&app_data_dir);

        let runtime = runtime_dir(&app_data_dir);
        fs::create_dir_all(&runtime).expect("create runtime dir");
        let invalid_snapshot = runtime.join("snapshot-invalid");
        fs::create_dir_all(&invalid_snapshot).expect("create invalid snapshot dir");
        // Intentionally leave required managed files missing.

        durable_write_json(
            &undo_checkpoint_meta_path(&app_data_dir),
            &UndoCheckpointMeta {
                snapshot_path: invalid_snapshot.to_string_lossy().to_string(),
                created_at: now_rfc3339(),
                expires_at: (Utc::now() + Duration::days(1)).to_rfc3339(),
                snapshot_layout: Some(UndoCheckpointSnapshotLayout {
                    history_db: true,
                    user_dictionary: true,
                    user_store: true,
                    recordings_dir: false,
                }),
            },
        )
        .expect("write invalid undo checkpoint metadata");

        let availability =
            undo_last_restore_availability(&app_handle).expect("query undo availability");
        assert!(!availability.available);

        let report = undo_last_restore(&app_handle, UndoLastRestoreRequest::default())
            .expect("undo should fail closed for invalid checkpoint");
        assert!(!report.restored);
        assert!(
            report.message.contains("unavailable"),
            "expected unavailable-checkpoint messaging, got: {}",
            report.message
        );

        assert_eq!(
            snapshot_state(&app_data_dir),
            expected_state,
            "undo should not modify active data when checkpoint is invalid"
        );
        assert!(
            !undo_checkpoint_meta_path(&app_data_dir).exists(),
            "invalid checkpoint metadata should be removed"
        );
        assert!(
            !invalid_snapshot.exists(),
            "invalid checkpoint snapshot should be removed"
        );
    }

    #[test]
    fn undo_last_restore_reverts_and_consumes_checkpoint() {
        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        seed_history_db(&app_data_dir, &history_rows("undo-source"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("undo-source", "SOURCE")],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "undo-source" }));
        seed_recordings(
            &app_data_dir,
            &[("undo-source-1.wav", b"A"), ("undo-source-2.wav", b"B")],
        );
        let source_state = snapshot_state(&app_data_dir);

        let archive_path = make_backup(&app_handle, BackupScope::Complete, "undo-roundtrip");

        seed_history_db(&app_data_dir, &history_rows("undo-mutated"));
        seed_dictionary(
            &app_handle,
            vec![custom_word("undo-mutated", "MUTATED")],
        );
        seed_user_store(&app_data_dir, &json!({ "profile": "undo-mutated" }));
        seed_recordings(&app_data_dir, &[("undo-mutated.wav", b"Z")]);
        let pre_restore_state = snapshot_state(&app_data_dir);

        apply_restore(
            &app_handle,
            ApplyRestoreRequest {
                archive_path: archive_path.to_string_lossy().to_string(),
            },
        )
        .expect("apply restore prior to undo");
        assert_eq!(snapshot_state(&app_data_dir), source_state);

        let availability_before =
            undo_last_restore_availability(&app_handle).expect("query undo availability");
        assert!(availability_before.available);

        let undo_report = undo_last_restore(&app_handle, UndoLastRestoreRequest::default())
            .expect("undo last restore");
        assert!(undo_report.restored);
        assert_eq!(snapshot_state(&app_data_dir), pre_restore_state);
        assert!(
            list_runtime_dirs_with_prefix(&app_data_dir, "snapshot-").is_empty(),
            "undo should clean checkpoint and rollback snapshots"
        );

        let availability_after =
            undo_last_restore_availability(&app_handle).expect("query undo availability after use");
        assert!(!availability_after.available);
    }

