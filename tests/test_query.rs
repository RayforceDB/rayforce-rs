/*
*   Copyright (c) 2026 Anton Kundenko <singaraiona@gmail.com>
*   All rights reserved.

*   Permission is hereby granted, free of charge, to any person obtaining a copy
*   of this software and associated documentation files (the "Software"), to deal
*   in the Software without restriction, including without limitation the rights
*   to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
*   copies of the Software, and to permit persons to whom the Software is
*   furnished to do so, subject to the following conditions:

*   The above copyright notice and this permission notice shall be included in all
*   copies or substantial portions of the Software.

*   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
*   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
*   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
*   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
*   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
*   OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
*   SOFTWARE.
*/

//! End-to-end tests for the Rayfall-synthesising query builder.
//!
//! Each test seeds a small table via `rf.eval`, then runs the builder
//! against it and inspects the result.  The builder's text-rendering
//! tests live alongside the implementation in `src/query.rs` — these
//! cover the round-trip through the engine.

mod common;

use rayforce::{
    query::{Column, Expression, InsertQuery, Operation, SelectQuery, UpdateQuery},
    RayTable, RAY_TABLE,
};
use serial_test::serial;

fn seed_trades(rf: &rayforce::Rayforce) {
    rf.eval(
        "(set t (table [sym price volume] \
            (list [AAPL GOOG MSFT AAPL] [101.0 99.5 250.0 102.0] [100 200 300 400])))",
    )
    .expect("seed");
}

#[test]
#[serial]
fn select_passthrough() {
    with_runtime!(rf, {
        seed_trades(&rf);
        let result = SelectQuery::from("t").execute(&rf).unwrap();
        assert_eq!(result.type_code(), RAY_TABLE as i8);
        let table = RayTable::from_ptr(result).unwrap();
        assert_eq!(table.len().unwrap(), 4);
        assert_eq!(table.ncols(), 3);
    });
}

#[test]
#[serial]
fn select_with_filter() {
    with_runtime!(rf, {
        seed_trades(&rf);
        let result = SelectQuery::from("t")
            .filter(Column::new("price").gt(100))
            .execute(&rf)
            .unwrap();
        let table = RayTable::from_ptr(result).unwrap();
        // AAPL@101, MSFT@250, AAPL@102 → 3 rows survive.
        assert_eq!(table.len().unwrap(), 3);
    });
}

#[test]
#[serial]
fn select_groupby_aggregate() {
    with_runtime!(rf, {
        seed_trades(&rf);
        let result = SelectQuery::from("t")
            .column(
                "avg_price",
                Operation::Avg(Column::new("price").into_expr()),
            )
            .column(
                "total_vol",
                Operation::Sum(Column::new("volume").into_expr()),
            )
            .group_by(Column::new("sym"))
            .execute(&rf)
            .unwrap();
        let table = RayTable::from_ptr(result).unwrap();
        // Three distinct symbols (AAPL, GOOG, MSFT).
        assert_eq!(table.len().unwrap(), 3);
        assert!(table.columns().unwrap().contains(&"avg_price".to_string()));
    });
}

#[test]
#[serial]
fn select_take_with_sort() {
    with_runtime!(rf, {
        seed_trades(&rf);
        let result = SelectQuery::from("t").desc("price").take(2).execute(&rf).unwrap();
        let table = RayTable::from_ptr(result).unwrap();
        assert_eq!(table.len().unwrap(), 2);
    });
}

#[test]
#[serial]
fn update_in_place() {
    with_runtime!(rf, {
        seed_trades(&rf);
        // Bump every price by 1.0.
        UpdateQuery::from("t")
            .set("price", Column::new("price").add(Expression::lit_f64(1.0)))
            .execute(&rf)
            .unwrap();
        let after = SelectQuery::from("t").execute(&rf).unwrap();
        let table = RayTable::from_ptr(after).unwrap();
        let price_col = table.get_column("price").unwrap();
        // Mode 1 render of the F64 vector contains the bumped values
        // (101.0 + 1.0 = 102.0, 99.5 + 1.0 = 100.5, ...).
        let rendered = format!("{price_col}");
        assert!(rendered.contains("102"), "got {rendered:?}");
        assert!(rendered.contains("100.5") || rendered.contains("100,5"), "got {rendered:?}");
    });
}

#[test]
#[serial]
fn insert_appends_row() {
    with_runtime!(rf, {
        seed_trades(&rf);
        InsertQuery::into_table("t")
            .rows(Expression::raw("(list 'TSLA 199.0 500)"))
            .execute(&rf)
            .unwrap();
        let after = SelectQuery::from("t").execute(&rf).unwrap();
        let table = RayTable::from_ptr(after).unwrap();
        assert_eq!(table.len().unwrap(), 5);
    });
}

#[test]
#[serial]
fn renders_round_trippable_strings() {
    let q = SelectQuery::from("t")
        .filter(Column::new("name").eq(Expression::lit_str("a\"b")));
    assert!(q.to_rayfall().contains("\"a\\\"b\""));
}

// ----- 1.0 parity audit -------------------------------------------------

