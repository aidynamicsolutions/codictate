    #[test]
    fn write_permit_blocks_operation_start_until_write_completes() {
        let (_guard, _env, app, _app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        let (write_started_tx, write_started_rx) = mpsc::channel();
        let (release_write_tx, release_write_rx) = mpsc::channel();
        let writer_handle = app_handle.clone();
        let writer_thread = std::thread::spawn(move || {
            with_write_permit(&writer_handle, || {
                write_started_tx
                    .send(())
                    .expect("signal write permit acquisition");
                release_write_rx
                    .recv()
                    .expect("wait for write completion signal");
                Ok::<(), String>(())
            })
        });

        write_started_rx
            .recv_timeout(StdDuration::from_secs(1))
            .expect("writer should acquire gate");

        let (operation_started_tx, operation_started_rx) = mpsc::channel();
        let operation_handle = app_handle.clone();
        let operation_thread = std::thread::spawn(move || {
            let operation_guard = start_operation(&operation_handle)
                .expect("operation should start after write permit is released");
            operation_started_tx
                .send(())
                .expect("signal operation start");
            drop(operation_guard);
        });

        assert!(
            operation_started_rx
                .recv_timeout(StdDuration::from_millis(120))
                .is_err(),
            "operation should wait for in-flight write permit"
        );

        release_write_tx
            .send(())
            .expect("release write permit for operation");
        operation_started_rx
            .recv_timeout(StdDuration::from_secs(1))
            .expect("operation should start after write permit completes");

        let writer_result = writer_thread.join().expect("writer thread join");
        assert!(writer_result.is_ok(), "writer should complete successfully");
        operation_thread.join().expect("operation thread join");
    }

    #[test]
    fn queued_writer_from_precheck_window_is_rejected_after_operation_starts() {
        let (_guard, _env, app, _app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();
        let runtime = app_handle.state::<BackupRestoreRuntime>();

        let precheck_gate_guard = runtime
            .write_gate
            .lock()
            .expect("acquire gate to force queued writer");

        let (writer_started_tx, writer_started_rx) = mpsc::channel();
        let (writer_result_tx, writer_result_rx) = mpsc::channel();
        let writer_handle = app_handle.clone();
        let writer_thread = std::thread::spawn(move || {
            writer_started_tx
                .send(())
                .expect("signal writer start before permit attempt");
            let result = with_write_permit(&writer_handle, || Ok::<(), String>(()));
            writer_result_tx
                .send(result)
                .expect("send queued writer result");
        });

        writer_started_rx
            .recv_timeout(StdDuration::from_secs(1))
            .expect("writer thread should start");
        std::thread::sleep(StdDuration::from_millis(40));

        let (operation_started_tx, operation_started_rx) = mpsc::channel();
        let operation_handle = app_handle.clone();
        let operation_thread = std::thread::spawn(move || {
            let operation_guard =
                start_operation(&operation_handle).expect("operation should start after gate release");
            operation_started_tx
                .send(())
                .expect("signal operation start");
            drop(operation_guard);
        });

        let deadline = std::time::Instant::now() + StdDuration::from_secs(1);
        while !is_operation_in_progress(&app_handle) && std::time::Instant::now() < deadline {
            std::thread::sleep(StdDuration::from_millis(5));
        }
        assert!(
            is_operation_in_progress(&app_handle),
            "operation flag should become active while queued writer is waiting for gate"
        );

        drop(precheck_gate_guard);

        let queued_writer_result = writer_result_rx
            .recv_timeout(StdDuration::from_secs(1))
            .expect("queued writer should finish after gate release");
        assert_eq!(
            queued_writer_result.expect_err("queued writer must be blocked"),
            WRITES_BLOCKED_MESSAGE,
            "writer that entered during precheck window must fail once operation starts"
        );

        operation_started_rx
            .recv_timeout(StdDuration::from_secs(1))
            .expect("operation should proceed after queued writer is rejected");

        writer_thread.join().expect("writer thread join");
        operation_thread.join().expect("operation thread join");
    }

    #[test]
    fn in_flight_write_permit_work_survives_operation_flag_flip() {
        let (_guard, _env, app, app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();
        let marker_file = app_data_dir.join("in-flight-write-marker.txt");

        let (write_started_tx, write_started_rx) = mpsc::channel();
        let (release_write_tx, release_write_rx) = mpsc::channel();
        let writer_handle = app_handle.clone();
        let writer_marker = marker_file.clone();
        let writer_thread = std::thread::spawn(move || {
            with_write_permit(&writer_handle, || {
                write_started_tx
                    .send(())
                    .expect("signal in-flight write permit start");
                release_write_rx
                    .recv()
                    .expect("wait for operation flag to flip");

                // Guardrail regression: an extra nested assert_writes_allowed check
                // would fail here because operation_in_progress is already true.
                assert!(
                    assert_writes_allowed(&writer_handle).is_err(),
                    "operation flag should be visible while write permit section is in-flight"
                );

                fs::write(&writer_marker, b"written-under-permit-after-flag-flip")
                    .map_err(|error| format!("failed to write marker file: {error}"))?;
                Ok::<(), String>(())
            })
        });

        write_started_rx
            .recv_timeout(StdDuration::from_secs(1))
            .expect("in-flight write should acquire permit");

        let (operation_finished_tx, operation_finished_rx) = mpsc::channel();
        let operation_handle = app_handle.clone();
        let operation_thread = std::thread::spawn(move || {
            let operation_guard = start_operation(&operation_handle)
                .expect("operation should eventually start once write permit releases");
            operation_finished_tx
                .send(())
                .expect("signal operation completion");
            drop(operation_guard);
        });

        let deadline = std::time::Instant::now() + StdDuration::from_secs(1);
        while !is_operation_in_progress(&app_handle) && std::time::Instant::now() < deadline {
            std::thread::sleep(StdDuration::from_millis(10));
        }
        assert!(
            is_operation_in_progress(&app_handle),
            "operation flag should flip while write permit section is still in-flight"
        );

        release_write_tx
            .send(())
            .expect("release in-flight write section");

        let writer_result = writer_thread.join().expect("writer thread join");
        assert!(
            writer_result.is_ok(),
            "write permit section should complete without silent drop"
        );

        operation_finished_rx
            .recv_timeout(StdDuration::from_secs(1))
            .expect("operation should complete after write permit drains");
        operation_thread.join().expect("operation thread join");

        let marker_contents =
            fs::read_to_string(&marker_file).expect("read in-flight write marker file");
        assert_eq!(marker_contents, "written-under-permit-after-flag-flip");
    }

    #[test]
    fn operation_in_progress_rejects_new_write_permits_fast() {
        let (_guard, _env, app, _app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        let operation_guard = start_operation(&app_handle).expect("start backup/restore operation");
        let blocked_write = with_write_permit(&app_handle, || Ok::<(), String>(()));
        assert_eq!(
            blocked_write.unwrap_err(),
            WRITES_BLOCKED_MESSAGE,
            "write should fail fast while operation gate is active"
        );

        drop(operation_guard);

        with_write_permit(&app_handle, || Ok::<(), String>(()))
            .expect("write should succeed after operation completes");
    }

    #[test]
    fn operation_flag_transitions_do_not_deadlock_write_attempts() {
        let (_guard, _env, app, _app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        let operation_guard = start_operation(&app_handle).expect("start operation");
        assert!(is_operation_in_progress(&app_handle));
        assert!(is_maintenance_mode(&app_handle));

        let (write_attempt_tx, write_attempt_rx) = mpsc::channel();
        let write_handle = app_handle.clone();
        let write_thread = std::thread::spawn(move || {
            let result = with_write_permit(&write_handle, || Ok::<(), String>(()));
            write_attempt_tx
                .send(result)
                .expect("send write result");
        });

        let blocked_result = write_attempt_rx
            .recv_timeout(StdDuration::from_millis(200))
            .expect("write attempt should return quickly while operation is active");
        assert_eq!(
            blocked_result.unwrap_err(),
            WRITES_BLOCKED_MESSAGE,
            "write attempts should fail fast during maintenance mode"
        );

        write_thread.join().expect("write thread join");

        drop(operation_guard);

        assert!(!is_operation_in_progress(&app_handle));
        assert!(!is_maintenance_mode(&app_handle));
        with_write_permit(&app_handle, || Ok::<(), String>(()))
            .expect("write should proceed after maintenance mode ends");
    }

    #[test]
    fn transcription_start_gate_blocks_while_operation_is_active() {
        let (_guard, _env, app, _app_data_dir) = setup_test_app();
        let app_handle = app.handle().clone();

        assert!(
            can_start_transcription(&app_handle),
            "transcription should be allowed before maintenance starts"
        );

        let operation_guard = start_operation(&app_handle).expect("start operation");
        assert!(
            !can_start_transcription(&app_handle),
            "transcription should be blocked while maintenance mode is active"
        );

        drop(operation_guard);

        assert!(
            can_start_transcription(&app_handle),
            "transcription should be allowed again after maintenance mode ends"
        );
    }

    #[test]
    fn transcription_block_notice_is_throttled() {
        reset_transcription_block_notice_for_tests();

        let now = std::time::Instant::now();
        assert!(
            should_emit_transcription_block_notice_at(now),
            "first blocked-start notice should be emitted"
        );
        assert!(
            !should_emit_transcription_block_notice_at(now + StdDuration::from_secs(1)),
            "second blocked-start notice inside cooldown should be suppressed"
        );
        assert!(
            should_emit_transcription_block_notice_at(now + StdDuration::from_secs(6)),
            "blocked-start notice should emit again after cooldown"
        );

        reset_transcription_block_notice_for_tests();
    }
