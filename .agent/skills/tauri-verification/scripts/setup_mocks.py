
import os
import re

MOCK_CONTENT = """
console.log("[Mocks] Initializing Tauri Mocks...");

// Ensure globals exist
window.__TAURI__ = window.__TAURI__ || {};
window.__TAURI_INTERNALS__ = window.__TAURI_INTERNALS__ || {};
window.__TAURI_IPC__ = window.__TAURI_IPC__ || ((...args) => console.log("[Mocks] IPC Call:", args));

// --- Mock Implementation ---

const mockInvoke = async (cmd, args) => {
    console.log(`[Mock Invoke] ${cmd}`, args);

    if (cmd === "get_app_settings") {
      return {
          app_language: "en",
          theme: "system",
          debug_mode: true,
          recording_cleanup_days: 30,
          recording_cleanup_enabled: false,
          model_path: "",
          silence_timeout_ms: 1000,
          vad_sensitivity: "high",
          audio_feedback: true,
      };
    }

    if (cmd === "get_transcription_history" || cmd === "get_history_entries") {
        return [
            {
                id: 1,
                file_name: "audio.wav",
                file_path: "/path/to/audio.wav",
                timestamp: Date.now(),
                duration_ms: 60000,
                transcription_text: "This is a mock transcription.",
                title: "Mock Recording",
                status: "success",
                saved: false,
                post_processed_text: null,
                post_process_prompt: null
            }
        ];
    }
    
    if (cmd === "get_user_profile_command") {
        return { 
            onboarding_completed: true,
            user_name: "Test User"
        };
    }
    
    if (cmd === "get_home_stats") {
        return {
            total_words: 15000,
            total_duration_minutes: 120,
            wpm: 125,
            time_saved_minutes: 120,
            streak_days: 2,
            faster_than_typing_percentage: 25
        };
    }
    
    if (cmd === "show_main_window") return null;
    if (cmd === "get_current_model") return "test-model";
    if (cmd === "check_apple_intelligence_available") return false;
    if (cmd === "get_microphone_mode") return true;
    if (cmd === "is_laptop") return true;
    
    if (cmd === "get_recommended_first_model") return "base";
    
    if (cmd === "get_model_info") {
        return {
            id: args?.modelId || "base",
            name: "Base Model",
            description: "Standard model",
            size_mb: 100,
            is_downloaded: true,
            speed_score: 0.9,
            needs_download: false,
            download_url: "",
            file_name: "base.bin"
        };
    }
    
    if (cmd === "download_model") return null;

    if (cmd.includes("plugin:macos-permissions")) return "granted";
    if (cmd === "plugin:os|locale") return "en-US";
    if (cmd === "plugin:os|platform") return "macos";
    
    if (cmd === "get_available_models") {
        return [{
            id: "base",
            name: "Base Model",
            description: "Standard model",
            size_mb: 100,
            is_downloaded: true,
            speed_score: 0.9,
            needs_download: false,
            download_url: "",
            file_name: "base.bin"
        }];
    }
    
    if (cmd === "is_model_loading") return false;
    if (cmd === "has_any_models_available") return true;
    
    if (cmd.startsWith("get_")) return null;

    return null;
};

// --- Apply Mocks ---

// 1. window.__TAURI__ (Legacy/Compat)
Object.assign(window.__TAURI__, {
    invoke: mockInvoke,
    transformCallback: (callback) => callback,
    promisified: (func) => func,
    convertFileSrc: (url) => url,
    event: {
        check: () => "mocked",
        listen: (event, handler) => {
             console.log("[Mock] Listen:", event);
             return Promise.resolve(() => console.log("[Mock] Unlisten:", event));
        },
        once: (event, handler) => {
             console.log("[Mock] Once:", event);
             return Promise.resolve(() => console.log("[Mock] Unlisten:", event));
        },
        emit: (event, payload) => console.log("[Mock] Emit:", event, payload)
    }
});

// 2. window.__TAURI_INTERNALS__ (V2)
window.__TAURI_INTERNALS__.invoke = mockInvoke;
window.__TAURI_INTERNALS__.convertFileSrc = (url) => url;
window.__TAURI_INTERNALS__.transformCallback = (callback) => callback;
window.__TAURI_INTERNALS__.promisified = (func) => func;
window.__TAURI_INTERNALS__.ipc = (message) => console.log("[Mock IPC]", message);

// Event internals
window.__TAURI_INTERNALS__.event = window.__TAURI_INTERNALS__.event || {};
Object.assign(window.__TAURI_INTERNALS__.event, {
    registerListener: (cb) => {
        console.log("[Mock] Registered listener", cb);
        return 123;
    },
    unregisterListener: (id) => {
        console.log("[Mock] Unregistered listener", id);
    }
});

// Plugin internals
window.__TAURI_INTERNALS__.plugins = window.__TAURI_INTERNALS__.plugins || {};
window.__TAURI_INTERNALS__.plugins.os = {
    locale: () => Promise.resolve("en-US"),
    platform: () => "macos",
};

// Metadata
window.__TAURI_INTERNALS__.metadata = {
    platform: "macos",
    arch: "arm64",
    os_version: "14.4.1",
    webview_version: "123.0.0.0",
    current_window: { label: "main" }
};

// 3. Specific OS Plugin Internal Mock
window.__TAURI_OS_PLUGIN_INTERNALS__ = {
    platform: "macos",
    eol: "\\n",
    version: "14.4.1",
    family: "unix",
    locale: "en-US",
};

console.log("[Mocks] Tauri Mocks Applied Successfully");
export {};
"""

