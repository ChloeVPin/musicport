# musicport

musicport is a Tauri desktop application for moving local music files to an iPhone over USB. The Rust core handles device communication and library operations; the UI is a Vite application.

The device workflow depends on iOS behavior and the installed `libimobiledevice` tools. Test transfers with a device you can restore and verify before relying on the result.

## Run locally

Install the native prerequisite for your platform, then install the JavaScript dependencies:

```sh
brew install libimobiledevice
npm install
npm run tauri:dev
```

The Homebrew command applies to macOS. Linux and Windows require the equivalent `libimobiledevice` packages for those platforms.

## Build and test

```sh
npm run build
npm run tauri:build
cargo test -p musicport-core
```

The generated bundle and device behavior can vary with the operating system, iOS version, and phone model. The application does not replace a backup of the device library.
