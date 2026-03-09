// Bridge header for audio_device_info.swift
// This header is imported by the Swift compiler to expose C-callable functions

#ifndef audio_device_info_bridge_h
#define audio_device_info_bridge_h

#include <stdint.h>

/// Check if an audio device is Bluetooth based on its name.
/// Returns: 1 = Bluetooth, 0 = not Bluetooth, -1 = device not found or error
int32_t is_audio_device_bluetooth(const char *device_name);

/// Check if an audio device is Built-in.
/// Returns: 1 = Builtin, 0 = not Builtin, -1 = device not found or error
int32_t is_audio_device_builtin(const char *device_name);

/// Check if an audio device is Virtual (phantom).
/// Returns: 1 = Virtual, 0 = not Virtual, -1 = device not found or error
int32_t is_audio_device_virtual(const char *device_name);

/// Check if an audio device is a Continuity Camera (iPhone mic).
/// Returns: 1 = Continuity Camera, 0 = not, -1 = device not found or error
int32_t is_audio_device_continuity_camera(const char *device_name);

/// Get the transport type of an audio device as a string (for debugging).
/// Returns NULL if device not found. Caller must free with free_transport_type_string.
char *get_audio_device_transport_type(const char *device_name);

/// Free a string returned by get_audio_device_transport_type
void free_transport_type_string(char *ptr);

#endif /* audio_device_info_bridge_h */
