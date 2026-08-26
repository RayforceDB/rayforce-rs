# :material-cog-outline: Technical Details

This page explains how `rayforce` is put together: the crate layout, the value
handle, the threading model, and how results are materialized.

## :material-layers-outline: A two-crate workspace

The repository is a Cargo workspace with two crates:

| Crate | Purpose |
|---|---|
| `rayforce-sys` | Raw `unsafe` FFI. Runs [`bindgen`](https://github.com/rust-lang/rust-bindgen) over the core's `rayforce.h`, builds the RayforceDB core (`make lib`), and statically links `librayforce.a`. |
| `rayforce` | The safe, ergonomic API you depend on — values, tables, the query DSL, IPC, and serialization. |

You depend only on `rayforce`. The `rayforce-sys` crate exists to isolate the
generated bindings and the link step; the safe layer is a thin wrapper over it.

## :material-tag-multiple-outline: `Value` — a refcounted RAII handle

Everything the engine holds — an integer atom, a float vector, a dictionary, a
whole table — is a [`Value`](../documentation/data-types/values.md). A `Value` is
a thin handle onto memory owned by the engine, with reference-counted lifetime
managed by Rust's RAII:

- **`Clone`** retains: it bumps the engine refcount and hands back another handle
  to the same payload. Cloning never copies data.
- **`Drop`** releases: it decrements the refcount; the engine frees the payload
  when the last handle goes away.

```rust
use rayforce::{Runtime, Value};
Runtime::scope(|_rt| {
    let a = Value::vec(&[1i64, 2, 3]);
    let b = a.clone();   // same payload, refcount += 1
    drop(b);             // refcount -= 1; `a` still valid
    assert_eq!(a.as_slice::<i64>()?, &[1, 2, 3]);
    Ok(())
})?;
# Ok::<(), rayforce::RayError>(())
```

Because lifetime is handled for you, there is no manual `free` and no
use-after-free: `Drop` keeps handles honest.

That refcount governs the *object*. The *heap* those objects live in is a
separate matter: `ray_runtime_destroy` unmaps every pool without consulting any
object's refcount, so an object with five handles is unmapped exactly like one
with one. Nothing checked at the point of use could make a surviving handle safe
— the pointer is what became invalid. Instead the runtime's life is bounded so
such a handle is never produced; see below.

## :material-cpu-64-bit: Single runtime, single thread, `!Send`

The RayforceDB core runs on a single thread with a thread-local VM and permits
**one live `Runtime` per process**. To make this safe in Rust:

- `Runtime::scope(|rt| …)` is the only way in. It creates the runtime, hands
  your closure a `&Runtime` — which you cannot drop or move out of — and tears
  it down when the closure returns, on the error path and on unwind alike.
  Values built inside are dropped first, because the closure's locals go first.
- `Value`, `Table`, `TcpClient` and `QConnection` are **`!Send`** and
  **`!Sync`**. They cannot be moved or shared across threads, which statically
  prevents you from touching the engine from a thread other than the one that
  owns the runtime.

```rust
use rayforce::{Runtime, Value};

Runtime::scope(|_rt| {
    let v = Value::i64(42);
    // `v` stays on this thread — it is !Send by design.
    Ok(())
})?;
# Ok::<(), rayforce::RayError>(())
```

### What keeps a `Value` inside its scope

The same `!Send` marker, read as a bound. `Runtime::scope` requires `Send` of
its return type and of the closure, and nothing engine-backed satisfies it:

```rust
use rayforce::{Runtime, Value};

// Returning one: rejected, `Value` is !Send.
// let v = Runtime::scope(|rt| rt.eval("(+ 1 1)"))?;

// Assigning one outward: rejected too — a closure is Send only if every
// capture is, and this one captures `&mut Option<Value>`.
// let mut out = None;
// Runtime::scope(|rt| { out = Some(rt.eval("1")?); Ok(()) })?;

// Extracting plain data is the way through.
let sum = Runtime::scope(|rt| Ok(rt.eval("(+ 1 1)")?.as_i64()?))?;
assert_eq!(sum, 2);
# Ok::<(), rayforce::RayError>(())
```

Both rejections read `required by a bound in Runtime::scope`. The cost is that
an unrelated `!Send` capture — an `Rc`, a `RefCell` borrow — is refused too,
with a diagnostic about threads when no thread is involved; construct such
values inside the closure, or move them in.

Calling `eval` or a constructor with no scope open panics rather than working
against a runtime nobody owns, and a nested `Runtime::scope` returns an error
rather than starting a second runtime.

!!! note "Why single-thread?"
    The engine's VM state is thread-local. Rather than hide this behind locks,
    the bindings surface it directly: `!Send`/`!Sync` turns a runtime invariant
    into a compile-time guarantee, and the cost of crossing a thread boundary
    is simply never paid.

## :material-lightning-bolt-outline: Lazy result materialization

Some operations — aggregations in particular — produce results lazily inside the
engine. The safe API always materializes them for you before handing back a
`Value`: by the time `execute()` or `eval()` returns, you hold a concrete value.
You never observe an unmaterialized result.

## :material-content-copy: Zero-copy reads

Reading a numeric vector back into Rust is zero-copy. `value.as_slice::<T>()`
returns a `&[T]` that borrows directly from engine-owned memory — no per-element
conversion, no intermediate `Vec`:

```rust
use rayforce::{Runtime, Value};
Runtime::scope(|_rt| {
    let prices = Value::vec(&[100.0f64, 200.0, 110.0]);
    let slice: &[f64] = prices.as_slice()?;   // borrows engine memory, no copy
    assert_eq!(slice, &[100.0, 200.0, 110.0]);
    Ok(())
})?;
# Ok::<(), rayforce::RayError>(())
```

The same applies to the typed slice readers for temporal and boolean columns
(`date_days_slice`, `time_millis_slice`, `timestamp_nanos_slice`, `bool_slice`).

## :material-arrow-right: Next steps

- [:octicons-database-16: Data Types](../documentation/data-types/overview.md) —
  the full value model.
- [:material-file-document: Documentation overview](../documentation/overview.md).
