fn main() {
    // Watch the whole vendored UnRAR tree so additions/removals — and edits
    // to files outside the `#include` graph that `cc` already tracks via
    // its own per-source `rerun-if-changed` emissions — trigger a rebuild.
    // (Cargo automatically tracks `build.rs` itself, so listing it here is
    // unnecessary.) Without this directive, an upstream upgrade that adds
    // a new `.cpp` we forget to register, or a `.hpp` not yet picked up by
    // any compiled translation unit, would silently use stale object files.
    println!("cargo:rerun-if-changed=vendor/unrar");

    // PATCH (vietzip, see LICENSES.md): upstream used `cfg!(windows)` /
    // `#[cfg(windows)]` here, which reflect the HOST platform running this
    // build script (rustc/cargo itself), not the TARGET platform being
    // built for. That's correct for a native build but breaks cross-compilation
    // (e.g. building on Windows for Android/Linux still "detects" Windows and
    // tries to compile Windows-only files like isnt.cpp/motw.cpp, which
    // reference DWORD/OSVERSIONINFO/MarkOfTheWeb that don't exist off-Windows).
    // Read the actual target from Cargo's CARGO_CFG_* env vars instead, the
    // same mechanism this file already correctly uses below for `target_os`.
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    let is_windows_target = target_os == "windows";

    if is_windows_target {
        println!("cargo:rustc-flags=-lpowrprof");
        println!("cargo:rustc-link-lib=shell32");
        println!("cargo:rustc-link-lib=advapi32");
        if target_env == "gnu" {
            println!("cargo:rustc-link-lib=pthread");
        }
    } else if target_os != "android" {
        // PATCH (vietzip): Android's Bionic libc has no separate libpthread —
        // pthread symbols have been part of libc itself since API 23. Linking
        // `-lpthread` unconditionally on "every non-Windows target" (the
        // upstream assumption) fails the link step on Android with
        // "unable to find library -lpthread". Every other Unix-like target
        // still gets it as before.
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
        "largepage", // New in unrar 7.x for large page memory allocation
    ];
    if is_windows_target {
        file_stems.push("isnt");
        file_stems.push("motw"); // New in unrar 7.x for Mark of the Web support (Windows only)
    }
    let files: Vec<String> = file_stems
        .iter()
        .map(|&s| format!("vendor/unrar/{s}.cpp"))
        .collect();
    // PATCH (vietzip): `cpp_link_stdlib(None)` below (needed to avoid a windows-gnu linking
    // issue) means NO C++ runtime gets linked on ANY target unless done explicitly here.
    // Windows/Linux/macOS happen to have the needed RTTI/exception symbols (e.g.
    // `_ZTISt12length_error`, typeinfo for std::length_error) satisfied some other way at
    // link/load time, but Android has no system libc++ available to third-party apps —
    // confirmed by a real `dlopen failed: cannot locate symbol "_ZTISt12length_error"`
    // crash on a physical device.
    //
    // First attempt statically linked the NDK's `libc++_static.a` + `libc++abi.a`, which
    // fixed that crash but introduced a NEW one, on real-device runtime testing: a
    // `SIGSEGV` null-pointer dereference right after `dlopen`, inside `getauxval()` calling
    // `__libc_shared_globals()`. Root-caused via a symbolized native tombstone + disassembly
    // (not guessed): the crashing code is the NDK's *static* `libc.a`'s own copy of
    // `bionic/libc/bionic/getauxval.cpp` — which carries its own private, NEVER-initialized
    // `__libc_shared_globals()` (a function-local static, disconnected from the real one the
    // actual running process already initialized via the dynamic `libc.so`) — not anything
    // from libc++_static/libc++abi at all. It got pulled in because THIS build script was
    // adding an explicit `cargo:rustc-link-search=native=<ndk>/usr/lib/<abi>/` — the exact
    // directory `libc.a` also lives in — which apparently let the linker resolve the
    // implicit `-lc` reference against the static archive instead of the intended dynamic
    // `libc.so`. Switched to the NDK-recommended dynamic `libc++_shared.so` for the C++
    // runtime itself (avoids needing `-lc++_static`/`-lc++abi` at all, so rustc's eager
    // static-lib compile-time existence check — the ONLY reason this extra `-L` was ever
    // added — no longer applies) and dropped the `-L` entirely; the external linker's own
    // default NDK sysroot search already resolves `-lc++_shared` correctly without it,
    // exactly the same way it already resolves `-lc`/`-lm`/`-llog` without any extra `-L`.
    // Since Android has no system-provided `libc++_shared.so` for 3rd-party apps, it must
    // still be bundled into the APK ourselves (see `bundle_libcxx_shared_for_gradle()`).
    if target_os == "android" {
        println!("cargo:rustc-link-lib=c++_shared");
        // Still need the NDK's per-ABI sysroot lib dir to locate `libc++_shared.so` to copy
        // (see `bundle_libcxx_shared_for_gradle` below) — just not as a linker `-L` flag.
        if let Some(dir) = android_libcxx_static_dir() {
            bundle_libcxx_shared_for_gradle(&dir);
        }
    }
    let mut build = cc::Build::new();
    build
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
        .define("RARDLL", None);

    // UNRAR_NG_FORCE_UTF8 commits Linux/BSD wide<->8bit filename conversions to
    // the same locale-independent WideToUtf / UtfToWide path that macOS has
    // used for years (raros.hpp:18-20 auto-defines _APPLE on __APPLE__ targets).
    //
    // Gated behind cargo feature `linux-batch-extract-utf8` (default-on). When
    // the feature is disabled, this define is NOT set and libunrar's
    // MBFUNCTIONS branch in unicode.cpp (`wcsrtombs` / `mbsrtowcs`) takes over.
    // In that mode the caller is responsible for `setlocale(LC_CTYPE, "")` —
    // either by calling it themselves before invoking this crate, or by
    // also enabling the high-level crate's `linux-batch-extract-setlocale`
    // cargo feature, which provides a Rust-side `OnceLock`-managed lazy init.
    //
    // Apple is excluded from the gate because raros.hpp already auto-defines
    // _APPLE on __APPLE__ targets — the WideToUtf path runs regardless of
    // this feature. Windows is excluded because the `_WIN_ALL` branch in
    // unicode.cpp uses `WideCharToMultiByte(CP_ACP, ...)` (OS-level system
    // codepage; CP936 zh-CN, CP932 ja-JP, CP65001 if user opted into the
    // Win10 ≥ 1803 / 11 "Beta UTF-8 ACP" toggle) and writes via
    // `CreateFile(LPCWSTR)` (wide-native NTFS). Vendor patch 0007 is a
    // no-op on both Apple and Windows builds.
    //
    // Cargo translates feature `linux-batch-extract-utf8` into the env var
    // `CARGO_FEATURE_LINUX_BATCH_EXTRACT_UTF8` (uppercase, hyphen → underscore).
    // (`target_os` already computed above.)
    let target_vendor = std::env::var("CARGO_CFG_TARGET_VENDOR").unwrap_or_default();
    let feature_linux_batch_extract_utf8 =
        std::env::var("CARGO_FEATURE_LINUX_BATCH_EXTRACT_UTF8").is_ok();
    let force_utf8 = feature_linux_batch_extract_utf8
        && target_os != "windows"
        && target_vendor != "apple";
    if force_utf8 {
        build.define("UNRAR_NG_FORCE_UTF8", None);
    }

    build.files(&files).compile("libunrar.a");
}

