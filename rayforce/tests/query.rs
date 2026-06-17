//! Phase 6: query builders — select / where / by / order, update, insert,
//! upsert, joins, and table conveniences.

use rayforce::{col, sum, Runtime, Table, Value};

fn trades() -> Table {
    // 5 rows
    let sym = Value::sym_vec(&["AAPL", "MSFT", "AAPL", "GOOG", "MSFT"]);
    let price = Value::vec(&[100.0f64, 200.0, 110.0, 300.0, 210.0]);
    let size = Value::vec(&[10i64, 20, 30, 40, 50]);
    Table::new(&["sym", "price", "size"], &[sym, price, size]).unwrap()
}

#[test]
fn select_columns() {
    let _rt = Runtime::new().unwrap();
    let t = trades();
    let r = t.select().cols(["sym", "price"]).execute().unwrap();
    assert_eq!(r.ncols(), 2);
    assert_eq!(r.nrows(), 5);
    assert_eq!(r.column_names(), vec!["sym", "price"]);
}

#[test]
fn select_where() {
    let _rt = Runtime::new().unwrap();
    let t = trades();
    let r = t
        .select()
        .col("price")
        .filter(col("price").gt(150.0))
        .execute()
        .unwrap();
    // prices > 150 -> 200, 300, 210
    assert_eq!(r.nrows(), 3);
    let prices = r
        .column("price")
        .unwrap()
        .as_slice::<f64>()
        .unwrap()
        .to_vec();
    assert_eq!(prices, vec![200.0, 300.0, 210.0]);
}

#[test]
fn select_where_combined() {
    let _rt = Runtime::new().unwrap();
    let t = trades();
    // price > 150 AND sym == MSFT -> rows with 200, 210
    let r = t
        .select()
        .filter(col("price").gt(150.0))
        .filter(col("sym").eq("MSFT"))
        .execute()
        .unwrap();
    assert_eq!(r.nrows(), 2);
}

#[test]
fn select_aggregate_by() {
    let _rt = Runtime::new().unwrap();
    let t = trades();
    // total size by sym: AAPL=40, MSFT=70, GOOG=40
    let r = t
        .select()
        .agg("total", sum(col("size")))
        .by("sym")
        .execute()
        .unwrap();
    assert_eq!(r.nrows(), 3);
    assert!(r.column_names().contains(&"total".to_string()));
}

#[test]
fn select_aggregate_only_collapses_to_one_row() {
    let _rt = Runtime::new().unwrap();
    let t = trades();
    let r = t.select().agg("total", sum(col("size"))).execute().unwrap();
    assert_eq!(r.nrows(), 1);
    assert_eq!(
        r.column("total").unwrap().get(0).unwrap().as_i64().unwrap(),
        150
    );
}

#[test]
fn select_order_by() {
    let _rt = Runtime::new().unwrap();
    let t = trades();
    let r = t
        .select()
        .col("price")
        .order_by(["price"], true) // descending
        .execute()
        .unwrap();
    let prices = r
        .column("price")
        .unwrap()
        .as_slice::<f64>()
        .unwrap()
        .to_vec();
    assert_eq!(prices, vec![300.0, 210.0, 200.0, 110.0, 100.0]);
}

#[test]
fn update_adds_column() {
    let _rt = Runtime::new().unwrap();
    let t = trades();
    // notional = price * size
    let r = t
        .update()
        .set("notional", col("price") * col("size"))
        .execute()
        .unwrap();
    assert!(r.column_names().contains(&"notional".to_string()));
    let n0 = r
        .column("notional")
        .unwrap()
        .get(0)
        .unwrap()
        .as_f64()
        .unwrap();
    assert_eq!(n0, 1000.0);
}

#[test]
fn insert_row() {
    let _rt = Runtime::new().unwrap();
    let t = trades();
    let r = t
        .insert_row(&[Value::sym("TSLA"), Value::f64(250.0), Value::i64(60)])
        .unwrap();
    assert_eq!(r.nrows(), 6);
    assert_eq!(
        r.column("sym").unwrap().get(5).unwrap().as_sym().unwrap(),
        "TSLA"
    );
}

#[test]
fn join_tables() {
    let _rt = Runtime::new().unwrap();
    let t = trades();
    // reference table: sym -> sector
    let sym = Value::sym_vec(&["AAPL", "MSFT", "GOOG"]);
    let sector = Value::sym_vec(&["Tech", "Tech", "Search"]);
    let ref_tbl = Table::new(&["sym", "sector"], &[sym, sector]).unwrap();

    let joined = t.inner_join(&ref_tbl, &["sym"]).unwrap();
    assert!(joined.column_names().contains(&"sector".to_string()));
    assert_eq!(joined.nrows(), 5);
}

#[test]
fn head_tail_take() {
    let _rt = Runtime::new().unwrap();
    let t = trades();
    assert_eq!(t.head(2).unwrap().nrows(), 2);
    assert_eq!(t.tail(3).unwrap().nrows(), 3);
    // head keeps first rows
    let h = t.head(1).unwrap();
    assert_eq!(
        h.column("size").unwrap().get(0).unwrap().as_i64().unwrap(),
        10
    );
    // tail keeps last rows
    let tl = t.tail(1).unwrap();
    assert_eq!(
        tl.column("size").unwrap().get(0).unwrap().as_i64().unwrap(),
        50
    );
}
