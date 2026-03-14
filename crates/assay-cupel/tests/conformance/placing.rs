use assay_cupel::{ContextItemBuilder, ScoredItem};
use chrono::{DateTime, Utc};

use super::{assert_ordered_eq, build_placer, load_vector};

fn build_placing_items(vector: &toml::Value) -> Vec<ScoredItem> {
    let items_array = vector
        .get("items")
        .and_then(|v| v.as_array())
        .expect("missing [[items]] array");

    items_array
        .iter()
        .map(|item| {
            let content = item["content"].as_str().expect("item missing content");
            let tokens = item["tokens"].as_integer().expect("item missing tokens");
            let score = item["score"]
                .as_float()
                .or_else(|| item["score"].as_integer().map(|i| i as f64))
                .expect("item missing score");

            let mut builder = ContextItemBuilder::new(content, tokens);

            if let Some(ts) = item.get("timestamp") {
                let dt = match ts {
                    toml::Value::Datetime(dt) => {
                        let s = dt.to_string();
                        s.parse::<DateTime<Utc>>()
                            .unwrap_or_else(|e| panic!("failed to parse datetime '{s}': {e}"))
                    }
                    other => panic!("expected TOML datetime, got: {other:?}"),
                };
                builder = builder.timestamp(dt);
            }

            ScoredItem {
                item: builder.build().expect("failed to build ContextItem"),
                score,
            }
        })
        .collect()
}

fn run_placing_test(vector_path: &str) {
    let vector = load_vector(vector_path);
    let scored_items = build_placing_items(&vector);
    let placer = build_placer(&vector);

    let placed = placer.place(&scored_items);
    let actual_contents: Vec<String> = placed.iter().map(|i| i.content().to_owned()).collect();

    let expected_contents: Vec<String> = vector["expected"]["ordered_contents"]
        .as_array()
        .expect("missing expected.ordered_contents")
        .iter()
        .map(|v| v.as_str().expect("expected content must be string").to_owned())
        .collect();

    assert_ordered_eq(&expected_contents, &actual_contents);
}

#[test]
fn chronological_basic() {
    run_placing_test("placing/chronological-basic.toml");
}

#[test]
fn chronological_null_timestamps() {
    run_placing_test("placing/chronological-null-timestamps.toml");
}

#[test]
fn u_shaped_basic() {
    run_placing_test("placing/u-shaped-basic.toml");
}

#[test]
fn u_shaped_equal_scores() {
    run_placing_test("placing/u-shaped-equal-scores.toml");
}
