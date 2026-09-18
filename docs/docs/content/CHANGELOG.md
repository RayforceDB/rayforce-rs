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

- **`rayforce-sys` binds the core's new public entry points.** v2.7.0 added
  `ray_delete` — an in-place row removal on a named table that keeps its key
  map — and `ray_shallow_bytes` / `ray_retained_bytes`, the native footprint of
  a value in two well-defined senses, meant for charging a foreign runtime's
  garbage collector for a wrapper that keeps a Rayforce value alive. All three
  are declared in `include/rayforce.h`, so bindgen picks them up with no
  hand-written declaration; the safe crate does not wrap them yet.

### Changed

- **The vendored core is v2.8.0 and `rayforce-q` is `1eabaf4`** (from v2.5.8 and
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

- **A failed `TcpClient::connect` says why.** Every negative return from the
  core collapsed into `connect to {host}:{port} failed`, which reads the same
  whether nothing was listening, the password was wrong, or the peer speaks a
  wire version this build would misparse every atom of. The core distinguishes
  six causes — v2.6.1 added two of them — so the message now ends in
  `connection refused`, `authentication failed`, `wire version mismatch`,
  `timed out`, or the OS error text, the same shape `QConnection::connect_with`
  has always used for its own three codes. Two of those read less plainly than
  they look: `timed out` also covers a server that is alive but busy inside a
  long evaluation, because the core folds `EAGAIN`/`EWOULDBLOCK` in with
  `ETIMEDOUT`, and a host that fails to resolve surfaces as `No route to host`,
  which is the `errno` the core stamps on that failure. `rayforce-sys` gained
  `RAY_IPC_ERR_*` constants for the codes, mirroring the `Q_ERR_*` ones — the
  public header declares no contract for them, so they are maintained by hand
  against `connect_fail_code()` in the core, and an unrecognised code still
  falls through to `connection refused`. One cause stays out of reach:
  `server requires authentication` needs a null password, and an empty `&str`
  arrives as a valid pointer to an empty string.

- **`QConnection` no longer takes the process down when a q peer disappears.**
  The `rayforce-q` pin moves off the 2.1.1 tag to `1eabaf4` — six fixes to
  `q.c`, the one file of that repo this crate compiles, and no tag carries them
  yet. Writing to a closed peer used to raise `SIGPIPE`, whose default
  disposition kills the process: a library has no business doing that to its
  host, and `q_send_all` now passes `MSG_NOSIGNAL` (`SO_NOSIGPIPE` on the BSDs).
  A reply is accepted only when the frame says it is one, instead of any message
  type being decoded as the answer to the request in flight. A q identity reply
  (`::`, what an assignment answers) decodes to the null object rather than
  failing the exchange with "unsupported wire type". A native `RAY_DICT` result
  now encodes, where the serializer had no branch for it and gave up. And a
  malformed reply whose decode left trailing bytes freed an error object through
  `ray_release` rather than `ray_error_free`. `q.h` is untouched, so nothing in
  this crate's FFI declarations moves.

- **`count (distinct …)` counts a null as a value inside `by:` groups.** The
  v2.6.1 core retires the per-group kernel's null-skipping arm: a grouped
  `count distinct` over a null-bearing column now answers one more than it did,
  matching what the ungrouped form has always returned. The old convention was
  not even self-consistent — the serial, partitioned and per-group-buffer
  kernels disagreed, so the answer moved with the row count, the group count and
  the core count. Nothing in this crate's surface changes; the numbers coming
  back from `Select::by(…)` do.

- **The rest of the v2.6.1 engine deltas that reach this crate.** `.csv.read`
  also accepts Rayfall's dotted temporal spellings (`2024.01.02`,
  `2024.01.02D01:02:03`) alongside the ISO forms the CSV writer emits, so a file
  written by `dump` round-trips. `if` with a null branch no longer writes an
  ordinary huge number where a null belongs — a null atom stays null across
  widths, and an `F64` past the `int64` range narrows to the integer null rather
  than an undefined cast. A periodic timer that overruns its period re-arms at
  the next deadline instead of replaying every fire it missed, and a failing
  callback prints `timer <id>: error: <code>: <message>`. The engine binary now
  exits 1 when `-p` cannot bind, rather than running the script and exiting 0
  with no listener — relevant to `tests/ipc.rs`, which spawns one. `update
  where:` and `upsert` write in place on a *named* flat table; the builders here
  pass a table value rather than a quoted name, so they keep taking the copy
  path and are unaffected.

