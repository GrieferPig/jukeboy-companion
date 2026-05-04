# Android APK Build

This project is a Tauri 2 + Vue + TypeScript app. Android packaging is driven from the repository root with Bun and the Tauri CLI. The Tauri build hook in `src-tauri/tauri.conf.json` runs `bun run build`, so every APK build also rebuilds the frontend before Gradle packages the Android app.

## Validated flow

- Host OS: Windows
- Shell used for the verified build: `bash`
- Package runner: `bun`
- Verified build: debug APK for `aarch64`

## Prerequisites

Install the following before building:

1. Bun
2. Rust and `rustup`
3. Android Studio or a complete Android SDK installation
4. Android SDK command-line tools
5. Android NDK
6. A JDK

The working setup for this repository used Android Studio's bundled JBR plus an installed Android SDK and NDK.

## Environment variables

Set these variables before running `tauri android` commands.

### Git Bash

```bash
export JAVA_HOME="/c/Program Files/Android/Android Studio/jbr"
export ANDROID_HOME="/c/Users/<your-user>/AppData/Local/Android/Sdk"
export NDK_HOME="$ANDROID_HOME/ndk/28.2.13676358"
```

### PowerShell

```powershell
$env:JAVA_HOME = "C:\Program Files\Android\Android Studio\jbr"
$env:ANDROID_HOME = "$env:LOCALAPPDATA\Android\Sdk"
$env:NDK_HOME = "$env:ANDROID_HOME\ndk\28.2.13676358"
```

Replace the NDK version with the exact version installed on your machine if it differs.

## Rust Android targets

Install the Android Rust targets once:

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
```

## One-time Android project initialization

From the repository root:

```bash
bun install
bun run tauri android init --ci
```

This generates the Android project under `src-tauri/gen/android`.

## Android BLE integration

This repository now uses a native Android BLE bridge instead of `btleplug` when targeting Android. The validated Android project includes these repository-specific pieces:

1. `src-tauri/src/lib.rs` initializes the Android BLE bridge during app startup and stores the JNI bridge state before the Tauri runtime starts.
2. `src-tauri/src/companion/android_ble.rs` owns the Rust side of the Android JNI bridge, including scan/connect/write/disconnect calls and notification callbacks.
3. `src-tauri/gen/android/app/src/main/java/com/grieferpig/jukeboy_companion/CompanionBleBridge.kt` owns the Android BLE scan, GATT session, write, disconnect, and notification logic.
4. `src-tauri/src/companion/client.rs` keeps the frame protocol/session logic in Rust and switches to the native bridge only on Android builds.
5. `src-tauri/gen/android/app/src/main/AndroidManifest.xml` and `src-tauri/gen/android/app/src/main/java/com/grieferpig/jukeboy_companion/MainActivity.kt` declare and request the Android BLE runtime permissions.

If you regenerate `src-tauri/gen/android`, verify the native bridge files are still present before testing BLE on Android.

## Build a debug APK

From the repository root:

```bash
bun run tauri android build --debug --apk -t aarch64
```

What this command does:

1. Runs the frontend build via `bun run build`
2. Uses the generated Android project in `src-tauri/gen/android`
3. Compiles the Rust mobile target and packages the APK

## Output

The verified APK output path is:

```text
src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk
```

## Troubleshooting

- `Android NDK not found`: install the NDK and point `NDK_HOME` at the exact installed NDK directory.
- `sdkmanager` or command-line tools missing: install Android SDK command-line tools under `ANDROID_HOME/cmdline-tools/latest`.
- Missing Rust target errors: rerun the `rustup target add ...` command above.
- Missing Android project files: rerun `bun run tauri android init --ci`.
- Android BLE bridge initialization errors: confirm `src-tauri/gen/android/app/src/main/java/com/grieferpig/jukeboy_companion/CompanionBleBridge.kt` still exists after any Android project regeneration.

## Release builds

The validated path in this repository is a debug APK build. A release APK or AAB will also need Android signing configured before it is ready for distribution.

## References

- Tauri start docs: `https://tauri.app/start/`
- Tauri prerequisites: `https://tauri.app/start/prerequisites/`
- Tauri CLI docs: `https://tauri.app/reference/cli/`
