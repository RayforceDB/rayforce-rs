//! Build script for `rayforce-sys`.
//!
//! 1. Locates the RayforceDB v2 core + `rayforce-q` source trees. Both ship
//!    inside this crate as git submodules under `vendor/` (see [`core_src_dir`]
//!    / [`q_src_dir`]), so a build never touches the network — docs.rs and
//!    other sandboxes have none.
//! 2. Stages the core into `OUT_DIR` ([`stage_core`]) and builds the static
//!    library `librayforce.a` there via the core's `make lib` (incremental — a
//!    no-op when objects are up to date).
//! 3. Emits the static link directives.
//! 4. Generates Rust bindings from `wrapper.h` with `bindgen`.

use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Version stamped into the vendored core at compile time. The core's Makefile
/// normally resolves this from `git describe` (`Makefile:19`), but a crate
/// unpacked from crates.io has no git history — without this the core would
/// report itself as `0.0.0`.
///
/// Must match the tag `vendor/rayforce` is pinned to. CI asserts the two agree;
/// see the "Check vendored core pin" step in `.github/workflows/ci.yml`.
const CORE_VERSION: &str = "2.5.8";

/// Commit the `vendor/rayforce` submodule is pinned to, stamped alongside
/// [`CORE_VERSION`]. Also checked by CI's "Check vendored core pin" step.
///
/// This must be passed explicitly for the same reason as the version, and for
/// one more: `Makefile:27` resolves it with `git rev-parse --short HEAD`, and
/// git searches *upward* from the working directory. Since the core is built
/// under OUT_DIR, an unset value does not fall back to "unknown" — it silently
/// reports the HEAD of whatever unrelated repository happens to enclose the
/// build directory.
const CORE_COMMIT: &str = "f0d4bb4";

/// Warning flags for the vendored core build — the core's own `WARNS`
/// (`Makefile:30`) minus `-Werror`. Consumers compile this with whatever
/// toolchain they happen to have, and a new diagnostic from a future compiler
/// should not be a hard failure inside someone else's dependency tree. The
/// core's own CI is where `-Werror` belongs.
const CORE_WARNS: &str = "-Wall -Wextra -Wstrict-prototypes -Wno-unused-parameter";

fn main() {
    // An explicit override means the caller brought their own core, with its
    // own git history and its own version; only stamp CORE_VERSION on ours.
    let core_is_vendored = env::var_os("RAYFORCE_SRC").is_none();
    let core_src = core_src_dir();
    let header_src = core_src.join("include/rayforce.h");

    assert!(
        header_src.exists(),
        "rayforce core header not found at {}.\n\
         If this is a git checkout, the vendored core submodule is not \
         initialized — run `git submodule update --init --recursive`.\n\
         To build against a different core, point RAYFORCE_SRC at it.",
        header_src.display()
    );

    // Our own vendored copy has to be staged into OUT_DIR before it is built;
    // a checkout the caller pointed us at is theirs, and building it in place
    // keeps their incremental state and the version its git history reports.
    let core = if core_is_vendored {
        stage_core(&core_src)
    } else {
        core_src.clone()
    };
    let include = core.join("include");

    sanitize_libclang_path();
    build_core_lib(&core, core_is_vendored);

    // --- Q IPC client (rayforce-q's q.c) ---
    // Linked BEFORE librayforce so its undefined `ray_*` symbols resolve from
    // the core archive. Needs the core's private `src/` on the include path
    // (`table/sym.h`).
    let q_src = q_src_dir();
    let q_c = q_src.join("q.c");
    assert!(
        q_c.exists(),
        "rayforce-q client not found at {}.\n\
         If this is a git checkout, the vendored submodule is not initialized \
         — run `git submodule update --init --recursive`.\n\
         To build against a different checkout, point RAYFORCE_Q_SRC at it.",
        q_c.display()
    );
    cc::Build::new()
        .file(&q_c)
        .include(&q_src)
        .include(&include)
        .include(core.join("src"))
        .warnings(false)
        .compile("rayforce_q");
    println!("cargo:rerun-if-changed={}", q_c.display());
    println!("cargo:rerun-if-changed={}", q_src.join("q.h").display());

    // --- linking ---
    println!("cargo:rustc-link-search=native={}", core.display());
    println!("cargo:rustc-link-lib=static=rayforce");
    println!("cargo:rustc-link-lib=dylib=m");
    if cfg!(target_os = "linux") {
        println!("cargo:rustc-link-lib=dylib=pthread");
    }
    // Expose the staged core dir to downstream crates (e.g. for symfile
    // fixtures) — this is where librayforce.a and the headers actually live.
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

    bindings
        .write_to_file(out_dir().join("bindings.rs"))
        .expect("failed to write bindings.rs");

    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=build.rs");
}

