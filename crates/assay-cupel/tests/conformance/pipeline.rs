use std::collections::HashMap;

use assay_cupel::{ContextBudget, OverflowStrategy, Pipeline};

use super::{assert_ordered_eq, build_items, build_placer_by_type, build_scorer_by_type, build_slicer_by_type, load_vector};

fn build_pipeline_from_config(vector: &toml::Value) -> Pipeline {
    let config = vector.get("config").expect("missing [config]");

    // Build scorer(s)
    let scorers_arr = config
        .get("scorers")
        .and_then(|v| v.as_array())
        .expect("missing config.scorers");

    let scorer: Box<dyn assay_cupel::Scorer> = if scorers_arr.len() == 1 {
        let entry = &scorers_arr[0];
        let scorer_type = entry["type"].as_str().expect("scorer missing type");
        build_scorer_by_type(scorer_type, None)
    } else {
        let entries: Vec<(Box<dyn assay_cupel::Scorer>, f64)> = scorers_arr
            .iter()
            .map(|entry| {
                let scorer_type = entry["type"].as_str().expect("scorer missing type");
                let weight = entry["weight"]
                    .as_float()
                    .or_else(|| entry["weight"].as_integer().map(|i| i as f64))
                    .expect("scorer missing weight");
                (build_scorer_by_type(scorer_type, None), weight)
            })
            .collect();
        Box::new(assay_cupel::CompositeScorer::new(entries).unwrap())
    };

    // Build slicer
    let slicer_type = config["slicer"].as_str().expect("missing config.slicer");
    let slicer = build_slicer_by_type(slicer_type, Some(config));

    // Build placer
    let placer_type = config["placer"].as_str().expect("missing config.placer");
    let placer = build_placer_by_type(placer_type);

    // Deduplication
    let deduplication = config
        .get("deduplication")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    // Overflow strategy
    let overflow_strategy = config
        .get("overflow_strategy")
        .and_then(|v| v.as_str())
        .map(|s| match s {
            "throw" => OverflowStrategy::Throw,
            "truncate" => OverflowStrategy::Truncate,
            "proceed" => OverflowStrategy::Proceed,
            other => panic!("unknown overflow strategy: {other}"),
        })
        .unwrap_or_default();

    Pipeline::builder()
        .scorer(scorer)
        .slicer(slicer)
        .placer(placer)
        .deduplication(deduplication)
        .overflow_strategy(overflow_strategy)
        .build()
        .expect("failed to build pipeline")
}

fn run_pipeline_test(vector_path: &str) {
    let vector = load_vector(vector_path);
    let items = build_items(&vector);
    let pipeline = build_pipeline_from_config(&vector);

    let budget_table = vector.get("budget").expect("missing [budget]");
    let max_tokens = budget_table["max_tokens"]
        .as_integer()
        .expect("missing budget.max_tokens");
    let target_tokens = budget_table["target_tokens"]
        .as_integer()
        .expect("missing budget.target_tokens");
    let output_reserve = budget_table
        .get("output_reserve")
        .and_then(|v| v.as_integer())
        .unwrap_or(0);

    let budget =
        ContextBudget::new(max_tokens, target_tokens, output_reserve, HashMap::new(), 0.0)
            .expect("budget should be valid");

    let result = pipeline
        .run(&items, &budget)
        .expect("pipeline run should succeed");

    let actual_contents: Vec<String> = result.iter().map(|i| i.content().to_owned()).collect();

    let expected_output = vector
        .get("expected_output")
        .and_then(|v| v.as_array())
        .expect("missing [[expected_output]]");

    let expected_contents: Vec<String> = expected_output
        .iter()
        .map(|v| {
            v.get("content")
                .and_then(|c| c.as_str())
                .expect("expected_output missing content")
                .to_owned()
        })
        .collect();

    assert_ordered_eq(&expected_contents, &actual_contents);
}

#[test]
fn greedy_chronological() {
    run_pipeline_test("pipeline/greedy-chronological.toml");
}

#[test]
fn greedy_ushaped() {
    run_pipeline_test("pipeline/greedy-ushaped.toml");
}

#[test]
fn knapsack_chronological() {
    run_pipeline_test("pipeline/knapsack-chronological.toml");
}

#[test]
fn composite_greedy_chronological() {
    run_pipeline_test("pipeline/composite-greedy-chronological.toml");
}

#[test]
fn pinned_items() {
    run_pipeline_test("pipeline/pinned-items.toml");
}