/// PATCH (vietzip): locate the Android NDK's per-ABI `usr/lib/<abi>` sysroot directory
/// (where `libc++_static.a` lives) from the `CXX_<target-triple>` env var that cargokit's
/// `AndroidEnvironment.buildEnvironment()` sets for every Android build (confirmed by
/// reading that env var directly during a real build — path shape is
/// `<ndk>/toolchains/llvm/prebuilt/<host>/bin/clang++.exe`). Returns `None` (rather than
/// panicking) if the env var is absent or doesn't parse as expected — e.g. a direct
/// `cargo build`/`cargo ndk` invocation outside cargokit's Gradle-driven build, which sets
/// up its own `-L` search differently and doesn't need this.
// Deliberately NOT gated by `#[cfg(target_os = "android")]` — that would reflect the HOST
// this build script itself is compiled for (always the dev machine, e.g. Windows), not the
// Android TARGET being cross-compiled to. Only called when the `CARGO_CFG_TARGET_OS`
// runtime check at the call site is "android".
fn android_libcxx_static_dir() -> Option<std::path::PathBuf> {
    let target = std::env::var("TARGET").ok()?;
    let cxx = std::env::var(format!("CXX_{target}")).ok()?;
    let toolchain_root = std::path::Path::new(&cxx).parent()?.parent()?; // bin/.. -> <host-arch>/
    let abi_dir = match target.as_str() {
        "armv7-linux-androideabi" => "arm-linux-androideabi",
        other => other, // aarch64/i686/x86_64-linux-android match their NDK dir name as-is
    };
    Some(
        toolchain_root
            .join("sysroot")
            .join("usr")
            .join("lib")
            .join(abi_dir),
    )
}

/// PATCH (vietzip): copy the NDK's `libc++_shared.so` into cargokit's own per-build-type
/// output directory (`$CARGOKIT_OUTPUT_DIR/<android-abi>/`), the same directory cargokit's
/// `plugin.gradle` already registers as a Gradle `jniLibs.srcDir` for the final APK — so
/// this rides along on Gradle's existing "package everything found here" mechanism instead
/// of needing a separate Gradle-level change. `CARGOKIT_OUTPUT_DIR` is set by
/// `android_environment.dart`'s `CargoKitBuildTask` on the `run_build_tool` process and
/// flows down to this build script because `Process.runSync` defaults to
/// `includeParentEnvironment: true` (confirmed by reading cargokit's own
/// `build_tool/lib/src/util.dart`). Silently does nothing if that env var is absent (e.g. a
/// direct `cargo build`/`cargo ndk` invocation outside cargokit) or the target isn't one of
/// the 4 Android ABIs cargokit builds.
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