/// Resolve the RayforceDB core source tree, in order of precedence:
/// 1. `RAYFORCE_SRC` — an explicit checkout, for building the bindings against
///    an unreleased core.
/// 2. `vendor/rayforce` — the submodule shipped inside this crate, pinned to
///    [`CORE_VERSION`]. Present both in a git checkout (once submodules are
///    initialized) and in the `.crate` published to crates.io.
fn core_src_dir() -> PathBuf {
    println!("cargo:rerun-if-env-changed=RAYFORCE_SRC");
    let src = match env::var("RAYFORCE_SRC") {
        Ok(p) => PathBuf::from(p),
        Err(_) => vendored("rayforce"),
    };
    // Rebuild on a submodule bump, or on an edit to a RAYFORCE_SRC checkout.
    println!("cargo:rerun-if-changed={}", src.join("Makefile").display());
    println!("cargo:rerun-if-changed={}", src.join("include").display());
    println!("cargo:rerun-if-changed={}", src.join("src").display());
    src
}

/// Resolve the `rayforce-q` source tree; same precedence as [`core_src_dir`],
/// keyed off `RAYFORCE_Q_SRC` / the `vendor/rayforce-q` submodule.
fn q_src_dir() -> PathBuf {
    println!("cargo:rerun-if-env-changed=RAYFORCE_Q_SRC");
    match env::var("RAYFORCE_Q_SRC") {
        Ok(p) => PathBuf::from(p),
        Err(_) => vendored("rayforce-q"),
    }
}

/// Path to a submodule under `vendor/`, resolved against the crate root so it
/// works from a git checkout and from an unpacked `.crate` alike.
fn vendored(name: &str) -> PathBuf {
    PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is always set"))
        .join("vendor")
        .join(name)
}

fn out_dir() -> PathBuf {
    PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is always set"))
}

/// Mirror the parts of the vendored core that `make lib` needs into
/// `OUT_DIR/core`, and return that path.
///
/// The core's Makefile builds strictly in-tree — `Makefile:129` names objects
/// `src/<dir>/<file>.rel.o` and `Makefile:185` drops `librayforce.a` at the
/// root — so running it where the sources sit would write into the crate's own
/// directory. For a crates.io consumer that is the shared registry cache, and
/// it is what makes `cargo package`'s verify step fail with "files added".
/// Staging keeps the usual rule that a build script writes only under OUT_DIR.
///
/// Copies are skipped when the destination is already current, so `make` stays
/// incremental across rebuilds (OUT_DIR persists).
fn stage_core(src: &Path) -> PathBuf {
    let dst = out_dir().join("core");
    let mut staged = HashSet::new();
    copy_if_stale(&src.join("Makefile"), &dst.join("Makefile"), &mut staged);
    mirror(&src.join("include"), &dst.join("include"), &mut staged);
    mirror(&src.join("src"), &dst.join("src"), &mut staged);
    prune_stale(&dst, &staged);
    dst
}

/// Recursively copy `.c` / `.h` files from `src` into `dst`, recording every
/// destination touched in `staged`.
fn mirror(src: &Path, dst: &Path, staged: &mut HashSet<PathBuf>) {
    let entries =
        fs::read_dir(src).unwrap_or_else(|e| panic!("failed to read {}: {e}", src.display()));
    for entry in entries.flatten() {
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            mirror(&from, &to, staged);
        } else if is_source(&from) {
            copy_if_stale(&from, &to, staged);
        }
    }
}

fn is_source(p: &Path) -> bool {
    matches!(p.extension().and_then(|e| e.to_str()), Some("c" | "h"))
}

