/* bindgen entry point for rayforce-sys.
 *
 * Pulls in the public v2 API, then hand-declares the handful of internal
 * core symbols the safe crate needs (all present in librayforce.a but not in
 * the public header — see PLAN.md). Signatures copied verbatim from the core
 * sources: src/lang/eval.h, src/lang/internal.h, src/store/serde.h. */

#include <rayforce.h>

#ifdef __cplusplus
extern "C" {
#endif

/* src/lang/eval.h — evaluate an already-compiled AST object */
ray_t* ray_eval(ray_t* obj);

/* src/lang/internal.h — query builtins (variadic arg-array form) */
ray_t* ray_update_fn(ray_t** args, int64_t n);
ray_t* ray_insert_fn(ray_t** args, int64_t n);
ray_t* ray_upsert_fn(ray_t** args, int64_t n);

/* src/lang/internal.h — CSV + on-disk table I/O */
ray_t* ray_read_csv_fn(ray_t** args, int64_t n);
ray_t* ray_write_csv_fn(ray_t** args, int64_t n);
ray_t* ray_set_splayed_fn(ray_t** args, int64_t n);
ray_t* ray_get_splayed_fn(ray_t** args, int64_t n);
ray_t* ray_get_parted_fn(ray_t** args, int64_t n);

/* src/ops/ops.h — materialize a lazy DAG result (no-op for non-lazy inputs;
 * consumes the lazy reference on success). RAY_LAZY (104) is the deferred type
 * returned by ray_eval and graph-aware builtins. */
ray_t* ray_lazy_materialize(ray_t* val);

/* src/store/serde.h — serialize / deserialize to a U8 vector with IPC header */
ray_t* ray_ser(ray_t* obj);
ray_t* ray_de(ray_t* bytes);

/* src/core/runtime.c — last per-VM error message (set alongside a RAY_ERROR) */
const char* ray_error_msg(void);

/* The poll object the IPC client needs (ray_poll_create / ray_runtime_get_poll
 * / ray_runtime_set_poll) is exported by the public header as of core v2.5.8,
 * so it is no longer hand-declared here. ray_poll_create now returns the typed
 * `ray_poll_t*` rather than the old `void*` — see the cast in
 * rayforce/src/ipc.rs::ensure_poll. */

#ifdef __cplusplus
}
#endif
