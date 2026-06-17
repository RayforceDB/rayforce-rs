# :material-history: Changelog

All notable changes to `rayforce` are documented here. This project adheres to
[Semantic Versioning](https://semver.org).

## 0.1.0

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
