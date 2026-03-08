fn main() {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    build_apple_intelligence_bridge();

    #[cfg(target_os = "macos")]
    build_audio_device_info_bridge();
    #[cfg(target_os = "macos")]
    build_transient_pasteboard_bridge();

    generate_tray_translations();

    tauri_build::build()
}

/// Generate tray menu translations from frontend locale files.
///
/// Source of truth: src/i18n/locales/*/translation.json
/// The English "tray" section defines the struct fields.
fn generate_tray_translations() {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::Path;

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let locales_dir = Path::new("../src/i18n/locales");

    println!("cargo:rerun-if-changed=../src/i18n/locales");

    // Collect all locale translations
    let mut translations: BTreeMap<String, serde_json::Value> = BTreeMap::new();

    for entry in fs::read_dir(locales_dir).unwrap().flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let lang = path.file_name().unwrap().to_str().unwrap().to_string();
        let json_path = path.join("translation.json");

        println!("cargo:rerun-if-changed={}", json_path.display());

        let content = fs::read_to_string(&json_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();

        if let Some(tray) = parsed.get("tray").cloned() {
            translations.insert(lang, tray);
        }
    }

    // English defines the schema
    let english = translations.get("en").unwrap().as_object().unwrap();
    let fields: Vec<_> = english
        .keys()
        .map(|k| (camel_to_snake(k), k.clone()))
        .collect();

    // Generate code
    let mut out = String::from(
        "// Auto-generated from src/i18n/locales/*/translation.json - do not edit\n\n",
    );

    // Struct
    out.push_str("#[derive(Debug, Clone)]\npub struct TrayStrings {\n");
    for (rust_field, _) in &fields {
        out.push_str(&format!("    pub {rust_field}: String,\n"));
    }
    out.push_str("}\n\n");

    // Static map
    out.push_str(
        "pub static TRANSLATIONS: Lazy<HashMap<&'static str, TrayStrings>> = Lazy::new(|| {\n",
    );
    out.push_str("    let mut m = HashMap::new();\n");

    for (lang, tray) in &translations {
        out.push_str(&format!("    m.insert(\"{lang}\", TrayStrings {{\n"));
        for (rust_field, json_key) in &fields {
            let val = tray.get(json_key).and_then(|v| v.as_str()).unwrap_or("");
            out.push_str(&format!(
                "        {rust_field}: \"{}\".to_string(),\n",
                escape_string(val)
            ));
        }
        out.push_str("    });\n");
    }

    out.push_str("    m\n});\n");

    fs::write(Path::new(&out_dir).join("tray_translations.rs"), out).unwrap();

    println!(
        "cargo:warning=Generated tray translations: {} languages, {} fields",
        translations.len(),
        fields.len()
    );
}

fn camel_to_snake(s: &str) -> String {
    s.chars()
        .enumerate()
        .fold(String::new(), |mut acc, (i, c)| {
            if c.is_uppercase() && i > 0 {
                acc.push('_');
            }
            acc.push(c.to_lowercase().next().unwrap());
            acc
        })
}

fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(target_os = "macos")]
fn command_stdout_or_panic(cmd: &str, args: &[&str], context: &str) -> String {
    use std::process::Command;

    let output = Command::new(cmd)
        .args(args)
        .output()
        .unwrap_or_else(|error| {
            panic!(
                "Failed while {} ({} {}): {}",
                context,
                cmd,
                args.join(" "),
                error
            )
        });

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let status = output
            .status
            .code()
            .map(|code| code.to_string())
            .unwrap_or_else(|| "signal".to_string());

        if stderr.contains("You have not agreed to the Xcode license agreements") {
            panic!(
                "{} failed (exit {}).\n{}\nAction required: run `sudo xcodebuild -license` and accept the license, then retry.",
                context,
                status,
                stderr
            );
        }

        panic!(
            "{} failed (exit {}) while running `{}`.\nstderr: {}",
            context,
            status,
            format!("{} {}", cmd, args.join(" ")),
            stderr
        );
    }

    String::from_utf8(output.stdout)
        .unwrap_or_else(|error| panic!("{} returned invalid UTF-8: {}", context, error))
        .trim()
        .to_string()
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn build_apple_intelligence_bridge() {
    use std::env;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    const REAL_SWIFT_FILE: &str = "swift/apple_intelligence.swift";
    const STUB_SWIFT_FILE: &str = "swift/apple_intelligence_stub.swift";
    const BRIDGE_HEADER: &str = "swift/apple_intelligence_bridge.h";

    println!("cargo:rerun-if-changed={REAL_SWIFT_FILE}");
    println!("cargo:rerun-if-changed={STUB_SWIFT_FILE}");
    println!("cargo:rerun-if-changed={BRIDGE_HEADER}");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let object_path = out_dir.join("apple_intelligence.o");
    let static_lib_path = out_dir.join("libapple_intelligence.a");

    let sdk_path = command_stdout_or_panic(
        "xcrun",
        &["--sdk", "macosx", "--show-sdk-path"],
        "Locating macOS SDK for Apple Intelligence bridge",
    );

    // Check if the SDK supports FoundationModels (required for Apple Intelligence)
    let framework_path =
        Path::new(&sdk_path).join("System/Library/Frameworks/FoundationModels.framework");
    let has_foundation_models = framework_path.exists();

    let source_file = if has_foundation_models {
        println!("cargo:warning=Building with Apple Intelligence support.");
        REAL_SWIFT_FILE
    } else {
        println!("cargo:warning=Apple Intelligence SDK not found. Building with stubs.");
        STUB_SWIFT_FILE
    };

    if !Path::new(source_file).exists() {
        panic!("Source file {} is missing!", source_file);
    }

    let swiftc_path = command_stdout_or_panic(
        "xcrun",
        &["--find", "swiftc"],
        "Locating swiftc for Apple Intelligence bridge",
    );

    let toolchain_swift_lib = Path::new(&swiftc_path)
        .parent()
        .and_then(|p| p.parent())
        .map(|root| root.join("lib/swift/macosx"))
        .expect("Unable to determine Swift toolchain lib directory");
    let sdk_swift_lib = Path::new(&sdk_path).join("usr/lib/swift");

    // Use macOS 11.0 as deployment target for compatibility
    // The @available(macOS 26.0, *) checks in Swift handle runtime availability
    // Weak linking for FoundationModels is handled via cargo:rustc-link-arg below
    let status = Command::new("xcrun")
        .args([
            "swiftc",
            "-target",
            "arm64-apple-macosx11.0",
            "-sdk",
            &sdk_path,
            "-O",
            "-import-objc-header",
            BRIDGE_HEADER,
            "-c",
            source_file,
            "-o",
            object_path
                .to_str()
                .expect("Failed to convert object path to string"),
        ])
        .status()
        .expect("Failed to invoke swiftc for Apple Intelligence bridge");

    if !status.success() {
        panic!("swiftc failed to compile {source_file}");
    }

    let status = Command::new("libtool")
        .args([
            "-static",
            "-o",
            static_lib_path
                .to_str()
                .expect("Failed to convert static lib path to string"),
            object_path
                .to_str()
                .expect("Failed to convert object path to string"),
        ])
        .status()
        .expect("Failed to create static library for Apple Intelligence bridge");

    if !status.success() {
        panic!("libtool failed for Apple Intelligence bridge");
    }

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=apple_intelligence");
    println!(
        "cargo:rustc-link-search=native={}",
        toolchain_swift_lib.display()
    );
    println!("cargo:rustc-link-search=native={}", sdk_swift_lib.display());
    println!("cargo:rustc-link-lib=framework=Foundation");

    if has_foundation_models {
        // Use weak linking so the app can launch on systems without FoundationModels
        println!("cargo:rustc-link-arg=-weak_framework");
        println!("cargo:rustc-link-arg=FoundationModels");
    }

    println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
}

