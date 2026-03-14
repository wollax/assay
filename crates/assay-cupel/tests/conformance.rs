mod conformance {
    pub mod pipeline;
    pub mod placing;
    pub mod scoring;
    pub mod slicing;

    use std::collections::HashMap;
    use std::path::Path;

    use chrono::{DateTime, Utc};
    use toml::Value;

    use assay_cupel::{
        ChronologicalPlacer, CompositeScorer, ContextItem, ContextItemBuilder,
        ContextKind, FrequencyScorer, GreedySlice, KindScorer, KnapsackSlice, Placer,
        PriorityScorer, QuotaEntry, QuotaSlice, RecencyScorer, ReflexiveScorer, ScaledScorer,
        Scorer, ScoredItem, Slicer, TagScorer, UShapedPlacer,
    };

    /// Load a TOML test vector from a path relative to the conformance/required/ directory.
    pub fn load_vector(relative_path: &str) -> Value {
        let base = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("conformance")
            .join("required");
        let path = base.join(relative_path);
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        content
            .parse::<Value>()
            .unwrap_or_else(|e| panic!("failed to parse TOML {}: {e}", path.display()))
    }

    /// Parse a TOML datetime value to chrono::DateTime<Utc>.
    fn parse_toml_datetime(val: &Value) -> DateTime<Utc> {
        match val {
            Value::Datetime(dt) => {
                let s = dt.to_string();
                s.parse::<DateTime<Utc>>()
                    .unwrap_or_else(|e| panic!("failed to parse datetime '{s}': {e}"))
            }
            other => panic!("expected TOML datetime, got: {other:?}"),
        }
    }

    /// Build ContextItems from the [[items]] array in a test vector.
    pub fn build_items(vector: &Value) -> Vec<ContextItem> {
        let items_array = vector
            .get("items")
            .and_then(|v| v.as_array())
            .expect("missing [[items]] array");

        items_array
            .iter()
            .map(|item| {
                let content = item["content"].as_str().expect("item missing content");
                let tokens = item["tokens"].as_integer().expect("item missing tokens");

                let mut builder = ContextItemBuilder::new(content, tokens);

                if let Some(kind_val) = item.get("kind").and_then(|v| v.as_str()) {
                    builder = builder.kind(ContextKind::new(kind_val).unwrap());
                }

                if let Some(ts) = item.get("timestamp") {
                    builder = builder.timestamp(parse_toml_datetime(ts));
                }

                if let Some(p) = item.get("priority").and_then(|v| v.as_integer()) {
                    builder = builder.priority(p);
                }

                if let Some(tags) = item.get("tags").and_then(|v| v.as_array()) {
                    let tag_vec: Vec<String> = tags
                        .iter()
                        .map(|t| t.as_str().expect("tag must be string").to_owned())
                        .collect();
                    builder = builder.tags(tag_vec);
                }

                if let Some(hint) = item.get("futureRelevanceHint").and_then(|v| v.as_float()) {
                    builder = builder.future_relevance_hint(hint);
                }

                if let Some(pinned) = item.get("pinned").and_then(|v| v.as_bool()) {
                    builder = builder.pinned(pinned);
                }

                builder.build().expect("failed to build ContextItem")
            })
            .collect()
    }

    /// Build ScoredItems from the [[scored_items]] array in a test vector.
    pub fn build_scored_items(vector: &Value) -> Vec<ScoredItem> {
        let items_array = vector
            .get("scored_items")
            .and_then(|v| v.as_array())
            .expect("missing [[scored_items]] array");

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

                if let Some(kind_val) = item.get("kind").and_then(|v| v.as_str()) {
                    builder = builder.kind(ContextKind::new(kind_val).unwrap());
                }

                if let Some(ts) = item.get("timestamp") {
                    builder = builder.timestamp(parse_toml_datetime(ts));
                }

                ScoredItem {
                    item: builder.build().expect("failed to build ContextItem"),
                    score,
                }
            })
            .collect()
    }

    /// Build a Scorer from the config section of a scoring test vector.
    pub fn build_scorer(vector: &Value) -> Box<dyn Scorer> {
        let scorer_type = vector["test"]["scorer"]
            .as_str()
            .expect("missing test.scorer");
        build_scorer_by_type(scorer_type, vector.get("config"))
    }

    pub fn build_scorer_by_type(scorer_type: &str, config: Option<&Value>) -> Box<dyn Scorer> {
        match scorer_type {
            "recency" => Box::new(RecencyScorer),
            "priority" => Box::new(PriorityScorer),
            "frequency" => Box::new(FrequencyScorer),
            "reflexive" => Box::new(ReflexiveScorer),
            "kind" => {
                let cfg = match config {
                    Some(c) => c,
                    None => return Box::new(KindScorer::with_default_weights()),
                };
                if cfg.get("use_default_weights").and_then(|v| v.as_bool()) == Some(true)
                    || (!cfg.as_table().is_some_and(|t| t.contains_key("weights")))
                {
                    Box::new(KindScorer::with_default_weights())
                } else {
                    let weights_arr = cfg
                        .get("weights")
                        .and_then(|v| v.as_array())
                        .expect("kind scorer needs config.weights");
                    let mut weights = HashMap::new();
                    for entry in weights_arr {
                        let kind =
                            entry["kind"].as_str().expect("weight entry missing kind");
                        let weight = entry["weight"]
                            .as_float()
                            .or_else(|| entry["weight"].as_integer().map(|i| i as f64))
                            .expect("weight entry missing weight");
                        weights.insert(ContextKind::new(kind).unwrap(), weight);
                    }
                    Box::new(KindScorer::new(weights).unwrap())
                }
            }
            "tag" => {
                let cfg = config.expect("tag scorer needs config");
                let tw_arr = cfg
                    .get("tag_weights")
                    .and_then(|v| v.as_array())
                    .expect("tag scorer needs config.tag_weights");
                let mut tag_weights = HashMap::new();
                for entry in tw_arr {
                    let tag = entry["tag"].as_str().expect("tag_weight missing tag");
                    let weight = entry["weight"]
                        .as_float()
                        .or_else(|| entry["weight"].as_integer().map(|i| i as f64))
                        .expect("tag_weight missing weight");
                    tag_weights.insert(tag.to_owned(), weight);
                }
                Box::new(TagScorer::new(tag_weights).unwrap())
            }
            "composite" => {
                let cfg = config.expect("composite scorer needs config");
                let scorers_arr = cfg
                    .get("scorers")
                    .and_then(|v| v.as_array())
                    .expect("composite needs config.scorers");
                let entries: Vec<(Box<dyn Scorer>, f64)> = scorers_arr
                    .iter()
                    .map(|entry| {
                        let child_type =
                            entry["type"].as_str().expect("scorer entry missing type");
                        let weight = entry["weight"]
                            .as_float()
                            .or_else(|| entry["weight"].as_integer().map(|i| i as f64))
                            .expect("scorer entry missing weight");
                        (build_scorer_by_type(child_type, None), weight)
                    })
                    .collect();
                Box::new(CompositeScorer::new(entries).unwrap())
            }
            "scaled" => {
                let cfg = config.expect("scaled scorer needs config");
                let inner_type = cfg["inner_scorer"]
                    .as_str()
                    .expect("scaled needs config.inner_scorer");
                let inner = build_scorer_by_type(inner_type, None);
                Box::new(ScaledScorer::new(inner))
            }
            other => panic!("unknown scorer type: {other}"),
        }
    }

    /// Build a Slicer from the test vector config.
    pub fn build_slicer(vector: &Value) -> Box<dyn Slicer> {
        let slicer_type = vector["test"]["slicer"]
            .as_str()
            .expect("missing test.slicer");
        build_slicer_by_type(slicer_type, vector.get("config"))
    }

    pub fn build_slicer_by_type(slicer_type: &str, config: Option<&Value>) -> Box<dyn Slicer> {
        match slicer_type {
            "greedy" => Box::new(GreedySlice),
            "knapsack" => {
                let bucket_size = config
                    .and_then(|c| c.get("bucket_size"))
                    .and_then(|v| v.as_integer())
                    .unwrap_or(100);
                Box::new(KnapsackSlice::new(bucket_size).unwrap())
            }
            "quota" => {
                let cfg = config.expect("quota slicer needs config");
                let inner_type = cfg["inner_slicer"]
                    .as_str()
                    .expect("quota needs inner_slicer");
                let inner = build_slicer_by_type(inner_type, None);

                let quotas_arr = cfg
                    .get("quotas")
                    .and_then(|v| v.as_array())
                    .expect("quota needs config.quotas");

                let quotas: Vec<QuotaEntry> = quotas_arr
                    .iter()
                    .map(|q| {
                        let kind = q["kind"].as_str().expect("quota missing kind");
                        let require = q["require"]
                            .as_float()
                            .or_else(|| q["require"].as_integer().map(|i| i as f64))
                            .expect("quota missing require");
                        let cap = q["cap"]
                            .as_float()
                            .or_else(|| q["cap"].as_integer().map(|i| i as f64))
                            .expect("quota missing cap");
                        QuotaEntry::new(
                            ContextKind::new(kind).unwrap(),
                            require,
                            cap,
                        ).unwrap()
                    })
                    .collect();

                Box::new(QuotaSlice::new(quotas, inner).unwrap())
            }
            other => panic!("unknown slicer type: {other}"),
        }
    }

    /// Build a Placer from the test vector config.
    pub fn build_placer(vector: &Value) -> Box<dyn Placer> {
        let placer_type = vector["test"]["placer"]
            .as_str()
            .or_else(|| {
                vector
                    .get("config")
                    .and_then(|c| c.get("placer"))
                    .and_then(|v| v.as_str())
            })
            .expect("missing placer type");
        build_placer_by_type(placer_type)
    }

    pub fn build_placer_by_type(placer_type: &str) -> Box<dyn Placer> {
        match placer_type {
            "chronological" => Box::new(ChronologicalPlacer),
            "u-shaped" => Box::new(UShapedPlacer),
            other => panic!("unknown placer type: {other}"),
        }
    }

    /// Assert that actual scores match expected scores within epsilon tolerance.
    pub fn assert_scores_match(vector: &Value, actual_scores: &[(String, f64)]) {
        let epsilon = vector
            .get("tolerance")
            .and_then(|t| t.get("score_epsilon"))
            .and_then(|v| v.as_float())
            .unwrap_or(1e-9);

        let expected = vector
            .get("expected")
            .and_then(|v| v.as_array())
            .expect("missing [[expected]] array");

        assert_eq!(
            expected.len(),
            actual_scores.len(),
            "expected {} scores, got {}",
            expected.len(),
            actual_scores.len()
        );

        for exp in expected {
            let content = exp["content"].as_str().expect("expected missing content");
            let expected_score = exp["score_approx"]
                .as_float()
                .or_else(|| exp["score_approx"].as_integer().map(|i| i as f64))
                .expect("expected missing score_approx");

            let actual = actual_scores
                .iter()
                .find(|(c, _)| c == content)
                .unwrap_or_else(|| panic!("no score found for content '{content}'"));

            let diff = (actual.1 - expected_score).abs();
            assert!(
                diff < epsilon,
                "score mismatch for '{content}': expected {expected_score}, got {}, diff {diff} >= epsilon {epsilon}",
                actual.1,
            );
        }
    }

    /// Assert two sets are equal (order-independent).
    pub fn assert_set_eq(expected: &[String], actual: &[String]) {
        let mut exp_sorted: Vec<&str> = expected.iter().map(|s| s.as_str()).collect();
        let mut act_sorted: Vec<&str> = actual.iter().map(|s| s.as_str()).collect();
        exp_sorted.sort();
        act_sorted.sort();
        assert_eq!(
            exp_sorted, act_sorted,
            "set mismatch:\n  expected: {exp_sorted:?}\n  actual:   {act_sorted:?}"
        );
    }

    /// Assert two ordered lists are equal (position matters).
    pub fn assert_ordered_eq(expected: &[String], actual: &[String]) {
        assert_eq!(
            expected.len(),
            actual.len(),
            "length mismatch: expected {}, got {}.\n  expected: {expected:?}\n  actual:   {actual:?}",
            expected.len(),
            actual.len()
        );
        for (i, (exp, act)) in expected.iter().zip(actual.iter()).enumerate() {
            assert_eq!(
                exp, act,
                "mismatch at position {i}: expected '{exp}', got '{act}'"
            );
        }
    }
}
