use super::{assert_scores_match, build_items, build_scorer, load_vector};

fn run_scoring_test(vector_path: &str) {
    let vector = load_vector(vector_path);
    let items = build_items(&vector);
    let scorer = build_scorer(&vector);

    let actual_scores: Vec<(String, f64)> = items
        .iter()
        .map(|item| {
            let score = scorer.score(item, &items);
            (item.content().to_owned(), score)
        })
        .collect();

    assert_scores_match(&vector, &actual_scores);
}

#[test]
fn recency_basic() {
    run_scoring_test("scoring/recency-basic.toml");
}

#[test]
fn recency_null_timestamps() {
    run_scoring_test("scoring/recency-null-timestamps.toml");
}

#[test]
fn priority_basic() {
    run_scoring_test("scoring/priority-basic.toml");
}

#[test]
fn priority_null() {
    run_scoring_test("scoring/priority-null.toml");
}

#[test]
fn kind_default_weights() {
    run_scoring_test("scoring/kind-default-weights.toml");
}

#[test]
fn kind_unknown() {
    run_scoring_test("scoring/kind-unknown.toml");
}

#[test]
fn tag_basic() {
    run_scoring_test("scoring/tag-basic.toml");
}

#[test]
fn tag_no_tags() {
    run_scoring_test("scoring/tag-no-tags.toml");
}

#[test]
fn frequency_basic() {
    run_scoring_test("scoring/frequency-basic.toml");
}

#[test]
fn reflexive_basic() {
    run_scoring_test("scoring/reflexive-basic.toml");
}

#[test]
fn reflexive_null() {
    run_scoring_test("scoring/reflexive-null.toml");
}

#[test]
fn composite_weighted() {
    run_scoring_test("scoring/composite-weighted.toml");
}

#[test]
fn scaled_basic() {
    run_scoring_test("scoring/scaled-basic.toml");
}
