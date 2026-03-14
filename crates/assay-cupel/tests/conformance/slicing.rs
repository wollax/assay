use std::collections::HashMap;

use assay_cupel::ContextBudget;

use super::{assert_set_eq, build_scored_items, build_slicer, load_vector};

fn run_slicing_test(vector_path: &str) {
    let vector = load_vector(vector_path);
    let scored_items = build_scored_items(&vector);
    let slicer = build_slicer(&vector);

    let target_tokens = vector["budget"]["target_tokens"]
        .as_integer()
        .expect("missing budget.target_tokens");

    let budget = ContextBudget::new(target_tokens, target_tokens, 0, HashMap::new(), 0.0)
        .expect("budget should be valid");

    let selected = slicer.slice(&scored_items, &budget);
    let actual_contents: Vec<String> = selected.iter().map(|i| i.content().to_owned()).collect();

    let expected_contents: Vec<String> = vector["expected"]["selected_contents"]
        .as_array()
        .expect("missing expected.selected_contents")
        .iter()
        .map(|v| v.as_str().expect("expected content must be string").to_owned())
        .collect();

    assert_set_eq(&expected_contents, &actual_contents);
}

#[test]
fn greedy_density() {
    run_slicing_test("slicing/greedy-density.toml");
}

#[test]
fn greedy_exact_fit() {
    run_slicing_test("slicing/greedy-exact-fit.toml");
}

#[test]
fn greedy_zero_tokens() {
    run_slicing_test("slicing/greedy-zero-tokens.toml");
}

#[test]
fn knapsack_basic() {
    run_slicing_test("slicing/knapsack-basic.toml");
}

#[test]
fn knapsack_zero_tokens() {
    run_slicing_test("slicing/knapsack-zero-tokens.toml");
}

#[test]
fn quota_basic() {
    run_slicing_test("slicing/quota-basic.toml");
}
