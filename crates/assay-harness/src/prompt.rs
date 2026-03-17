//! Prompt builder for assembling layered agent prompts.
//!
//! Consumes [`PromptLayer`](assay_types::PromptLayer) values and assembles
//! them into a final system prompt ordered by priority.

use assay_types::PromptLayer;

/// Assemble prompt layers into a single prompt string ordered by priority.
///
/// Layers are stable-sorted by `priority` (ascending — lowest value first).
/// Layers whose `content` is empty after trimming are excluded. Each surviving
/// layer is formatted as `## {name}\n\n{content}`, and sections are joined
/// with `\n\n---\n\n`. Returns an empty string when no layers survive filtering.
pub fn build_prompt(layers: &[PromptLayer]) -> String {
    let mut sorted: Vec<&PromptLayer> = layers
        .iter()
        .filter(|l| !l.content.trim().is_empty())
        .collect();
    sorted.sort_by_key(|l| l.priority);

    sorted
        .iter()
        .map(|l| format!("## {}\n\n{}", l.name, l.content))
        .collect::<Vec<_>>()
        .join("\n\n---\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use assay_types::PromptLayerKind;

    fn layer(kind: PromptLayerKind, name: &str, content: &str, priority: i32) -> PromptLayer {
        PromptLayer {
            kind,
            name: name.to_string(),
            content: content.to_string(),
            priority,
        }
    }

    #[test]
    fn empty_layers() {
        assert_eq!(build_prompt(&[]), "");
    }

    #[test]
    fn single_layer() {
        let layers = vec![layer(PromptLayerKind::System, "Rules", "Be helpful.", 0)];
        assert_eq!(build_prompt(&layers), "## Rules\n\nBe helpful.");
    }

    #[test]
    fn priority_ordering() {
        let layers = vec![
            layer(PromptLayerKind::Spec, "Spec", "spec content", 20),
            layer(PromptLayerKind::System, "System", "system content", 0),
            layer(PromptLayerKind::Project, "Project", "project content", 10),
        ];
        let result = build_prompt(&layers);
        let sections: Vec<&str> = result.split("\n\n---\n\n").collect();
        assert_eq!(sections.len(), 3);
        assert!(sections[0].starts_with("## System"));
        assert!(sections[1].starts_with("## Project"));
        assert!(sections[2].starts_with("## Spec"));
    }

    #[test]
    fn equal_priority_stability() {
        let layers = vec![
            layer(PromptLayerKind::Custom, "Alpha", "a", 5),
            layer(PromptLayerKind::Custom, "Beta", "b", 5),
            layer(PromptLayerKind::Custom, "Gamma", "c", 5),
        ];
        let result = build_prompt(&layers);
        let sections: Vec<&str> = result.split("\n\n---\n\n").collect();
        assert_eq!(sections.len(), 3);
        assert!(sections[0].starts_with("## Alpha"));
        assert!(sections[1].starts_with("## Beta"));
        assert!(sections[2].starts_with("## Gamma"));
    }

    #[test]
    fn empty_content_skipped() {
        let layers = vec![
            layer(PromptLayerKind::System, "Keep", "real content", 0),
            layer(PromptLayerKind::Project, "Empty", "", 1),
            layer(PromptLayerKind::Spec, "Whitespace", "   \n\t  ", 2),
        ];
        let result = build_prompt(&layers);
        assert_eq!(result, "## Keep\n\nreal content");
    }

    #[test]
    fn negative_priority() {
        let layers = vec![
            layer(PromptLayerKind::System, "Normal", "normal", 0),
            layer(PromptLayerKind::Custom, "Early", "early", -10),
        ];
        let result = build_prompt(&layers);
        let sections: Vec<&str> = result.split("\n\n---\n\n").collect();
        assert!(sections[0].starts_with("## Early"));
        assert!(sections[1].starts_with("## Normal"));
    }

    #[test]
    fn mixed_kinds() {
        let layers = vec![
            layer(PromptLayerKind::Custom, "Custom", "c", 2),
            layer(PromptLayerKind::System, "System", "s", 2),
            layer(PromptLayerKind::Spec, "Spec", "sp", 1),
            layer(PromptLayerKind::Project, "Project", "p", 3),
        ];
        let result = build_prompt(&layers);
        let sections: Vec<&str> = result.split("\n\n---\n\n").collect();
        // Priority determines order, not kind. Priority 1, then two at 2 (stable), then 3.
        assert!(sections[0].starts_with("## Spec"));
        assert!(sections[1].starts_with("## Custom"));
        assert!(sections[2].starts_with("## System"));
        assert!(sections[3].starts_with("## Project"));
    }
}
