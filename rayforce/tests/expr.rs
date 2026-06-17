//! Phase 5: expression AST — compile, evaluate, operators, aggregations.

use rayforce::{col, lit, sum, Expr, Operation, Runtime, Value};

#[test]
fn literal_arithmetic() {
    let _rt = Runtime::new().unwrap();
    // (+ [1 2 3] 10) -> [11 12 13]
    let e = lit(Value::vec(&[1i64, 2, 3])) + 10i64;
    let r = e.execute().unwrap();
    assert_eq!(r.as_slice::<i64>().unwrap(), &[11, 12, 13]);
}

#[test]
fn chained_arithmetic() {
    let _rt = Runtime::new().unwrap();
    // (* (- v 1) 2) over [1 2 3] -> [0 2 4]
    let v = lit(Value::vec(&[1i64, 2, 3]));
    let e = (v - 1i64) * 2i64;
    assert_eq!(e.execute().unwrap().as_slice::<i64>().unwrap(), &[0, 2, 4]);
}

#[test]
fn aggregation_free_and_method() {
    let _rt = Runtime::new().unwrap();
    let data = Value::vec(&[1i64, 2, 3, 4]);
    // free function form
    assert_eq!(
        sum(lit(data.clone())).execute().unwrap().as_i64().unwrap(),
        10
    );
    // method form
    assert_eq!(lit(data).sum().execute().unwrap().as_i64().unwrap(), 10);
}

#[test]
fn comparison_produces_mask() {
    let _rt = Runtime::new().unwrap();
    // (> [1 2 3 4] 2) -> [0 0 1 1] (bool vector)
    let e = lit(Value::vec(&[1i64, 2, 3, 4])).gt(2i64);
    let r = e.execute().unwrap();
    assert_eq!(r.len(), 4);
    // formatting check (engine bool vec)
    assert!(!r.get(0).unwrap().as_bool().unwrap());
    assert!(r.get(3).unwrap().as_bool().unwrap());
}

#[test]
fn column_reference_resolves_from_global() {
    let _rt = Runtime::new().unwrap();
    // Bind a global vector `xs`, then reference it by name in an expression.
    let xs = Value::vec(&[10i64, 20, 30]);
    rayforce::set_global("xs", &xs).unwrap();

    let e = col("xs") + 5i64;
    assert_eq!(
        e.execute().unwrap().as_slice::<i64>().unwrap(),
        &[15, 25, 35]
    );

    assert_eq!(sum(col("xs")).execute().unwrap().as_i64().unwrap(), 60);
}

#[test]
fn logical_combination() {
    let _rt = Runtime::new().unwrap();
    rayforce::set_global("v", &Value::vec(&[1i64, 5, 10, 15])).unwrap();
    // (and (> v 2) (< v 12)) -> [0 1 1 0]
    let e = col("v").gt(2i64).and(col("v").lt(12i64));
    let r = e.execute().unwrap();
    let bits: Vec<bool> = (0..4)
        .map(|i| r.get(i).unwrap().as_bool().unwrap())
        .collect();
    assert_eq!(bits, vec![false, true, true, false]);
}

#[test]
fn operator_overload_bitand() {
    let _rt = Runtime::new().unwrap();
    rayforce::set_global("w", &Value::vec(&[1i64, 5, 10, 15])).unwrap();
    // same as logical_combination but using `&`
    let e = col("w").gt(2i64) & col("w").lt(12i64);
    let r = e.execute().unwrap();
    assert!(!r.get(0).unwrap().as_bool().unwrap());
    assert!(r.get(1).unwrap().as_bool().unwrap());
}

#[test]
fn compile_shape_is_a_list() {
    let _rt = Runtime::new().unwrap();
    let e = col("price").gt(100i64);
    let compiled = e.compile();
    // (> price 100) — a 3-element list: op, name-ref, literal
    assert!(compiled.is_list());
    assert_eq!(compiled.len(), 3);
}

#[test]
fn operation_names() {
    assert_eq!(Operation::Add.as_str(), "+");
    assert_eq!(Operation::Sum.as_str(), "sum");
    assert_eq!(Operation::InnerJoin.as_str(), "inner-join");
    assert_eq!(Operation::Median.as_str(), "med");
}

#[test]
fn sum_of_filter() {
    let _rt = Runtime::new().unwrap();
    rayforce::set_global("c", &Value::vec(&[10i64, 20, 30, 40])).unwrap();
    rayforce::set_global("m", &Value::bool_vec(&[true, false, true, false])).unwrap();
    // sum(filter(c, m)) keeps [10, 30] -> 40
    let e = Expr::op(Operation::Sum, vec![col("c").filter(col("m"))]);
    assert_eq!(e.execute().unwrap().as_i64().unwrap(), 40);
}
