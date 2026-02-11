## 1. Backend Implementation
- [ ] 1.1 Add atomic counter `FN_TEST_EVENT_COUNT` and test mode flag `FN_TEST_MODE_ACTIVE` in `fn_key_monitor.rs`
- [ ] 1.2 Add `start_fn_key_test` command to reset counter and enable test mode
- [ ] 1.3 Add `get_fn_key_test_result` command to return event count and stop test mode
- [ ] 1.4 Add `is_fn_key_monitor_active` command to check monitoring status
- [ ] 1.5 Increment counter in `handle_flags_changed_event` when test mode active
- [ ] 1.6 Register new commands in `lib.rs`

## 2. Frontend Implementation
- [ ] 2.1 Add "Test Fn Key" button in ShortcutSettings section
- [ ] 2.2 Implement 3-second test flow with visual feedback (testing/success/failed states)
- [ ] 2.3 Add troubleshooting section that appears on test failure
- [ ] 2.4 Show 4 resolution steps: System Settings, "Do Nothing" setting, close other apps, change shortcut

## 3. Translations
- [ ] 3.1 Add translation keys for test button, testing state, success/failure messages
- [ ] 3.2 Add translation keys for troubleshooting title and 4 steps

## 4. Optional: Passive Conflict Detection
- [ ] 4.1 Detect when Fn events stop being received after initial success
- [ ] 4.2 Emit event to frontend when conflict detected
- [ ] 4.3 Show notification directing user to Settings

## 5. Verification
- [ ] 5.1 Test normal operation (no conflict): Fn key works, test detects events
- [ ] 5.2 Test with conflicting app: Fn key blocked, test detects no events, troubleshooting shown
- [ ] 5.3 Verify troubleshooting steps are clear and actionable
