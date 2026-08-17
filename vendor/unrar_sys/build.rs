fn main() {
    // PATCH (vietzip, see LICENSES.md / CLAUDE.md's "unarc-rs" note): upstream used
    // `cfg!(windows)` / `#[cfg(windows)]` here, which reflect the HOST platform running
    // this build script, not the TARGET platform being built for — breaks cross-compiling
    // (e.g. Windows host -> Android target still "detects" Windows and tries to compile
    // isnt.cpp, which doesn't build off-Windows). Mirrors the same fix already applied to
    // vendor/unrar-ng-sys/build.rs. Read the real target from Cargo's CARGO_CFG_* env vars.
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    let is_windows_target = target_os == "windows";

    if is_windows_target {
        println!("cargo:rustc-flags=-lpowrprof");
        println!("cargo:rustc-link-lib=shell32");
        if target_env == "gnu" {
            println!("cargo:rustc-link-lib=pthread");
        }
    } else if target_os != "android" {
        // PATCH (vietzip): Android's Bionic libc has no separate libpthread — pthread
        // symbols have been part of libc itself since API 23. Linking `-lpthread`
        // unconditionally on "every non-Windows target" (the upstream assumption) fails
        // the link step on Android with "unable to find library -lpthread".
        println!("cargo:rustc-link-lib=pthread");
    }
    let mut file_stems: Vec<&str> = vec![
        "strlist",
        "strfn",
        "pathfn",
        "smallfn",
        "global",
        "file",
        "filefn",
        "filcreat",
        "archive",
        "arcread",
        "unicode",
        "system",
        "crypt",
        "crc",
        "rawread",
        "encname",
        "match",
        "timefn",
        "rdwrfn",
        "consio",
        "options",
        "errhnd",
        "rarvm",
        "secpassword",
        "rijndael",
        "getbits",
        "sha1",
        "sha256",
        "blake2s",
        "hash",
        "extinfo",
        "extract",
        "volume",
        "list",
        "find",
        "unpack",
        "headers",
        "threadpool",
        "rs16",
        "cmddata",
        "ui",
        "filestr",
        "scantree",
        "dll",
        "qopen",
    ];
    if is_windows_target {
        file_stems.push("isnt");
    }
    let files: Vec<String> = file_stems
        .iter()
        .map(|&s| format!("vendor/unrar/{s}.cpp"))
        .collect();
    // PATCH (vietzip): see the matching note in vendor/unrar-ng-sys/build.rs — first tried
    // statically linking `libc++_static.a` + `libc++abi.a` (fixed the missing-symbol dlopen
    // crash) but that caused a NEW `SIGSEGV`, root-caused via a symbolized tombstone +
    // disassembly to the NDK's *static* `libc.a`'s own copy of `getauxval.cpp` (a private,
    // never-initialized `__libc_shared_globals()`) getting linked in ahead of the real
    // dynamic `libc.so` — caused by this build script's own `cargo:rustc-link-search`
    // pointing at the exact directory `libc.a` also lives in. Switched to dynamic
    // `libc++_shared.so` (no longer needs that `-L` at all — rustc only eagerly verifies
    // *static* libs at compile time, and the external linker's own default NDK sysroot
    // search resolves `-lc++_shared` correctly on its own) — bundled into the APK ourselves
    // since Android has no system-provided copy for 3rd-party apps.
    if target_os == "android" {
        println!("cargo:rustc-link-lib=c++_shared");
        if let Some(dir) = android_libcxx_static_dir() {
            bundle_libcxx_shared_for_gradle(&dir);
        }
    }
    cc::Build::new()
        .cpp(true) // Switch to C++ library compilation.
        .opt_level(2)
        .std("c++14")
        // by default cc crate tries to link against dynamic stdlib, which causes problems on windows-gnu target
        .cpp_link_stdlib(None)
        .warnings(false)
        .extra_warnings(false)
        .flag_if_supported("-stdlib=libc++")
        .flag_if_supported("-fPIC")
        .flag_if_supported("-Wno-switch")
        .flag_if_supported("-Wno-parentheses")
        .flag_if_supported("-Wno-macro-redefined")
        .flag_if_supported("-Wno-dangling-else")
        .flag_if_supported("-Wno-logical-op-parentheses")
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-variable")
        .flag_if_supported("-Wno-unused-function")
        .flag_if_supported("-Wno-missing-braces")
        .flag_if_supported("-Wno-unknown-pragmas")
        .flag_if_supported("-Wno-deprecated-declarations")
        .define("_FILE_OFFSET_BITS", Some("64"))
        .define("_LARGEFILE_SOURCE", None)
        .define("RAR_SMP", None)
        .define("RARDLL", None)
        .files(&files)
        .compile("libunrar.a");
}

// PATCH (vietzip): see the matching function in vendor/unrar-ng-sys/build.rs — locates the
// Android NDK's per-ABI `usr/lib/<abi>` sysroot dir (where `libc++_static.a` lives) from
// the `CXX_<target-triple>` env var cargokit's Android build environment sets. Deliberately
// not gated by `#[cfg(target_os = "android")]`, which would reflect the HOST this build
// script itself compiles for, not the Android TARGET being cross-compiled to.
fn android_libcxx_static_dir() -> Option<std::path::PathBuf> {
    let target = std::env::var("TARGET").ok()?;
    let cxx = std::env::var(format!("CXX_{target}")).ok()?;
    let toolchain_root = std::path::Path::new(&cxx).parent()?.parent()?; // bin/.. -> <host-arch>/
    let abi_dir = match target.as_str() {
        "armv7-linux-androideabi" => "arm-linux-androideabi",
        other => other,
    };
    Some(
        toolchain_root
            .join("sysroot")
            .join("usr")
            .join("lib")
            .join(abi_dir),
    )
}

// PATCH (vietzip): see the matching function in vendor/unrar-ng-sys/build.rs — copies the
// NDK's `libc++_shared.so` into cargokit's per-build-type output dir so Gradle packages it
// into the final APK via the `jniLibs.srcDir` it already registers there.
fn bundle_libcxx_shared_for_gradle(ndk_lib_dir: &std::path::Path) {
    let Ok(target) = std::env::var("TARGET") else {
        return;
    };
    let Ok(output_dir) = std::env::var("CARGOKIT_OUTPUT_DIR") else {
        return;
    };
    let android_abi = match target.as_str() {
        "aarch64-linux-android" => "arm64-v8a",
        "armv7-linux-androideabi" => "armeabi-v7a",
        "x86_64-linux-android" => "x86_64",
        "i686-linux-android" => "x86",
        _ => return,
    };
    let src = ndk_lib_dir.join("libc++_shared.so");
    let dest_dir = std::path::Path::new(&output_dir).join(android_abi);
    let _ = std::fs::create_dir_all(&dest_dir);
    let _ = std::fs::copy(&src, dest_dir.join("libc++_shared.so"));
}
