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
  crash below on the commit before its fix. Both legs then assert the archive
  they built: `ray_dfd_check_live` must be present on the debug leg and absent on
  the release one. Without that pair, a break in the `RAYFORCE_CORE_DEBUG`
  plumbing would turn the debug leg into a second release run that stays green.

- **The IPC tests run in CI.** `tests/ipc.rs` drives `TcpClient` against a
  spawned server and was returning early for want of one — which reports as a
  pass, so the gap was invisible. CI now builds the server binary, and
  `RAYFORCE_REQUIRE_SERVER=1` turns a missing one into a failure rather than a
  skip. `tests/q_real.rs` still opts out via `RAYFORCE_Q_ADDR`: it needs a real
  `q` server, which cannot be provisioned on a runner.

### Changed

- **The vendored core is v2.6.0 and `rayforce-q` is 2.1.1** (from v2.5.8 and
  2.0.0). The core now recognises in-band nulls at construction, which changes
  what a vector built from a raw buffer reports: `Value::vec(&[1i64, i64::MIN, 3])`
  answers `is_null_at(1)` and `get(1)` returns the null singleton, where before
  the sentinel was ordinary data until `set_null` marked it — the engine scans
  the payload once and raises `HAS_NULLS`, so such values no longer aggregate as
  data. The empty symbol and the empty string are now their types' nulls:
  `is_null_at` reports them, but `get` returns the empty atom rather than the
  null singleton, so `to_vec::<String>()` keeps working and
  `to_vec::<Option<String>>()` yields `None` for them. `set_null(idx, false)` is
  a no-op in the core; overwrite the element with `set` instead. The docs no
  longer describe a "null bitmap": nulls are sentinels behind a `HAS_NULLS`
  fast-path hint.

- **The submodules are addressed over SSH.** `.gitmodules` now points at
  `git@github.com:RayforceDB/rayforce.git` and `rayforce-q.git`. An existing
  clone picks the change up with `git submodule sync --recursive`; CI needs
  nothing, since `actions/checkout` rewrites `git@github.com:` to https with the
  job token. Without a GitHub SSH key, set
  `git config --global url."https://github.com/".insteadOf "git@github.com:"`
  before initializing the submodules — and, for a `git = "https://…"` Cargo
  dependency, `net.git-fetch-with-cli = true` in `~/.cargo/config.toml` so Cargo
  fetches through git and honours the rewrite. crates.io users are unaffected:
  the C sources ship inside the crate.

- **A core-flavour switch rebuilds a `RAYFORCE_SRC` checkout from scratch.**
  Release and debug objects share every filename and `make` tracks headers but
  not flags, so a flavour flip would otherwise archive a mixed library. The
  build script now drops every object under the core's `src/` and the
  `librayforce.a` beside them on the first build after the flags change, and
  records them in an untracked `.stamp` file — in your own checkout as well as
  under `OUT_DIR`, which previously had the only such check. Nothing tracked by
  git is touched.

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

- **Breaking: `is_live()` is now `on_runtime_thread()`**, and answers a
  per-thread question rather than a per-process one. A live runtime is required
  for everything except reading and dropping handles you already hold: `eval`,
  `set_global`, `get_global`, the value constructors and the connection
  constructors all answer to this one predicate, which is true only inside a
  scope *and* only on the thread that entered it. A `false` result does not mean
  a runtime can be created — one may be live on another thread, and
  `Runtime::scope` says so.

### Fixed

- **Engine calls from another thread are refused instead of segfaulting.** The
  liveness flag was a process-wide `AtomicBool`, but everything it guards is
  thread-local: the core's VM (`__VM`) and heap (`ray_tl_heap`) both are. So
  inside a scope, any other thread saw a live runtime and every guard passed —
  `std::thread::spawn(|| rayforce::eval("(+ 1 1)"))` crashed in `ray_eval_str`,
  which dereferences `__VM` with no null check, from safe code with no `unsafe`
  anywhere. Constructors were quieter but not better: off-thread
  `Value::sym("hello")` succeeded, allocating into a per-thread heap that no
  `ray_runtime_destroy` would ever unmap. The guard is now a thread-local, so
  those calls panic naming the thread; creating a runtime stays process-wide,
  because the core's `__RUNTIME` is an unguarded global that a second
  `ray_runtime_create` would overwrite in silence.

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
