    #[test]
    fn normalize_archive_path_rejects_windows_drive_prefix() {
        let result = normalize_archive_path("C:/Users/alice/file.txt");
        assert!(
            result.is_err(),
            "windows drive-prefixed path should be rejected"
        );
    }

    #[test]
    fn normalize_archive_path_rejects_control_characters() {
        let result = normalize_archive_path("recordings/bad\nfile.wav");
        assert!(
            result.is_err(),
            "control characters in archive paths should be rejected"
        );
    }

    #[test]
    fn normalize_archive_path_accepts_regular_unicode_paths() {
        let normalized =
            normalize_archive_path("recordings/naive-resume-こんにちは.wav").expect("normalize");
        assert_eq!(normalized, "recordings/naive-resume-こんにちは.wav");
    }

    #[test]
    fn parse_checksums_accepts_regular_paths() {
        let checksums = parse_checksums(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef  history/history.jsonl\n\
             abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789  dictionary/dictionary.json\n",
        )
        .expect("parse checksums");

        assert_eq!(
            checksums.get(HISTORY_FILE),
            Some(&"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string())
        );
        assert_eq!(
            checksums.get(DICTIONARY_FILE),
            Some(&"abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789".to_string())
        );
    }

