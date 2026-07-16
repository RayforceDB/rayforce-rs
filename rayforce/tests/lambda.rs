//! Lambda functions (`Fn`) — direct calls, query application, introspection.

use rayforce::{col, sum, Fn, Runtime, Table, Value};

#[test]
fn direct_call_scalar() {
    let _rt = Runtime::new().unwrap();
    let square = Fn::new("(fn [x] (* x x))").unwrap();
    assert_eq!(square.call(&[Value::i64(5)]).unwrap().as_i64().unwrap(), 25);
    assert_eq!(
        square.call(&[Value::i64(10)]).unwrap().as_i64().unwrap(),
        100
    );
}

#[test]
fn direct_call_multiple_args() {
    let _rt = Runtime::new().unwrap();
    let add = Fn::new("(fn [x y] (+ x y))").unwrap();
    assert_eq!(
        add.call(&[Value::i64(5), Value::i64(3)])
            .unwrap()
            .as_i64()
            .unwrap(),
        8
    );
}

#[test]
fn call_over_vector() {
    let _rt = Runtime::new().unwrap();
    let square = Fn::new("(fn [x] (* x x))").unwrap();
    let r = square.call(&[Value::vec(&[2i64, 3, 4])]).unwrap();
    assert_eq!(r.as_slice::<i64>().unwrap(), &[4, 9, 16]);
}

#[test]
fn rejects_non_fn_source() {
    let _rt = Runtime::new().unwrap();
    assert!(Fn::new("(+ 1 2)").is_err());
}

#[test]
fn apply_in_select() {
    let _rt = Runtime::new().unwrap();
    let table = Table::new(
        &["id", "value"],
        &[Value::sym_vec(&["a", "b", "c"]), Value::vec(&[2i64, 3, 4])],
    )
    .unwrap();

    let square = Fn::new("(fn [x] (* x x))").unwrap();
    let out = table
        .select()
        .col("id")
        .agg("squared", square.apply([col("value")]).unwrap())
        .execute()
        .unwrap();

    let squared = out.column("squared").unwrap();
    assert_eq!(squared.as_slice::<i64>().unwrap(), &[4, 9, 16]);
}

#[test]
fn apply_with_aggregation() {
    let _rt = Runtime::new().unwrap();
    let table = Table::new(
        &["id", "value"],
        &[
            Value::sym_vec(&["a", "b", "c", "d"]),
            Value::vec(&[2i64, 3, 4, 5]),
        ],
    )
    .unwrap();

    let square = Fn::new("(fn [x] (* x x))").unwrap();
    let out = table
        .select()
        .agg("sum_sq", sum(square.apply([col("value")]).unwrap()))
        .execute()
        .unwrap();

    // 4 + 9 + 16 + 25 = 54
    assert_eq!(
        out.column("sum_sq")
            .unwrap()
            .get(0)
            .unwrap()
            .as_i64()
            .unwrap(),
        54
    );
}

#[test]
fn introspection() {
    let _rt = Runtime::new().unwrap();
    let square = Fn::new("(fn [x] (* x x))").unwrap();
    assert_eq!(square.source(), Some("(fn [x] (* x x))"));
    assert_eq!(format!("{square}"), "(fn [x] (* x x))");
    // `meta` reflects whatever the engine reports for the lambda.
    assert!(square.meta().is_ok());
}
