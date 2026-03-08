// Bridge header for transient_pasteboard.swift
// This header is imported by the Swift compiler to expose C-callable functions.

#ifndef transient_pasteboard_bridge_h
#define transient_pasteboard_bridge_h

#include <stdint.h>

/// Stage a transient general-pasteboard item whose string data is provided lazily.
/// Returns: 0 = staged, negative values indicate validation or AppKit failures.
int32_t stage_transient_pasteboard_text(const char *text, const char *backup_text, uint64_t timeout_ms);

#endif /* transient_pasteboard_bridge_h */
