# :material-history: Changelog

All notable changes to `rayforce` are documented here. This project adheres to
[Semantic Versioning](https://semver.org).

## Unreleased

### Added

- **CI runs the suite against a debug-flavour engine.** Set
  `RAYFORCE_CORE_DEBUG=1` and `rayforce-sys` builds `librayforce.a` with
  `-DDEBUG`, which compiles in the core's invariant checks and its stale
  retain/release detector; arm it at runtime with `RAY_DFD=1`. This is the only
  tool that sees a use-after-free inside the engine's `mmap`-backed pool
  allocator — AddressSanitizer and Valgrind track `malloc`, which the engine
  never calls, and Miri cannot execute the C library at all. The `test` job now
  runs both flavours; the debug leg reproduces the `Value`-outliving-`Runtime`
  crash below on the commit before its fix.

- **The IPC tests run in CI.** `tests/ipc.rs` drives `TcpClient` against a
  spawned server and was returning early for want of one — which reports as a
  pass, so the gap was invisible. CI now builds the server binary, and
  `RAYFORCE_REQUIRE_SERVER=1` turns a missing one into a failure rather than a
  skip. `tests/q_real.rs` still opts out via `RAYFORCE_Q_ADDR`: it needs a real
  `q` server, which cannot be provisioned on a runner.

### Changed

- **Breaking: `Runtime::scope` replaces `Runtime::new`.** `Runtime::new` is
  private; the only way to a runtime is
  `Runtime::scope(|rt| { … })`, which creates it, hands the closure a
  `&Runtime` you cannot drop or move out of, and tears it down when the closure
  returns — on the error path and on unwind alike. A nested scope errors rather
  than starting a second runtime. Migration is mechanical: delete
  `let _rt = Runtime::new()?;`, wrap the body, end it with `Ok(())`.

- **Nothing engine-backed leaves a scope.** `Runtime::scope` requires `Send` of
  its return type and of the closure, and `Value`, `Table`, `Fn`, `TcpClient`
  and `QConnection` are all `!Send` — so returning one, or assigning one into a
  variable declared outside, is a compile error reading `required by a bound in
  Runtime::scope`. The cost is that an unrelated `!Send` capture (an `Rc`, a
  `RefCell` borrow) is refused too, with a diagnostic about threads when no
  thread is involved; construct such values inside the closure, or move them in.

- **A live runtime is required for everything except reading and dropping
  handles you already hold.** `eval`, `set_global`, `get_global`, the value
  constructors and the connection constructors all answer to the same
  `is_live()`, which is true only inside a scope.

### Fixed

- **A `Value` can no longer outlive its `Runtime`.** Dropping the runtime
  unmaps the engine heap, so a handle still alive afterwards released into
  memory that is no longer mapped. No check at the point of use could have
  helped: `ray_t.rc` counts references to an *object*, while
  `ray_runtime_destroy` munmaps every pool without consulting it, and by the
  time a stale handle is used the thing to check is the pointer — which is what
  became invalid. `Runtime::scope` removes the shape instead: the closure's
  locals are dropped before the runtime is, and its `Send` bounds stop a value
  leaving. `Value` stays one pointer wide, with no bookkeeping on clone or drop.

- **The connection types are confined to their scope too.** `TcpClient` and
  `QConnection` had no liveness tracking of any kind, so a client outliving its
  `Runtime` called `ray_ipc_close` / `q_close` against an unmapped heap. Both
  are now `!Send`/`!Sync` with `compile_fail` markers pinning it, which is what
  the scope's bounds read, and both `Drop`s run before the runtime's.

- **Building a value requires a live `Runtime`.** `Value::i64(1)` with no runtime
  was safe Rust calling straight into the engine with no check at all. It did not
  crash, which is why it went unnoticed: `ray_alloc` lazily maps a heap when none
  exists, so the value landed in an orphan one. The sharp case was symbols, which
  are runtime-scoped — `Value::sym("hello")` returned an *empty* symbol, dropping
  the string with no error anywhere.

- **The runtime tears down its event loop.** `TcpClient::connect` installs a
  poll on first use and `ray_runtime_destroy` does not touch it, so it leaked.
  `Runtime`'s `Drop` now takes it down first, while the heap it releases
  selector state into is still there.

- **`QConnection` is `!Send`/`!Sync`**, like every other handle in the crate.
  It was a bare file descriptor, so it inferred both, while `execute` interns
  symbols and builds engine objects that belong to the runtime's thread.

- Building with `--no-default-features` (no `chrono`) is now warning-free.


## 1.0.1

### Added

- **Decode Q wire messages from an external transport.** New
  [`q::decode_response`](documentation/ipc.md) turns a complete Q IPC message
  (8-byte wire header + body, compressed or not) into a `Value`. This lets
  socket I/O live in a separate transport thread that owns a plain `TcpStream`
  and just moves bytes, while deserialization into engine objects stays on the
  thread that owns the `Runtime`. Q server-side errors surface as `Err`.


## 1.0.0

Initial release of the Rust bindings for RayforceDB v2.

### Added

- **Value model.** A single reference-counted [`Value`](documentation/data-types/values.md)
  handle (`Clone` = retain, `Drop` = release) covering all atom types — bool,
  `u8`, `i16`/`i32`/`i64`, `f32`/`f64`, symbol, string, date, time, timestamp,
  and GUID — plus typed nulls.
- **Containers.** Vectors with zero-copy `as_slice::<T>()` reads, lists, and
  dictionaries.
- **Tables.** [`Table::new`](documentation/table/overview.md) from typed columns, column/row
  accessors, `head`/`tail`/`take`, and inner/left/asof joins.
- **Query DSL.** A fluent builder over `select` and `update` with `col(..)`
  expressions, arithmetic operator overloads, comparison and aggregation
  methods, filtering, grouping (`by`), and ordering.
- **CSV & splayed I/O.** `read_csv` / `write_csv`, plus `save_splayed`,
  `load_splayed`, and `load_parted` for on-disk columnar data.
- **Serialization.** [`Value::serialize`](documentation/serialization.md) /
  `Value::deserialize` round-trips using RayforceDB's native wire format.
- **Conversions.** `ToValue` / `FromValue` for native Rust types and an optional
  `chrono` feature (default) for temporal interop.
- **IPC client.** [`TcpClient`](documentation/ipc.md) to connect to a running RayforceDB
  server, `execute` queries, and `send` / `send_async` values.

### Notes

- A single live `Runtime` per process; `Value`, `Table`, and `TcpClient` are
  `!Send`/`!Sync`.
- An embedded IPC server, window joins, pivots, and feature-gated
  dataframe/SQL plugins are planned for future releases.