def create_mock_file():
    mock_dir = "src/mocks"
    if not os.path.exists(mock_dir):
        os.makedirs(mock_dir)
    
    mock_path = os.path.join(mock_dir, "tauri.ts")
    with open(mock_path, "w") as f:
        f.write(MOCK_CONTENT)
    print(f"Created/Updated {mock_path}")

def inject_import():
    main_path = "src/main.tsx"
    if not os.path.exists(main_path):
        print(f"Error: {main_path} not found")
        return
    
    with open(main_path, "r") as f:
        content = f.read()
    
    import_statement = 'import "./mocks/tauri";'
    
    if import_statement not in content:
        print(f"Injecting import into {main_path}")
        # Insert at the top
        new_content = import_statement + "\n" + content
        with open(main_path, "w") as f:
            f.write(new_content)
    else:
        print(f"Import already present in {main_path}")


def cleanup():
    # Remove import from main.tsx
    main_path = "src/main.tsx"
    if os.path.exists(main_path):
        with open(main_path, "r") as f:
            lines = f.readlines()
        
        # Filter out the import line
        new_lines = [line for line in lines if 'import "./mocks/tauri";' not in line]
        
        # Check if the file starts with a newline that was added by us (heuristic)
        # If we added `import ...\n` to the top, removing it might leave the original content starting on line 1, 
        # or if there was no newline before, we are good.
        # But if we added a newline, we should probably check if we need to trim leading newlines if they weren't there?
        # A simpler approach is just ensuring the file doesn't start with unnecessary empty lines if we caused them.
        # However, precise restoration:
        # We injected `import_statement + "\n" + content`.
        # So effective removal is removing the line containing the import. 
        # The `\n` we added serves as the separator.
        # `lines` will have `import ...\n` as one element.
        # Removing that element removes the newline we added too.
        
        if len(new_lines) != len(lines):
            print(f"Removing import from {main_path}")
            with open(main_path, "w") as f:
                f.writelines(new_lines)
        else:
            print(f"Import not found in {main_path}")

    # Remove mock file
    mock_path = "src/mocks/tauri.ts"
    if os.path.exists(mock_path):
        try:
            os.remove(mock_path)
            print(f"Removed {mock_path}")
        except Exception as e:
            print(f"Error removing {mock_path}: {e}")
            
    # Remove mock directory if empty
    mock_dir = "src/mocks"
    if os.path.exists(mock_dir) and not os.listdir(mock_dir):
        try:
            os.rmdir(mock_dir)
            print(f"Removed empty directory {mock_dir}")
        except:
            pass

def main():
    import sys
    if len(sys.argv) > 1 and sys.argv[1] == "--cleanup":
        cleanup()
    else:
        create_mock_file()
        inject_import()

if __name__ == "__main__":
    main()