fn copy_if_stale(from: &Path, to: &Path, staged: &mut HashSet<PathBuf>) {
    staged.insert(to.to_path_buf());
    if is_current(from, to) {
        return;
    }
    let parent = to.parent().expect("staged paths always have a parent");
    fs::create_dir_all(parent)
        .unwrap_or_else(|e| panic!("failed to create {}: {e}", parent.display()));
    fs::copy(from, to)
        .unwrap_or_else(|e| panic!("failed to copy {} to {}: {e}", from.display(), to.display()));
}

/// `fs::copy` does not preserve mtime, so a freshly staged file is always newer
/// than its source; the size check guards against a same-instant edit.
fn is_current(from: &Path, to: &Path) -> bool {
    let (Ok(f), Ok(t)) = (from.metadata(), to.metadata()) else {
        return false;
    };
    match (f.modified(), t.modified()) {
        (Ok(fm), Ok(tm)) => tm >= fm && f.len() == t.len(),
        _ => false,
    }
}

/// Delete staged sources that no longer exist upstream. Without this, a file
/// dropped by a core version bump would linger in OUT_DIR and still be compiled
/// in via the Makefile's `$(wildcard src/*/*.c)` (`Makefile:120`). Only `.c` /
/// `.h` are considered, so the objects and archive built here survive.
fn prune_stale(dst: &Path, staged: &HashSet<PathBuf>) {
    for root in [dst.join("src"), dst.join("include")] {
        walk(&root, &mut |path| {
            if is_source(path) && !staged.contains(path) {
                let _ = fs::remove_file(path);
            }
        });
    }
}

/// Visit every file under `root`. Missing directories are simply empty.
fn walk(root: &Path, visit: &mut dyn FnMut(&Path)) {
    let mut dirs = vec![root.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                dirs.push(path);
            } else {
                visit(&path);
            }
        }
    }
}

/// Drop the compiled objects when the flags stamped into them change. The
/// Makefile tracks header dependencies (`Makefile:143`) but not flag changes,
/// so editing [`CORE_VERSION`] on its own would otherwise leave the previous
/// string baked into objects that `make` still considers up to date.
fn invalidate_on_stamp_change(core: &Path, stamp: &str) {
    let marker = core.join(".stamp");
    if fs::read_to_string(&marker).is_ok_and(|current| current == stamp) {
        return;
    }
    walk(&core.join("src"), &mut |path| {
        if path.extension().is_some_and(|e| e == "o") {
            let _ = fs::remove_file(path);
        }
    });
    let _ = fs::remove_file(core.join("librayforce.a"));
    fs::write(&marker, stamp)
        .unwrap_or_else(|e| panic!("failed to write {}: {e}", marker.display()));
}

fn sanitize_libclang_path() {
    println!("cargo:rerun-if-env-changed=LIBCLANG_PATH");
    let Ok(p) = env::var("LIBCLANG_PATH") else {
        return;
    };
    let dir = Path::new(&p);
    let has_libclang = fs::read_dir(dir).is_ok_and(|entries| {
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

/// Run the core's `make lib`. `stamp_version` is set when the core is our
/// pinned submodule staged under OUT_DIR, rather than a `RAYFORCE_SRC`
/// checkout building in place with its own git history.
fn build_core_lib(core: &Path, stamp_version: bool) {
    // Cargo budgets build-script parallelism via NUM_JOBS. Without it make runs
    // serially — minutes of wall clock for ~90 translation units at -O3, which
    // matters inside docs.rs's capped build.
    let jobs = env::var("NUM_JOBS").unwrap_or_else(|_| "1".to_string());

    // Make command-line assignments override the Makefile's own definitions,
    // including `?=` ones.
    let mut defs = vec![format!("WARNS={CORE_WARNS}")];
    if stamp_version {
        // Staged under OUT_DIR, with no git history of its own — and `git`
        // searches upward, so leaving these unset would report the enclosing
        // repository rather than falling back to "unknown".
        defs.push(format!("RAY_VERSION={CORE_VERSION}"));
        defs.push(format!("GIT_HASH={CORE_COMMIT}"));
        invalidate_on_stamp_change(core, &defs.join(" "));
    }

    let status = Command::new("make")
        .arg("lib")
        .arg(format!("-j{jobs}"))
        .args(&defs)
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