- **The v2.6.2 engine deltas that reach this crate.** A `Value::slice` of a
  vector holding nulls inherits its parent's `HAS_NULLS` hint; before, every
  gate that reads the bit took the window for null-free, so an aggregate over it
  folded the sentinel in as a value. `is_null_at` on a slice already asked the
  parent and is unchanged. `Table::save_splayed` derives the on-disk bit from
  the payload rather than trusting the in-memory header, so a column holding a
  sentinel reloads with its nulls. `nil?` (`Operation::NilQ`) is element-wise
  outside queries as it always was inside them: over a vector or a list it
  answers a `B8` vector, where it used to answer `false`. The parser rejects a
  symbol, keyword, name or number glued to a quote, a colon or another name
  character — `['a:1]` is a parse error rather than the two symbols `a` and `1`,
  and so is `0Na` — so Rayfall text handed to `eval` must separate its tokens.
  `Table::load_parted` no longer reads a calendar-impossible directory such as
  `2024.02.31` as a date partition, which it used to normalise silently into
  `2024.03.02`; a root holding one falls back to symbol partitions. `.log.write`
  is refused inside an auto-journaled IPC evaluation. The rest lives in the
  engine binary rather than this crate's surface: the per-connection transmit
  backlog is configurable (`.ipc.txlimit`, 256 MiB by default — the old fixed
  cap), `.mc.sub` no longer requires a filter argument, and a script or piped
  session stays alive until its pending timers are spent.

- **The v2.7.0 and v2.8.0 engine deltas that reach this crate.** A grouped
  `select` that matches no row keeps every aggregate column and every literal
  projection column, so an empty result has the same shape as a populated one
  instead of losing columns. Every sort path keeps equal keys in source order,
  and `pivot` emits its index rows and pivot columns in first-seen order.
  Grouped `var` / `stddev` are computed numerically stably, and `pearson_corr`
  answers null when either side has no variance rather than a division
  artefact. `count (distinct …)` accepts a symbol-vector `by:`. Integer atoms
  order as `i64` — the integer arms of range comparison no longer round-trip
  through `f64`. `Table::load_parted` orders integer partitions by value and
  rejects a directory name past `i64` as corrupt. The table key map that
  `upsert` builds is now kept on the table and maintained across in-place
  insert, non-key update and single-row upsert, so repeated keyed upserts stop
  rebuilding it; copying a table drops it, and `delete` — new in v2.7.0, in
  place on a named table like `update where:` — keeps it. `find`, `in` and dict
  `at` consult a hash index when one is attached, admit `STR` needles, and
  carry the index across `concat`. On the wire, `.ipc.open` takes an options
  dict with a `compress` threshold, loopback and UNIX-domain links are never
  compressed, `.ipc.on.close` fires for outbound connections too, and
  `.ipc.handle` reports the direction; `TcpClient` itself is unchanged, since
  `ray_ipc_connect` keeps its signature and the threshold variant is private to
  the core. The core's legacy IPC server API (`ray_ipc_server_t`,
  `ray_ipc_poll`) is gone; it was private and this crate never bound it. The
  rest lives in the engine binary: `RAYFORCE_HOME` as the fallback root for
  relative paths, datalog integer arithmetic checked to `0Nl` on overflow and
  division by zero, journal archives preserved when rolls share a timestamp,
  and a multicast framing failure that no longer drops subscribers.

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
