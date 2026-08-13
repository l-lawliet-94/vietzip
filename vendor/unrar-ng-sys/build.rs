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
