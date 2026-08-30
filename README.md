<p align="center">
  <img src="logo.png" alt="musicport logo" width="160" />
</p>

<h1 align="center">musicport</h1>

<p align="center">
  Move music onto your iPhone <strong>locally</strong> - no iCloud, no sync wipe, no per-app<br>
  subscription fees.
</p>

<p align="center">
  <a href="https://github.com/ChloeVPin/musicport"><img src="https://img.shields.io/github/stars/ChloeVPin/musicport?style=flat&logo=github&logoColor=white&label=Stars" alt="GitHub stars" /></a>
  <a href="https://tauri.app"><img src="https://img.shields.io/badge/Tauri-2.0-24c8db?style=flat&logo=tauri&logoColor=white" alt="Tauri 2" /></a>
  <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/Rust-2024-orange?style=flat&logo=rust&logoColor=white" alt="Rust" /></a>
  <img src="https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey?style=flat" alt="Platform" />
  <img src="https://img.shields.io/badge/license-Proprietary-red?style=flat" alt="License" />
</p>

<p align="center">
  <strong>Plug in. Drop files. Done.</strong><br>
  The simple way to get your music on iPhone - without iTunes, without the cloud.
</p>

---

### Why musicport?

iTunes wants to sync your whole library. Cloud services want a subscription. musicport just moves files over USB.

- **Local only** - direct USB transfer, nothing uploaded
- **No sync wipe** - your existing music stays untouched
- **No subscription** - one app, your files, forever
- **Safe by default** - automatic backup before every write to `~/.musicport/backups/`
- **Fast** - native Rust core, instant catalog updates, no reboot needed

### How it works

No jailbreak. iOS exposes the media partition over USB via AFC.

1. Plug in over USB, unlock, tap **Trust**
2. musicport reads your Music library (`MediaLibrary.sqlitedb`)
3. Clones an existing song entry and rewrites it for your new files - schema-safe, survives iOS updates
4. Uploads audio to `iTunes_Control/Music/`
5. Pushes the catalog back - the Music app picks it up instantly

### Quick start

Requires Rust and `libimobiledevice`:

```bash
brew install libimobiledevice # macOS

npm install
npm run tauri:dev   # run the app - one command, starts Vite + Tauri
```

Other commands:

```bash
npm run tauri:build                 # production bundle (.app/.dmg / AppImage)
cargo test -p musicport-core        # unit + snapshot tests
```

Plug in an iPhone over USB, unlock it, tap **Trust** if prompted, and add files.

### Project structure

```
package.json           npm workspace root - single-command tauri workflows
crates/
├── core/        musicport-core - everything that talks to the phone
│   └── src/
│       ├── ffi.rs      hand-written libimobiledevice bindings (no bindgen)
│       ├── device.rs   device discovery, lockdown, AFC
│       ├── db/         catalog read/write (the moat)
│       │   ├── schema.rs  handle, schema detection, row primitives
│       │   ├── query.rs   star-schema joins as TrackRows
│       │   ├── clone.rs   deep-clone machinery (survives schema drift)
│       │   ├── write.rs   add / remove tracks
│       │   ├── paths.rs   obfuscated folder-bucket helpers
│       │   └── tags.rs    audio metadata via lofty
│       └── services/   operations a shell calls
│           ├── phone.rs    connection + catalog plumbing
│           ├── library.rs  list / add / remove / export flows
│           ├── reports.rs  operation results (Serializable)
│           └── naming.rs   readable export filenames
└── desktop/      musicport-desktop - the Tauri 2 shell
    ├── src/           main.rs + lib.rs bootstrap, commands.rs (invoke API)
    ├── tauri.conf.json, capabilities/, build.rs
ui/                frontend - vanilla TS + Vite (npm workspace)
    ├── index.html, vite.config.ts, tsconfig.json
    └── src/          main.ts, api.ts (typed invoke), types.ts, style.css
```

<details>
<summary>For developers</summary>

- `npm run tauri:dev` runs `vite` with HMR and launches the Rust shell - the two halves never need to be started separately. For Rust-only iteration: `cargo run -p musicport-desktop` (keep `npm run dev` running).
- Reference implementation (`~/musicctl`, Python + pymobiledevice3) is GPL-3.0 and remains as the spec and test harness. This workspace is the zero-Python successor.

</details>

### Licensing

**Proprietary** (closed source). The only copyleft dependency is `libimobiledevice` (LGPL-2.1), which allows closed-source linking. No GPL code is included, so no GPL obligations apply. The reference `musicctl` implementation is GPL-3.0 and is not part of this product.

---

<p align="center">
  If you find this useful, give it a star!
</p>
