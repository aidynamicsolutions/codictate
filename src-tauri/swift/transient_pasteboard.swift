// MARK: - Swift implementation for transient macOS pasteboard staging
// This file is compiled via Cargo build script for macOS targets.
// NOTE: All logging is handled by Rust wrappers - do not add logging here.

import AppKit
import Foundation
import ObjectiveC

private var providerRegistryAssociationKey: UInt8 = 0

private func activeProviderRegistry(for pasteboard: NSPasteboard) -> NSMutableSet {
    if let registry = objc_getAssociatedObject(
        pasteboard,
        &providerRegistryAssociationKey
    ) as? NSMutableSet {
        return registry
    }

    let registry = NSMutableSet()
    objc_setAssociatedObject(
        pasteboard,
        &providerRegistryAssociationKey,
        registry,
        .OBJC_ASSOCIATION_RETAIN_NONATOMIC
    )
    return registry
}

private func retainProvider(_ provider: TransientPasteboardProvider, on pasteboard: NSPasteboard) {
    activeProviderRegistry(for: pasteboard).add(provider)
}

private func releaseProvider(_ provider: TransientPasteboardProvider, from pasteboard: NSPasteboard) {
    guard let registry = objc_getAssociatedObject(
        pasteboard,
        &providerRegistryAssociationKey
    ) as? NSMutableSet else {
        return
    }

    registry.remove(provider)

    if registry.count == 0 {
        objc_setAssociatedObject(
            pasteboard,
            &providerRegistryAssociationKey,
            nil,
            .OBJC_ASSOCIATION_RETAIN_NONATOMIC
        )
    }
}

private final class TransientPasteboardProvider: NSObject, NSPasteboardItemDataProvider {
    enum FinalizeReason: Equatable {
        case providerFinished
        case timeout
    }

    let text: String
    let backupText: String?
    let timeoutMs: UInt64

    private let stateLock = NSLock()
    private var stagedChangeCount: Int = 0
    private var didProvideString = false
    private var isFinalized = false
    private var timeoutWorkItem: DispatchWorkItem?

    init(text: String, backupText: String?, timeoutMs: UInt64) {
        self.text = text
        self.backupText = backupText
        self.timeoutMs = timeoutMs
    }

    func arm(stagedChangeCount: Int) {
        stateLock.lock()
        self.stagedChangeCount = stagedChangeCount
        let workItem = DispatchWorkItem { [weak self] in
            self?.finalize(reason: .timeout)
        }
        timeoutWorkItem = workItem
        stateLock.unlock()

        DispatchQueue.main.asyncAfter(
            deadline: .now() + .milliseconds(Int(timeoutMs)),
            execute: workItem
        )
    }

    func pasteboard(
        _ pasteboard: NSPasteboard?,
        item: NSPasteboardItem,
        provideDataForType type: NSPasteboard.PasteboardType
    ) {
        guard type == .string else {
            return
        }

        _ = item.setString(text, forType: .string)

        stateLock.lock()
        didProvideString = true
        stateLock.unlock()
    }

    func pasteboardFinishedWithDataProvider(_ pasteboard: NSPasteboard) {
        if Thread.isMainThread {
            finalize(reason: .providerFinished)
            return
        }

        DispatchQueue.main.async { [weak self] in
            self?.finalize(reason: .providerFinished)
        }
    }

    private func finalize(reason: FinalizeReason) {
        let pasteboard = NSPasteboard.general

        stateLock.lock()
        if isFinalized {
            stateLock.unlock()
            return
        }
        isFinalized = true
        let didProvideString = self.didProvideString
        let stagedChangeCount = self.stagedChangeCount
        timeoutWorkItem?.cancel()
        timeoutWorkItem = nil
        stateLock.unlock()

        defer {
            releaseProvider(self, from: pasteboard)
        }

        let stillOwnsPasteboard = pasteboard.changeCount == stagedChangeCount

        if didProvideString {
            guard let backupText, !backupText.isEmpty, stillOwnsPasteboard else {
                return
            }

            let item = NSPasteboardItem()
            guard item.setString(backupText, forType: .string) else {
                return
            }

            pasteboard.clearContents()
            _ = pasteboard.writeObjects([item])
            return
        }

        guard reason == .timeout, stillOwnsPasteboard else {
            return
        }

        let item = NSPasteboardItem()
        guard item.setString(text, forType: .string) else {
            return
        }

        pasteboard.clearContents()
        _ = pasteboard.writeObjects([item])
    }
}

private func stageTransientPasteboardTextOnMain(
    _ textPointer: UnsafePointer<CChar>?,
    _ backupPointer: UnsafePointer<CChar>?,
    _ timeoutMs: UInt64
) -> Int32 {
    guard let textPointer else {
        return -1
    }

    let text = String(cString: textPointer)
    if text.isEmpty {
        return -2
    }

    let backupText = backupPointer.map { String(cString: $0) }
    let provider = TransientPasteboardProvider(
        text: text,
        backupText: backupText,
        timeoutMs: timeoutMs
    )
    let item = NSPasteboardItem()

    guard item.setDataProvider(provider, forTypes: [.string]) else {
        return -3
    }

    let pasteboard = NSPasteboard.general
    retainProvider(provider, on: pasteboard)
    pasteboard.clearContents()
    guard pasteboard.writeObjects([item]) else {
        releaseProvider(provider, from: pasteboard)
        return -4
    }

    provider.arm(stagedChangeCount: pasteboard.changeCount)
    return 0
}

/// Stage a transient general-pasteboard item whose string payload is supplied lazily.
/// Returns: 0 = staged, -1 = invalid text pointer, -2 = empty text,
///          -3 = failed to register provider, -4 = failed to write item
@_cdecl("stage_transient_pasteboard_text")
public func stageTransientPasteboardText(
    _ textPointer: UnsafePointer<CChar>?,
    _ backupPointer: UnsafePointer<CChar>?,
    _ timeoutMs: UInt64
) -> Int32 {
    if Thread.isMainThread {
        return stageTransientPasteboardTextOnMain(textPointer, backupPointer, timeoutMs)
    }

    var status: Int32 = -4
    DispatchQueue.main.sync {
        status = stageTransientPasteboardTextOnMain(textPointer, backupPointer, timeoutMs)
    }
    return status
}
