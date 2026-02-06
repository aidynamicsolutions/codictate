// MARK: - Swift implementation for audio device transport type detection
// This file is compiled via Cargo build script for macOS targets

import AudioToolbox
import CoreAudio
import Foundation

/// Checks if an audio device with the given name is a Bluetooth device.
/// Returns 1 if Bluetooth, 0 if not, -1 if device not found.
@_cdecl("is_audio_device_bluetooth")
public func isAudioDeviceBluetooth(_ deviceName: UnsafePointer<CChar>) -> Int32 {
    let targetName = String(cString: deviceName)
    
    // Get array of all audio devices
    var propertySize: UInt32 = 0
    var propertyAddress = AudioObjectPropertyAddress(
        mSelector: kAudioHardwarePropertyDevices,
        mScope: kAudioObjectPropertyScopeGlobal,
        mElement: kAudioObjectPropertyElementMain
    )
    
    // Get size of device list
    var status = AudioObjectGetPropertyDataSize(
        AudioObjectID(kAudioObjectSystemObject),
        &propertyAddress,
        0,
        nil,
        &propertySize
    )
    
    if status != noErr {
        NSLog("[audio_device_info] Failed to get device list size: \(status)")
        return -1
    }
    
    let deviceCount = Int(propertySize) / MemoryLayout<AudioDeviceID>.size
    if deviceCount == 0 {
        NSLog("[audio_device_info] No audio devices found")
        return -1
    }
    
    // Get device IDs
    var deviceIDs = [AudioDeviceID](repeating: 0, count: deviceCount)
    status = AudioObjectGetPropertyData(
        AudioObjectID(kAudioObjectSystemObject),
        &propertyAddress,
        0,
        nil,
        &propertySize,
        &deviceIDs
    )
    
    if status != noErr {
        NSLog("[audio_device_info] Failed to get device list: \(status)")
        return -1
    }
    
    // Find the device with matching name
    for deviceID in deviceIDs {
        // Get device name
        var nameSize: UInt32 = UInt32(MemoryLayout<CFString>.size)
        var nameAddress = AudioObjectPropertyAddress(
            mSelector: kAudioDevicePropertyDeviceNameCFString,
            mScope: kAudioObjectPropertyScopeGlobal,
            mElement: kAudioObjectPropertyElementMain
        )
        
        var deviceNameRef: CFString? = nil
        status = AudioObjectGetPropertyData(
            deviceID,
            &nameAddress,
            0,
            nil,
            &nameSize,
            &deviceNameRef
        )
        
        guard status == noErr, let cfName = deviceNameRef else {
            continue
        }
        
        let currentName = cfName as String
        
        // Check if this is the device we're looking for
        if currentName == targetName {
            // Get transport type
            var transportType: UInt32 = 0
            var transportSize: UInt32 = UInt32(MemoryLayout<UInt32>.size)
            var transportAddress = AudioObjectPropertyAddress(
                mSelector: kAudioDevicePropertyTransportType,
                mScope: kAudioObjectPropertyScopeGlobal,
                mElement: kAudioObjectPropertyElementMain
            )
            
            status = AudioObjectGetPropertyData(
                deviceID,
                &transportAddress,
                0,
                nil,
                &transportSize,
                &transportType
            )
            
            if status != noErr {
                NSLog("[audio_device_info] Failed to get transport type for '\(currentName)': \(status)")
                return -1
            }
            
            // Check for Bluetooth transport types
            let isBluetooth = transportType == kAudioDeviceTransportTypeBluetooth
                           || transportType == kAudioDeviceTransportTypeBluetoothLE
            
            NSLog("[audio_device_info] Device '\(currentName)' transport type: 0x\(String(format: "%08X", transportType)), isBluetooth: \(isBluetooth)")
            
            return isBluetooth ? 1 : 0
        }
    }
    
    NSLog("[audio_device_info] Device '\(targetName)' not found in device list")
    return -1
}