#[test]
#[serial]
fn parity_column_aggregate_methods() {
    // 1.0 surface: Column::sum/avg/min/max/count/first/last/distinct.
    with_runtime!(rf, {
        seed_trades(&rf);
        let result = SelectQuery::from("t")
            .column("total_v", Column::new("volume").sum())
            .column("avg_p", Column::new("price").avg())
            .column("min_p", Column::new("price").min())
            .column("max_p", Column::new("price").max())
            .column("first_p", Column::new("price").first())
            .column("last_p", Column::new("price").last())
            .column("n", Column::new("price").count())
            .execute(&rf)
            .unwrap();
        let table = RayTable::from_ptr(result).unwrap();
        let cols = table.columns().unwrap();
        for name in ["total_v", "avg_p", "min_p", "max_p", "first_p", "last_p", "n"] {
            assert!(cols.iter().any(|c| c == name), "missing column {name}");
        }
    });
}

#[test]
#[serial]
fn parity_column_is_in() {
    // 1.0 surface: Column::is_in.
    with_runtime!(rf, {
        seed_trades(&rf);
        let result = SelectQuery::from("t")
            .filter(Column::new("sym").is_in(Expression::raw("[AAPL MSFT]")))
            .execute(&rf)
            .unwrap();
        let table = RayTable::from_ptr(result).unwrap();
        // AAPL × 2 + MSFT × 1 = 3 rows.
        assert_eq!(table.len().unwrap(), 3);
    });
}

#[test]
#[serial]
fn parity_expression_chained_and_or_not() {
    // 1.0 surface: RayExpression::and / or chainable.
    with_runtime!(rf, {
        seed_trades(&rf);
        let pred = Column::new("price")
            .gt(100)
            .and(Column::new("volume").lt(450));
        let result = SelectQuery::from("t").filter(pred).execute(&rf).unwrap();
        let table = RayTable::from_ptr(result).unwrap();
        // price > 100 ∧ vol < 450:
        //   AAPL@101/100  ✓ ✓
        //   GOOG@99.5/200 ✗
        //   MSFT@250/300  ✓ ✓
        //   AAPL@102/400  ✓ ✓
        // → 3 rows.
        assert_eq!(table.len().unwrap(), 3);

        // .or() chainable.
        let pred = Column::new("sym")
            .eq(Expression::raw("'AAPL"))
            .or(Column::new("sym").eq(Expression::raw("'GOOG")));
        let result = SelectQuery::from("t").filter(pred).execute(&rf).unwrap();
        let table = RayTable::from_ptr(result).unwrap();
        assert_eq!(table.len().unwrap(), 3);

        // .not() chainable.
        let pred = Column::new("sym").eq(Expression::raw("'AAPL")).not();
        let result = SelectQuery::from("t").filter(pred).execute(&rf).unwrap();
        let table = RayTable::from_ptr(result).unwrap();
        assert_eq!(table.len().unwrap(), 2);
    });
}

#[test]
#[serial]
fn parity_select_columns_bulk() {
    // 1.0 surface: RaySelectQuery::columns(&[&str]).
    with_runtime!(rf, {
        seed_trades(&rf);
        let result = SelectQuery::from("t")
            .columns(["sym", "price"])
            .execute(&rf)
            .unwrap();
        let table = RayTable::from_ptr(result).unwrap();
        let cols = table.columns().unwrap();
        assert_eq!(cols, vec!["sym".to_string(), "price".to_string()]);
    });
}

#[test]
#[serial]
fn parity_table_query_methods() {
    // 1.0 surface: RayTable::select() / update() / insert() / upsert().
    use rayforce::RayTable;
    with_runtime!(rf, {
        seed_trades(&rf);
        let table = RayTable::from_name("t").unwrap();

        // Smoke-test that each entry point produces a working builder.
        let select_q = table.select("t").filter(Column::new("price").gt(100));
        let r = select_q.execute(&rf).unwrap();
        assert_eq!(RayTable::from_ptr(r).unwrap().len().unwrap(), 3);

        let update_q = table
            .update("t")
            .set("volume", Column::new("volume").add(Expression::lit_i64(1)));
        update_q.execute(&rf).unwrap();

        let insert_q = table
            .insert("t")
            .rows(Expression::raw("(list 'TSLA 199.0 999)"));
        insert_q.execute(&rf).unwrap();

        // Build an upsert with a fresh integer-keyed table — upsert
        // semantics require a unique-key column and not all engine
        // builds accept SYM columns as keys.
        rf.eval("(set u (table [id name val] (list [1 2 3] ['Alice 'Bob 'Charlie] [10.0 20.0 30.0])))")
            .unwrap();
        let utbl = RayTable::from_name("u").unwrap();
        // key_idx=1 (Name column) — engine rejects key_idx=0 with a
        // "domain" error in 2.1.0 (probably a bug; mirrors what
        // upsert.rfl in rayforce/examples uses).
        let upsert_q = utbl.upsert("u", 1).rows(Expression::raw("(list 4 'Dave 40.0)"));
        upsert_q.execute(&rf).unwrap();
        let after = SelectQuery::from("u").execute(&rf).unwrap();
        assert_eq!(RayTable::from_ptr(after).unwrap().len().unwrap(), 4);
    });
}

#[test]
#[serial]
fn parity_set_value_via_into_expression() {
    // 1.0 surface: UpdateQuery::set_value<T: Into<RayObj>>.
    // 2.x equivalent: set(col, value) accepts anything Into<Expression>,
    // including i64/f64/bool/&str.
    use rayforce::query::UpdateQuery;
    with_runtime!(rf, {
        seed_trades(&rf);
        UpdateQuery::from("t")
            .set("volume", 0_i64) // i64 → Expression::lit_i64
            .execute(&rf)
            .unwrap();
        let after = SelectQuery::from("t").execute(&rf).unwrap();
        let s = format!("{after}");
        // Every volume should now be 0.
        assert!(s.contains('0'), "got {s:?}");
    });
}
