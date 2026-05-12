# Tauri + Vue + TypeScript

This template should help get you started developing with Vue 3 and TypeScript in Vite. The template uses Vue 3 `<script setup>` SFCs, check out the [script setup docs](https://v3.vuejs.org/api/sfc-script-setup.html#sfc-script-setup) to learn more.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Vue - Official](https://marketplace.visualstudio.com/items?itemName=Vue.volar) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## Companion Core And Windows REPL

The BLE backend, mock backend, and high-level companion protocol manager live in `src-tauri/crates/jukeboy-companion-core` so they can be reused outside the Tauri shell.

Build the Windows command-line REPL with:

```powershell
cd src-tauri
cargo build -p jukeboy-companion-repl --target x86_64-pc-windows-msvc
```

Run it on Windows with:

```powershell
cargo run -p jukeboy-companion-repl --target x86_64-pc-windows-msvc
```

Inside the prompt, use `help`, `scan`, `connect`, `status`, `snapshot`, playback commands such as `next` or `volume 60`, and `quit`.