/// Checks if an audio device with the given name is a built-in device.
/// Returns 1 if Built-in, 0 if not, -1 if device not found.
@_cdecl("is_audio_device_builtin")
public func isAudioDeviceBuiltin(_ deviceName: UnsafePointer<CChar>) -> Int32 {
    let targetName = String(cString: deviceName)
    
    // Get array of all audio devices
    var propertySize: UInt32 = 0
    var propertyAddress = AudioObjectPropertyAddress(
        mSelector: kAudioHardwarePropertyDevices,
        mScope: kAudioObjectPropertyScopeGlobal,
        mElement: kAudioObjectPropertyElementMain
    )
    
    // Get size of device list
    var status = AudioObjectGetPropertyDataSize(
        AudioObjectID(kAudioObjectSystemObject),
        &propertyAddress,
        0,
        nil,
        &propertySize
    )
    
    if status != noErr {
        NSLog("[audio_device_info] Failed to get device list size: \(status)")
        return -1
    }
    
    let deviceCount = Int(propertySize) / MemoryLayout<AudioDeviceID>.size
    if deviceCount == 0 {
        NSLog("[audio_device_info] No audio devices found")
        return -1
    }
    
    // Get device IDs
    var deviceIDs = [AudioDeviceID](repeating: 0, count: deviceCount)
    status = AudioObjectGetPropertyData(
        AudioObjectID(kAudioObjectSystemObject),
        &propertyAddress,
        0,
        nil,
        &propertySize,
        &deviceIDs
    )
    
    if status != noErr {
        NSLog("[audio_device_info] Failed to get device list: \(status)")
        return -1
    }
    
    // Find the device with matching name
    for deviceID in deviceIDs {
        // Get device name
        var nameSize: UInt32 = UInt32(MemoryLayout<CFString>.size)
        var nameAddress = AudioObjectPropertyAddress(
            mSelector: kAudioDevicePropertyDeviceNameCFString,
            mScope: kAudioObjectPropertyScopeGlobal,
            mElement: kAudioObjectPropertyElementMain
        )
        
        var deviceNameRef: CFString? = nil
        status = AudioObjectGetPropertyData(
            deviceID,
            &nameAddress,
            0,
            nil,
            &nameSize,
            &deviceNameRef
        )
        
        guard status == noErr, let cfName = deviceNameRef else {
            continue
        }
        
        let currentName = cfName as String
        
        // Check if this is the device we're looking for
        if currentName == targetName {
            // Get transport type
            var transportType: UInt32 = 0
            var transportSize: UInt32 = UInt32(MemoryLayout<UInt32>.size)
            var transportAddress = AudioObjectPropertyAddress(
                mSelector: kAudioDevicePropertyTransportType,
                mScope: kAudioObjectPropertyScopeGlobal,
                mElement: kAudioObjectPropertyElementMain
            )
            
            status = AudioObjectGetPropertyData(
                deviceID,
                &transportAddress,
                0,
                nil,
                &transportSize,
                &transportType
            )
            
            if status != noErr {
                NSLog("[audio_device_info] Failed to get transport type for '\(currentName)': \(status)")
                return -1
            }
            
            // Check for Built-in transport type
            let isBuiltin = transportType == kAudioDeviceTransportTypeBuiltIn
            
            NSLog("[audio_device_info] Device '\(currentName)' transport type: 0x\(String(format: "%08X", transportType)), isBuiltin: \(isBuiltin)")
            
            return isBuiltin ? 1 : 0
        }
    }
    
    NSLog("[audio_device_info] Device '\(targetName)' not found in device list")
    return -1
}

/// Checks if an audio device with the given name is a virtual device.
/// Returns 1 if Virtual, 0 if not, -1 if device not found.
@_cdecl("is_audio_device_virtual")
public func isAudioDeviceVirtual(_ deviceName: UnsafePointer<CChar>) -> Int32 {
    let targetName = String(cString: deviceName)
    
    // Get array of all audio devices
    var propertySize: UInt32 = 0
    var propertyAddress = AudioObjectPropertyAddress(
        mSelector: kAudioHardwarePropertyDevices,
        mScope: kAudioObjectPropertyScopeGlobal,
        mElement: kAudioObjectPropertyElementMain
    )
    
    // Get size of device list
    var status = AudioObjectGetPropertyDataSize(
        AudioObjectID(kAudioObjectSystemObject),
        &propertyAddress,
        0,
        nil,
        &propertySize
    )
    
    if status != noErr {
        return -1
    }
    
    let deviceCount = Int(propertySize) / MemoryLayout<AudioDeviceID>.size
    if deviceCount == 0 {
        return -1
    }
    
    // Get device IDs
    var deviceIDs = [AudioDeviceID](repeating: 0, count: deviceCount)
    status = AudioObjectGetPropertyData(
        AudioObjectID(kAudioObjectSystemObject),
        &propertyAddress,
        0,
        nil,
        &propertySize,
        &deviceIDs
    )
    
    if status != noErr {
        return -1
    }
    
    // Find the device with matching name
    for deviceID in deviceIDs {
        // Get device name
        var nameSize: UInt32 = UInt32(MemoryLayout<CFString>.size)
        var nameAddress = AudioObjectPropertyAddress(
            mSelector: kAudioDevicePropertyDeviceNameCFString,
            mScope: kAudioObjectPropertyScopeGlobal,
            mElement: kAudioObjectPropertyElementMain
        )
        
        var deviceNameRef: CFString? = nil
        status = AudioObjectGetPropertyData(
            deviceID,
            &nameAddress,
            0,
            nil,
            &nameSize,
            &deviceNameRef
        )
        
        guard status == noErr, let cfName = deviceNameRef else {
            continue
        }
        
        let currentName = cfName as String
        
        // Check if this is the device we're looking for
        if currentName == targetName {
            // Get transport type
            var transportType: UInt32 = 0
            var transportSize: UInt32 = UInt32(MemoryLayout<UInt32>.size)
            var transportAddress = AudioObjectPropertyAddress(
                mSelector: kAudioDevicePropertyTransportType,
                mScope: kAudioObjectPropertyScopeGlobal,
                mElement: kAudioObjectPropertyElementMain
            )
            
            status = AudioObjectGetPropertyData(
                deviceID,
                &transportAddress,
                0,
                nil,
                &transportSize,
                &transportType
            )
            
            if status != noErr {
                return -1
            }
            
            // Check for Virtual transport type
            // Also include Aggregate as they are often virtual-ish, but keeping it to Virtual for now as per user request for "phantom"
            // Teams/BlackHole are Virtual.
            let isVirtual = transportType == kAudioDeviceTransportTypeVirtual
            
            return isVirtual ? 1 : 0
        }
    }
    
    return -1
}

