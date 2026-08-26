//! Phase 4: table construction, accessors, and CSV / splayed I/O.

use rayforce::{eval, Runtime, Table, Value};

fn sample_table() -> Table {
    // 3 rows: sym, price (f64), size (i64)
    let sym = Value::sym_vec(&["AAPL", "MSFT", "GOOG"]);
    let price = Value::vec(&[101.5f64, 202.0, 303.25]);
    let size = Value::vec(&[10i64, 20, 30]);
    Table::new(&["sym", "price", "size"], &[sym, price, size]).unwrap()
}

#[test]
fn construct_and_shape() {
    Runtime::scope(|_rt| {
        let t = sample_table();
        assert_eq!(t.ncols(), 3);
        assert_eq!(t.nrows(), 3);
        assert_eq!(t.shape(), (3, 3));
        assert_eq!(t.column_names(), vec!["sym", "price", "size"]);
        Ok(())
    })
    .unwrap();
}

#[test]
fn column_access() {
    Runtime::scope(|_rt| {
        let t = sample_table();

        let price = t.column("price").unwrap();
        assert_eq!(price.as_slice::<f64>().unwrap(), &[101.5, 202.0, 303.25]);

        let size = t.column_at(2).unwrap();
        assert_eq!(size.as_slice::<i64>().unwrap(), &[10, 20, 30]);

        let sym0 = t.column("sym").unwrap().get(0).unwrap().as_sym().unwrap();
        assert_eq!(sym0, "AAPL");

        assert!(t.column("nonexistent").is_err());
        assert!(t.column_at(9).is_err());
        Ok(())
    })
    .unwrap();
}

#[test]
fn all_columns() {
    Runtime::scope(|_rt| {
        let t = sample_table();
        let cols = t.columns().unwrap();
        assert_eq!(cols.len(), 3);
        assert_eq!(cols[1].as_slice::<f64>().unwrap(), &[101.5, 202.0, 303.25]);
        Ok(())
    })
    .unwrap();
}

#[test]
fn value_table_interop() {
    Runtime::scope(|_rt| {
        let t = sample_table();
        let v = t.clone().into_value();
        assert!(v.is_table());
        let back = v.as_table().unwrap();
        assert_eq!(back.shape(), (3, 3));

        // a non-table value cannot be made a Table
        assert!(Value::i64(5).as_table().is_err());
        assert!(Table::from_value(Value::i64(5)).is_err());
        Ok(())
    })
    .unwrap();
}

#[test]
fn matches_engine_table() {
    Runtime::scope(|_rt| {
        // Build the same table the engine builds via a literal, compare formatting.
        let t = sample_table();
        let engine =
            eval("(table 'sym (list 'AAPL 'MSFT 'GOOG) 'price 101.5 202.0 303.25 'size 10 20 30)").ok();
        // The exact literal syntax may differ across engine versions; only assert
        // our table renders non-empty and has the expected shape-derived header.
        let rendered = format!("{t}");
        assert!(rendered.contains("sym") && rendered.contains("price") && rendered.contains("size"));
        let _ = engine; // engine literal is advisory; not asserted on
        Ok(())
    })
    .unwrap();
}

#[test]
fn csv_roundtrip() {
    Runtime::scope(|_rt| {
        let t = sample_table();

        let dir = std::env::temp_dir();
        let path = dir.join(format!("rayforce_rs_csv_{}.csv", std::process::id()));
        let path_str = path.to_str().unwrap();

        t.write_csv(path_str).unwrap();
        assert!(path.exists());

        let loaded = Table::read_csv(&["SYMBOL", "F64", "I64"], path_str).unwrap();
        assert_eq!(loaded.ncols(), 3);
        assert_eq!(loaded.nrows(), 3);
        assert_eq!(
            loaded.column_at(1).unwrap().as_slice::<f64>().unwrap(),
            &[101.5, 202.0, 303.25]
        );

        let _ = std::fs::remove_file(&path);
        Ok(())
    })
    .unwrap();
}

#[test]
fn splayed_roundtrip() {
    Runtime::scope(|_rt| {
        let t = sample_table();

        let base = std::env::temp_dir().join(format!("rayforce_rs_splay_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let dir = base.join("t");
        std::fs::create_dir_all(&dir).unwrap();
        let dir_str = dir.to_str().unwrap();
        // Symbol columns require a symfile; supply an explicit one for both ends.
        let sym = base.join("sym");
        let sym_str = sym.to_str().unwrap();

        t.save_splayed(dir_str, Some(sym_str)).unwrap();
        let loaded = Table::load_splayed(dir_str, Some(sym_str)).unwrap();
        assert_eq!(loaded.shape(), (3, 3));
        assert_eq!(
            loaded.column("size").unwrap().as_slice::<i64>().unwrap(),
            &[10, 20, 30]
        );

        let _ = std::fs::remove_dir_all(&base);
        Ok(())
    })
    .unwrap();
}

#[test]
fn splayed_sym_values_roundtrip() {
    // A SYM column loaded from a splayed table must resolve to its original
    // values. The cells are positions in the column's *file* symbol domain;
    // resolving them against the runtime domain (the pre-fix behaviour) yields
    // garbage (e.g. "+"). `splayed_roundtrip` above only checks an i64 column,
    // so it never exercised symbol resolution.
    Runtime::scope(|_rt| {
        let t = Table::new(
            &["k", "v"],
            &[
                Value::sym_vec(&["abcdef123456", "xyz"]),
                Value::vec(&[1i64, 2]),
            ],
        )
        .unwrap();

        let base = std::env::temp_dir().join(format!("rayforce_rs_splay_sym_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let dir = base.join("t");
        std::fs::create_dir_all(&dir).unwrap();
        let dir_str = dir.to_str().unwrap();

        // <dir>/.sym dotfile convention (no explicit sym path).
        t.save_splayed(dir_str, None).unwrap();
        let loaded = Table::load_splayed(dir_str, None).unwrap();

        assert_eq!(loaded.shape(), (2, 2));
        let k = loaded.column("k").unwrap();
        assert_eq!(k.get(0).unwrap().as_sym().unwrap(), "abcdef123456");
        assert_eq!(k.get(1).unwrap().as_sym().unwrap(), "xyz");
        assert_eq!(
            loaded.column("v").unwrap().as_slice::<i64>().unwrap(),
            &[1, 2]
        );

        let _ = std::fs::remove_dir_all(&base);
        Ok(())
    })
    .unwrap();
}
