use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let rayforce_dir = out_dir.join("rayforce-c");
    let rayforce_github =
        env::var("RAYFORCE_GITHUB").unwrap_or_else(|_| "https://github.com/RayforceDB/rayforce.git".to_string());

    // Check if rayforce is already built
    let lib_path = rayforce_dir.join("librayforce.a");
    let needs_build = !lib_path.exists();

    if needs_build {
        // Clean previous build artifacts
        if rayforce_dir.exists() {
            std::fs::remove_dir_all(&rayforce_dir).ok();
        }

        // Clone rayforce repository
        println!("cargo:warning=Cloning rayforce from {}", rayforce_github);
        let status = Command::new("git")
            .args(["clone", &rayforce_github, rayforce_dir.to_str().unwrap()])
            .status()
            .expect("Failed to clone rayforce repository");

        if !status.success() {
            panic!("Failed to clone rayforce repository");
        }

        // Build rayforce static library
        println!("cargo:warning=Building rayforce static library...");
        let status = Command::new("make")
            .current_dir(&rayforce_dir)
            .args(["lib"])
            .status()
            .expect("Failed to build rayforce");

        if !status.success() {
            panic!("Failed to build rayforce library");
        }
    }

    // Tell cargo to link against the static library
    println!("cargo:rustc-link-search=native={}", rayforce_dir.display());
    println!("cargo:rustc-link-lib=static=rayforce");

    // Link against system libraries
    #[cfg(target_os = "linux")]
    {
        println!("cargo:rustc-link-lib=dylib=m");
        println!("cargo:rustc-link-lib=dylib=dl");
        println!("cargo:rustc-link-lib=dylib=pthread");
    }

    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-lib=dylib=m");
        println!("cargo:rustc-link-lib=dylib=dl");
        println!("cargo:rustc-link-lib=dylib=pthread");
    }

    // Tell cargo to invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-env-changed=RAYFORCE_GITHUB");

    // Generate wrapper.h that includes all necessary headers
    let wrapper_path = manifest_dir.join("wrapper.h");
    let wrapper_content = format!(
        r#"/*
 * Rayforce Rust bindings wrapper header
 * Auto-generated - includes all necessary rayforce headers
 */

#include "{rayforce_dir}/core/rayforce.h"
#include "{rayforce_dir}/core/def.h"
#include "{rayforce_dir}/core/runtime.h"
#include "{rayforce_dir}/core/string.h"
#include "{rayforce_dir}/core/eval.h"
#include "{rayforce_dir}/core/env.h"
#include "{rayforce_dir}/core/format.h"
#include "{rayforce_dir}/core/query.h"
#include "{rayforce_dir}/core/io.h"
#include "{rayforce_dir}/core/binary.h"
#include "{rayforce_dir}/core/guid.h"
#include "{rayforce_dir}/core/date.h"
#include "{rayforce_dir}/core/time.h"
#include "{rayforce_dir}/core/timestamp.h"
#include "{rayforce_dir}/core/error.h"
#include "{rayforce_dir}/core/items.h"
#include "{rayforce_dir}/core/update.h"
#include "{rayforce_dir}/core/compose.h"
"#,
        rayforce_dir = rayforce_dir.display()
    );
    std::fs::write(&wrapper_path, wrapper_content).expect("Failed to write wrapper.h");

    // Generate bindings
    let bindings = bindgen::Builder::default()
        .header(wrapper_path.to_str().unwrap())
        .clang_arg(format!("-I{}", rayforce_dir.join("core").display()))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .allowlist_type("obj_t")
        .allowlist_type("runtime_t")
        .allowlist_type("ray_error_t")
        .allowlist_type("guid_t")
        // Type constants
        .allowlist_var("TYPE_.*")
        .allowlist_var("ERR_.*")
        .allowlist_var("OK")
        .allowlist_var("NULL_.*")
        .allowlist_var("INF_.*")
        .allowlist_var("B8_.*")
        // Constructors
        .allowlist_function("version")
        .allowlist_function("null")
        .allowlist_function("nullv")
        .allowlist_function("atom")
        .allowlist_function("vector")
        .allowlist_function("b8")
        .allowlist_function("c8")
        .allowlist_function("symbol")
        .allowlist_function("symboli64")
        .allowlist_function("adate")
        .allowlist_function("atime")
        .allowlist_function("timestamp")
        .allowlist_function("guid")
        .allowlist_function("guid_from_str")
        .allowlist_function("guid_to_str")
        .allowlist_function("enumerate")
        .allowlist_function("anymap")
        .allowlist_function("table")
        .allowlist_function("dict")
        .allowlist_function("ray_table")
        .allowlist_function("ray_dict")
        // Memory management
        .allowlist_function("clone_obj")
        .allowlist_function("copy_obj")
        .allowlist_function("cow_obj")
        .allowlist_function("rc_obj")
        .allowlist_function("drop_obj")
        .allowlist_function("drop_raw")
        // Errors
        .allowlist_function("ray_error")
        // Accessors
        .allowlist_function("is_null")
        .allowlist_function("type_name")
        // List operations
        .allowlist_function("push_raw")
        .allowlist_function("push_obj")
        .allowlist_function("push_sym")
        .allowlist_function("append_list")
        .allowlist_function("unify_list")
        .allowlist_function("diverse_obj")
        .allowlist_function("pop_obj")
        .allowlist_function("remove_idx")
        .allowlist_function("remove_ids")
        .allowlist_function("remove_obj")
        .allowlist_function("ins_raw")
        .allowlist_function("ins_obj")
        .allowlist_function("ins_sym")
        // Read operations
        .allowlist_function("at_idx")
        .allowlist_function("at_ids")
        .allowlist_function("at_obj")
        .allowlist_function("at_sym")
        // Format
        .allowlist_function("str_from_symbol")
        .allowlist_function("obj_fmt")
        .allowlist_function("string_from_str")
        // Set operations
        .allowlist_function("zero_obj")
        .allowlist_function("set_idx")
        .allowlist_function("set_ids")
        .allowlist_function("set_obj")
        .allowlist_function("resize_obj")
        // Search
        .allowlist_function("find_raw")
        .allowlist_function("find_obj_idx")
        .allowlist_function("find_obj_ids")
        .allowlist_function("find_sym")
        // Cast
        .allowlist_function("cast_obj")
        // Comparison
        .allowlist_function("cmp_obj")
        // Serialization
        .allowlist_function("ser_obj")
        .allowlist_function("de_obj")
        // Parse and eval
        .allowlist_function("parse_str")
        .allowlist_function("eval_str")
        .allowlist_function("eval_obj")
        .allowlist_function("try_obj")
        .allowlist_function("ray_eval_str")
        // Runtime
        .allowlist_function("runtime_create")
        .allowlist_function("runtime_run")
        .allowlist_function("runtime_destroy")
        .allowlist_function("runtime_get_arg")
        .allowlist_function("ray_init")
        .allowlist_function("ray_clean")
        // Query operations
        .allowlist_function("ray_select")
        .allowlist_function("ray_update")
        .allowlist_function("ray_insert")
        .allowlist_function("ray_upsert")
        // Compose operations
        .allowlist_function("ray_table")
        .allowlist_function("ray_dict")
        // Items operations
        .allowlist_function("ray_key")
        .allowlist_function("ray_value")
        .allowlist_function("ray_first")
        .allowlist_function("ray_last")
        .allowlist_function("ray_at")
        .allowlist_function("ray_find")
        .allowlist_function("ray_filter")
        .allowlist_function("ray_where")
        // Dict operations
        .allowlist_function("ray_key")
        .allowlist_function("ray_value")
        // IO operations
        .allowlist_function("ray_hopen")
        .allowlist_function("ray_hclose")
        .allowlist_function("ray_write")
        .allowlist_function("ray_read")
        // Environment
        .allowlist_function("env_get_internal_function")
        .allowlist_function("env_get_internal_name")
        // Binary operations
        .allowlist_function("binary_set")
        // Quote
        .allowlist_function("ray_quote")
        // Reference counting sync
        .allowlist_function("rc_sync_get")
        .allowlist_function("rc_sync_set")
        // Generate
        .generate()
        .expect("Unable to generate bindings");

    let bindings_path = out_dir.join("bindings.rs");
    bindings
        .write_to_file(&bindings_path)
        .expect("Couldn't write bindings!");

    println!("cargo:warning=Bindings generated at {}", bindings_path.display());
}