/// Returns the transport type of an audio device as a string (for debugging).
/// Returns nil if device not found.
@_cdecl("get_audio_device_transport_type")
public func getAudioDeviceTransportType(_ deviceName: UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar>? {
    let targetName = String(cString: deviceName)
    
    // Get array of all audio devices
    var propertySize: UInt32 = 0
    var propertyAddress = AudioObjectPropertyAddress(
        mSelector: kAudioHardwarePropertyDevices,
        mScope: kAudioObjectPropertyScopeGlobal,
        mElement: kAudioObjectPropertyElementMain
    )
    
    var status = AudioObjectGetPropertyDataSize(
        AudioObjectID(kAudioObjectSystemObject),
        &propertyAddress,
        0,
        nil,
        &propertySize
    )
    
    if status != noErr {
        return nil
    }
    
    let deviceCount = Int(propertySize) / MemoryLayout<AudioDeviceID>.size
    var deviceIDs = [AudioDeviceID](repeating: 0, count: deviceCount)
    
    status = AudioObjectGetPropertyData(
        AudioObjectID(kAudioObjectSystemObject),
        &propertyAddress,
        0,
        nil,
        &propertySize,
        &deviceIDs
    )
    
    if status != noErr {
        return nil
    }
    
    for deviceID in deviceIDs {
        var nameSize: UInt32 = UInt32(MemoryLayout<CFString>.size)
        var nameAddress = AudioObjectPropertyAddress(
            mSelector: kAudioDevicePropertyDeviceNameCFString,
            mScope: kAudioObjectPropertyScopeGlobal,
            mElement: kAudioObjectPropertyElementMain
        )
        
        var deviceNameRef: CFString? = nil
        status = AudioObjectGetPropertyData(
            deviceID,
            &nameAddress,
            0,
            nil,
            &nameSize,
            &deviceNameRef
        )
        
        guard status == noErr, let cfName = deviceNameRef, (cfName as String) == targetName else {
            continue
        }
        
        // Get transport type
        var transportType: UInt32 = 0
        var transportSize: UInt32 = UInt32(MemoryLayout<UInt32>.size)
        var transportAddress = AudioObjectPropertyAddress(
            mSelector: kAudioDevicePropertyTransportType,
            mScope: kAudioObjectPropertyScopeGlobal,
            mElement: kAudioObjectPropertyElementMain
        )
        
        status = AudioObjectGetPropertyData(
            deviceID,
            &transportAddress,
            0,
            nil,
            &transportSize,
            &transportType
        )
        
        if status != noErr {
            return nil
        }
        
        // Convert transport type to readable string
        let transportString: String
        switch transportType {
        case kAudioDeviceTransportTypeBuiltIn:
            transportString = "BuiltIn"
        case kAudioDeviceTransportTypeAggregate:
            transportString = "Aggregate"
        case kAudioDeviceTransportTypeVirtual:
            transportString = "Virtual"
        case kAudioDeviceTransportTypePCI:
            transportString = "PCI"
        case kAudioDeviceTransportTypeUSB:
            transportString = "USB"
        case kAudioDeviceTransportTypeFireWire:
            transportString = "FireWire"
        case kAudioDeviceTransportTypeBluetooth:
            transportString = "Bluetooth"
        case kAudioDeviceTransportTypeBluetoothLE:
            transportString = "BluetoothLE"
        case kAudioDeviceTransportTypeHDMI:
            transportString = "HDMI"
        case kAudioDeviceTransportTypeDisplayPort:
            transportString = "DisplayPort"
        case kAudioDeviceTransportTypeAirPlay:
            transportString = "AirPlay"
        case kAudioDeviceTransportTypeAVB:
            transportString = "AVB"
        case kAudioDeviceTransportTypeThunderbolt:
            transportString = "Thunderbolt"
        default:
            transportString = "Unknown(0x\(String(format: "%08X", transportType)))"
        }
        
        return strdup(transportString)
    }
    
    return nil
}

/// Frees a string returned by get_audio_device_transport_type
@_cdecl("free_transport_type_string")
public func freeTransportTypeString(_ ptr: UnsafeMutablePointer<CChar>?) {
    if let ptr = ptr {
        free(ptr)
    }
}