#[cfg(target_os = "macos")]
fn build_audio_device_info_bridge() {
    use std::env;
    use std::path::PathBuf;
    use std::process::Command;

    const SWIFT_FILE: &str = "swift/audio_device_info.swift";
    const BRIDGE_HEADER: &str = "swift/audio_device_info_bridge.h";

    println!("cargo:rerun-if-changed={SWIFT_FILE}");
    println!("cargo:rerun-if-changed={BRIDGE_HEADER}");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let object_path = out_dir.join("audio_device_info.o");
    let static_lib_path = out_dir.join("libaudio_device_info.a");

    let sdk_path = command_stdout_or_panic(
        "xcrun",
        &["--sdk", "macosx", "--show-sdk-path"],
        "Locating macOS SDK for audio_device_info bridge",
    );

    // Detect target architecture from Cargo environment
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_else(|_| "aarch64".to_string());
    
    // Map Rust arch to Swift arch
    let swift_arch = match target_arch.as_str() {
        "x86_64" => "x86_64",
        "aarch64" => "arm64",
        _ => "arm64", // Default to arm64 if unknown (likely Apple Silicon)
    };
    
    let swift_target = format!("{}-apple-macosx11.0", swift_arch);

    // Compile Swift file
    let status = Command::new("xcrun")
        .args([
            "swiftc",
            "-target",
            &swift_target,
            "-sdk",
            &sdk_path,
            "-O",
            "-import-objc-header",
            BRIDGE_HEADER,
            "-c",
            SWIFT_FILE,
            "-o",
            object_path
                .to_str()
                .expect("Failed to convert object path to string"),
        ])
        .status()
        .expect("Failed to invoke swiftc for audio_device_info bridge");

    if !status.success() {
        panic!("swiftc failed to compile {SWIFT_FILE}");
    }

    // Create static library
    let status = Command::new("libtool")
        .args([
            "-static",
            "-o",
            static_lib_path
                .to_str()
                .expect("Failed to convert static lib path to string"),
            object_path
                .to_str()
                .expect("Failed to convert object path to string"),
        ])
        .status()
        .expect("Failed to create static library for audio_device_info bridge");

    if !status.success() {
        panic!("libtool failed for audio_device_info bridge");
    }

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=audio_device_info");
    println!("cargo:rustc-link-lib=framework=CoreAudio");
    println!("cargo:rustc-link-lib=framework=AudioToolbox");

    println!("cargo:warning=Built audio_device_info bridge for Bluetooth detection");
}

#[cfg(target_os = "macos")]
fn build_transient_pasteboard_bridge() {
    use std::env;
    use std::path::PathBuf;
    use std::process::Command;

    const SWIFT_FILE: &str = "swift/transient_pasteboard.swift";
    const BRIDGE_HEADER: &str = "swift/transient_pasteboard_bridge.h";

    println!("cargo:rerun-if-changed={SWIFT_FILE}");
    println!("cargo:rerun-if-changed={BRIDGE_HEADER}");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let object_path = out_dir.join("transient_pasteboard.o");
    let static_lib_path = out_dir.join("libtransient_pasteboard.a");

    let sdk_path = command_stdout_or_panic(
        "xcrun",
        &["--sdk", "macosx", "--show-sdk-path"],
        "Locating macOS SDK for transient_pasteboard bridge",
    );

    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_else(|_| "aarch64".to_string());
    let swift_arch = match target_arch.as_str() {
        "x86_64" => "x86_64",
        "aarch64" => "arm64",
        _ => "arm64",
    };
    let swift_target = format!("{swift_arch}-apple-macosx11.0");

    let status = Command::new("xcrun")
        .args([
            "swiftc",
            "-target",
            &swift_target,
            "-sdk",
            &sdk_path,
            "-O",
            "-import-objc-header",
            BRIDGE_HEADER,
            "-c",
            SWIFT_FILE,
            "-o",
            object_path
                .to_str()
                .expect("Failed to convert object path to string"),
        ])
        .status()
        .expect("Failed to invoke swiftc for transient_pasteboard bridge");

    if !status.success() {
        panic!("swiftc failed to compile {SWIFT_FILE}");
    }

    let status = Command::new("libtool")
        .args([
            "-static",
            "-o",
            static_lib_path
                .to_str()
                .expect("Failed to convert static lib path to string"),
            object_path
                .to_str()
                .expect("Failed to convert object path to string"),
        ])
        .status()
        .expect("Failed to create static library for transient_pasteboard bridge");

    if !status.success() {
        panic!("libtool failed for transient_pasteboard bridge");
    }

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=transient_pasteboard");
    println!("cargo:rustc-link-lib=framework=AppKit");
    println!("cargo:rustc-link-lib=framework=Foundation");

    println!("cargo:warning=Built transient_pasteboard bridge for macOS clipboard staging");
}
