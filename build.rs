/*
*   Copyright (c) 2025-2026 Anton Kundenko <singaraiona@gmail.com>
*   All rights reserved.

*   Permission is hereby granted, free of charge, to any person obtaining a copy
*   of this software and associated documentation files (the "Software"), to deal
*   in the Software without restriction, including without limitation the rights
*   to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
*   copies of the Software, and to permit persons to whom the Software is
*   furnished to do so, subject to the following conditions:

*   The above copyright notice and this permission notice shall be included in all
*   copies or substantial portions of the Software.

*   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
*   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
*   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
*   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
*   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
*   OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
*   SOFTWARE.
*/

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let rayforce_dir = out_dir.join("rayforce-c");
    let rayforce_github =
        env::var("RAYFORCE_GITHUB").unwrap_or_else(|_| "https://github.com/RayforceDB/rayforce.git".to_string());

    let lib_path = rayforce_dir.join("librayforce.a");
    let needs_build = !lib_path.exists();

    if needs_build {
        if rayforce_dir.exists() {
            std::fs::remove_dir_all(&rayforce_dir).ok();
        }

        println!("cargo:warning=Cloning rayforce from {}", rayforce_github);
        let status = Command::new("git")
            .args(["clone", &rayforce_github, rayforce_dir.to_str().unwrap()])
            .status()
            .expect("Failed to clone rayforce repository");

        if !status.success() {
            panic!("Failed to clone rayforce repository");
        }

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

    println!("cargo:rustc-link-search=native={}", rayforce_dir.display());
    println!("cargo:rustc-link-lib=static=rayforce");

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

    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-env-changed=RAYFORCE_GITHUB");

    let include_dir = rayforce_dir.join("include");
    let public_header = include_dir.join("rayforce.h");

    let wrapper_path = manifest_dir.join("wrapper.h");
    let wrapper_content = format!(
        r#"/*
 * Rayforce Rust bindings wrapper header
 * Auto-generated - includes the single rayforce 2.0 public header.
 */

#include "{header}"
"#,
        header = public_header.display()
    );
    std::fs::write(&wrapper_path, wrapper_content).expect("Failed to write wrapper.h");

    let bindings = bindgen::Builder::default()
        .header(wrapper_path.to_str().unwrap())
        .clang_arg(format!("-I{}", include_dir.display()))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        // Core types
        .allowlist_type("ray_t")
        .allowlist_type("ray_err_t")
        .allowlist_type("ray_runtime_s")
        .allowlist_type("ray_runtime_t")
        .allowlist_type("ray_progress_t")
        .allowlist_type("ray_progress_cb")
        // Type tag constants and attribute flags
        .allowlist_var("RAY_.*")
        // Versioning
        .allowlist_function("ray_version_major")
        .allowlist_function("ray_version_minor")
        .allowlist_function("ray_version_patch")
        .allowlist_function("ray_version_string")
        // Memory / allocator
        .allowlist_function("ray_alloc")
        .allowlist_function("ray_free")
        .allowlist_function("ray_mem_budget")
        .allowlist_function("ray_mem_pressure")
        // Interrupt + Progress
        .allowlist_function("ray_request_interrupt")
        .allowlist_function("ray_clear_interrupt")
        .allowlist_function("ray_interrupted")
        .allowlist_function("ray_progress_set_callback")
        .allowlist_function("ray_progress_update")
        .allowlist_function("ray_progress_label")
        .allowlist_function("ray_progress_end")
        // COW / refcount
        .allowlist_function("ray_retain")
        .allowlist_function("ray_release")
        // Atom constructors
        .allowlist_function("ray_bool")
        .allowlist_function("ray_u8")
        .allowlist_function("ray_i16")
        .allowlist_function("ray_i32")
        .allowlist_function("ray_i64")
        .allowlist_function("ray_f32")
        .allowlist_function("ray_f64")
        .allowlist_function("ray_str")
        .allowlist_function("ray_sym")
        .allowlist_function("ray_date")
        .allowlist_function("ray_time")
        .allowlist_function("ray_timestamp")
        .allowlist_function("ray_guid")
        .allowlist_function("ray_typed_null")
        // Vector API
        .allowlist_function("ray_vec_new")
        .allowlist_function("ray_sym_vec_new")
        .allowlist_function("ray_vec_append")
        .allowlist_function("ray_vec_set")
        .allowlist_function("ray_vec_get")
        .allowlist_function("ray_vec_slice")
        .allowlist_function("ray_vec_concat")
        .allowlist_function("ray_vec_from_raw")
        .allowlist_function("ray_vec_insert_at")
        .allowlist_function("ray_vec_insert_vec_at")
        .allowlist_function("ray_vec_insert_many")
        .allowlist_function("ray_vec_set_null")
        .allowlist_function("ray_vec_set_null_checked")
        .allowlist_function("ray_vec_is_null")
        // String vector / string atom
        .allowlist_function("ray_str_vec_append")
        .allowlist_function("ray_str_vec_get")
        .allowlist_function("ray_str_vec_set")
        .allowlist_function("ray_str_vec_insert_at")
        .allowlist_function("ray_str_vec_compact")
        .allowlist_function("ray_str_ptr")
        .allowlist_function("ray_str_len")
        .allowlist_function("ray_str_cmp")
        // List API
        .allowlist_function("ray_list_new")
        .allowlist_function("ray_list_append")
        .allowlist_function("ray_list_get")
        .allowlist_function("ray_list_set")
        .allowlist_function("ray_list_insert_at")
        .allowlist_function("ray_list_insert_many")
        // Symbol intern table
        .allowlist_function("ray_sym_init")
        .allowlist_function("ray_sym_destroy")
        .allowlist_function("ray_sym_intern")
        .allowlist_function("ray_sym_find")
        .allowlist_function("ray_sym_str")
        .allowlist_function("ray_sym_count")
        .allowlist_function("ray_sym_ensure_cap")
        .allowlist_function("ray_sym_save")
        .allowlist_function("ray_sym_load")
        // Environment
        .allowlist_function("ray_env_get")
        .allowlist_function("ray_env_set")
        // Table
        .allowlist_function("ray_table_new")
        .allowlist_function("ray_table_add_col")
        .allowlist_function("ray_table_get_col")
        .allowlist_function("ray_table_get_col_idx")
        .allowlist_function("ray_table_col_name")
        .allowlist_function("ray_table_set_col_name")
        .allowlist_function("ray_table_ncols")
        .allowlist_function("ray_table_nrows")
        .allowlist_function("ray_table_schema")
        // Dict
        .allowlist_function("ray_dict_new")
        .allowlist_function("ray_dict_keys")
        .allowlist_function("ray_dict_vals")
        .allowlist_function("ray_dict_len")
        .allowlist_function("ray_dict_get")
        .allowlist_function("ray_dict_upsert")
        .allowlist_function("ray_dict_remove")
        // Runtime + eval
        .allowlist_function("ray_runtime_create")
        .allowlist_function("ray_runtime_create_with_sym")
        .allowlist_function("ray_runtime_create_with_sym_err")
        .allowlist_function("ray_runtime_destroy")
        .allowlist_function("ray_eval_str")
        // Errors
        .allowlist_function("ray_error")
        .allowlist_function("ray_err_code_str")
        .allowlist_function("ray_err_from_obj")
        .allowlist_function("ray_err_code")
        .allowlist_function("ray_error_free")
        // Globals
        .allowlist_var("__ray_null")
        .allowlist_var("__ray_oom")
        .allowlist_var("ray_type_sizes")
        .generate()
        .expect("Unable to generate bindings");

    let bindings_path = out_dir.join("bindings.rs");
    bindings
        .write_to_file(&bindings_path)
        .expect("Couldn't write bindings!");

    println!("cargo:warning=Bindings generated at {}", bindings_path.display());
}
