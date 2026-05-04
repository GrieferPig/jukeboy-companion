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

This repository uses `btleplug` on Android, which is a hybrid Rust/Java integration. The validated Android project includes these repository-specific pieces:

1. `src-tauri/src/lib.rs` initializes `btleplug` during Android startup and records any init failure instead of panicking in the JNI entrypoint.
2. `src-tauri/gen/android/settings.gradle` includes the btleplug Android Java library from the local Cargo registry as the `:btleplug-android` Gradle subproject.
3. `src-tauri/gen/android/app/build.gradle.kts` depends on `project(":btleplug-android")` so the Java classes are packaged into the APK.
4. `src-tauri/gen/android/app/src/main/AndroidManifest.xml` includes location permissions and BLE feature declarations, while the btleplug library manifest contributes the Bluetooth permissions it needs.
5. `src-tauri/gen/android/app/proguard-rules.pro` keeps btleplug JNI-loaded Java classes from being stripped in release builds.

If you regenerate `src-tauri/gen/android`, verify these files still contain the btleplug integration before testing BLE on Android.

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
- `failed to initialize btleplug on Android: Other(JavaException)`: confirm the generated Android project still includes `:btleplug-android` in `src-tauri/gen/android/settings.gradle` and `implementation(project(":btleplug-android"))` in `src-tauri/gen/android/app/build.gradle.kts`.

## Release builds

The validated path in this repository is a debug APK build. A release APK or AAB will also need Android signing configured before it is ready for distribution.

## References

- Tauri start docs: `https://tauri.app/start/`
- Tauri prerequisites: `https://tauri.app/start/prerequisites/`
- Tauri CLI docs: `https://tauri.app/reference/cli/`
