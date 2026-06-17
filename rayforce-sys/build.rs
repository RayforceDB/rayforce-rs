//! Build script for `rayforce-sys`.
//!
//! 1. Locates the RayforceDB v2 core source tree (env `RAYFORCE_SRC`, default
//!    `~/rayforce`).
//! 2. Builds the static library `librayforce.a` via the core's `make lib`
//!    (incremental — a no-op when objects are up to date).
//! 3. Emits the static link directives.
//! 4. Generates Rust bindings from `wrapper.h` with `bindgen`.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let core = core_src_dir();
    let include = core.join("include");
    let header = include.join("rayforce.h");

    assert!(
        header.exists(),
        "rayforce core header not found at {}.\n\
         Set RAYFORCE_SRC to your rayforce checkout (default: ~/rayforce).",
        header.display()
    );

    sanitize_libclang_path();
    build_core_lib(&core);

    // --- linking ---
    println!("cargo:rustc-link-search=native={}", core.display());
    println!("cargo:rustc-link-lib=static=rayforce");
    println!("cargo:rustc-link-lib=dylib=m");
    if cfg!(target_os = "linux") {
        println!("cargo:rustc-link-lib=dylib=pthread");
    }
    // Expose the core dir to downstream crates (e.g. for symfile fixtures).
    println!("cargo:root={}", core.display());

    // --- bindgen ---
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg(format!("-I{}", include.display()))
        // Keep the surface tight and deterministic.
        .allowlist_function("ray_.*")
        .allowlist_type("ray_.*")
        .allowlist_var("RAY_.*")
        .allowlist_var("NULL_.*")
        .allowlist_var("__ray_.*")
        .allowlist_var("ray_type_sizes")
        // ray_t is a union with a flexible array member + nested anon structs;
        // let bindgen represent it faithfully.
        .layout_tests(true)
        .derive_debug(false)
        .generate_comments(false)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("failed to generate rayforce bindings");

    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out.join("bindings.rs"))
        .expect("failed to write bindings.rs");

    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=RAYFORCE_SRC");
    println!("cargo:rerun-if-changed={}", header.display());
    // Relink when the core archive changes (e.g. the C core was rebuilt).
    // `make lib` is incremental, so this doesn't cause perpetual rebuilds.
    // For a guaranteed pickup after editing core sources, touch build.rs or
    // `cargo clean -p rayforce-sys`.
    println!(
        "cargo:rerun-if-changed={}",
        core.join("librayforce.a").display()
    );
}

fn core_src_dir() -> PathBuf {
    if let Ok(p) = env::var("RAYFORCE_SRC") {
        return PathBuf::from(p);
    }
    let home = env::var("HOME").expect("HOME not set and RAYFORCE_SRC unset");
    Path::new(&home).join("rayforce")
}

fn sanitize_libclang_path() {
    println!("cargo:rerun-if-env-changed=LIBCLANG_PATH");
    let Ok(p) = env::var("LIBCLANG_PATH") else {
        return;
    };
    let dir = Path::new(&p);
    let has_libclang = std::fs::read_dir(dir).is_ok_and(|entries| {
        entries.flatten().any(|e| {
            let name = e.file_name();
            let name = name.to_string_lossy();
            name.starts_with("libclang")
                && (name.contains(".so") || name.contains(".dylib") || name.contains(".dll"))
        })
    });
    if !has_libclang {
        println!(
            "cargo:warning=LIBCLANG_PATH ({p}) contains no libclang; ignoring it \
             so bindgen can auto-detect the system libclang."
        );
        // Safe: single-threaded build script, before bindgen runs.
        env::remove_var("LIBCLANG_PATH");
    }
}

fn build_core_lib(core: &Path) {
    let status = Command::new("make")
        .arg("lib")
        .current_dir(core)
        .status()
        .expect("failed to invoke `make` to build librayforce.a");
    assert!(
        status.success(),
        "`make lib` failed in {} (exit {:?})",
        core.display(),
        status.code()
    );
    assert!(
        core.join("librayforce.a").exists(),
        "make lib succeeded but librayforce.a is missing in {}",
        core.display()
    );
}
