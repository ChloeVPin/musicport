// link against libimobiledevice - check if lib exists before linking
// handles .so version quirks and .local-lib symlinks so partial installs still build
fn main() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let local = std::path::Path::new(&manifest).join(".local-lib");
    if local.exists() {
        println!("cargo:rustc-link-search=native={}", local.display());
    }

    craft_lib("imobiledevice-1.0", &local);
    craft_lib("imobiledevice-glue-1.0", &local);
    craft_lib("plist-2.0", &local);
    craft_lib("usbmuxd-2.0", &local);
    craft_lib("tatsu", &local);

    println!("cargo:rerun-if-changed=build.rs");
}

/// only emit -l if we can find the lib, skip missing so partial builds still work
fn craft_lib(lib: &str, local: &std::path::Path) {
    let has = |dir: &std::path::Path| {
        for ext in ["so", "dylib", "a"] {
            if dir.join(format!("lib{lib}.{ext}")).exists()
                || std::fs::read_dir(dir)
                    .is_ok_and(|mut it| {
                        it.any(|e| {
                            e.map(|e| e.file_name().to_string_lossy().starts_with(&format!("lib{lib}.")))
                                .unwrap_or(false)
                        })
                    })
            {
                return true;
            }
        }
        false
    };

    if (local.exists() && has(local))
        || has(&std::path::Path::new("/usr/lib/x86_64-linux-gnu"))
        || has(&std::path::Path::new("/usr/lib"))
        || has(&std::path::Path::new("/opt/homebrew/lib"))
    {
        println!("cargo:rustc-link-lib={lib}");
    }
}