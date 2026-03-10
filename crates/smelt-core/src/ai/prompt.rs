//! Prompt template construction for AI conflict resolution.

/// Returns the system prompt instructing the LLM to output only raw file content.
pub fn build_system_prompt() -> &'static str {
    "You are resolving a git merge conflict. Output ONLY the complete resolved file content.\n\
     \n\
     Rules:\n\
     - No markdown fences, no explanations, no commentary\n\
     - Output the COMPLETE file, not a diff or patch\n\
     - Preserve all functionality from both versions\n\
     - If changes are in different parts of the file, include both\n\
     - If changes conflict on the same lines, integrate both intents\n\
     - Maintain consistent code style with the surrounding context"
}

/// Build the user prompt with 3-way merge context.
pub fn build_resolution_prompt(
    file_path: &str,
    base: &str,
    ours: &str,
    theirs: &str,
    session_name: &str,
    task_desc: Option<&str>,
    commit_subjects: &[String],
) -> String {
    let task_line = task_desc.unwrap_or("(none)");
    let commits_line = if commit_subjects.is_empty() {
        "(none)".to_owned()
    } else {
        commit_subjects.join("\n")
    };

    format!(
        "File: {file_path}\n\
         Session: {session_name}\n\
         Task: {task_line}\n\
         Recent commits:\n\
         {commits_line}\n\
         \n\
         ## Base version (common ancestor)\n\
         {base}\n\
         \n\
         ## Current version (ours - target branch)\n\
         {ours}\n\
         \n\
         ## Incoming version (theirs - session branch)\n\
         {theirs}\n\
         \n\
         Resolve the conflict and output the complete merged file."
    )
}

/// Build a retry prompt incorporating user feedback on a previous attempt.
pub fn build_retry_prompt(original_prompt: &str, feedback: &str) -> String {
    format!(
        "{original_prompt}\n\
         \n\
         ## User feedback on previous attempt\n\
         {feedback}\n\
         \n\
         Please resolve again incorporating this feedback."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_prompt_contains_key_instructions() {
        let prompt = build_system_prompt();
        assert!(prompt.contains("ONLY"));
        assert!(prompt.contains("No markdown"));
    }

    #[test]
    fn resolution_prompt_includes_all_sections() {
        let prompt = build_resolution_prompt(
            "src/main.rs",
            "base content",
            "ours content",
            "theirs content",
            "feature-auth",
            Some("Add authentication"),
            &["feat: add login".to_owned(), "fix: handle errors".to_owned()],
        );
        assert!(prompt.contains("src/main.rs"));
        assert!(prompt.contains("feature-auth"));
        assert!(prompt.contains("Add authentication"));
        assert!(prompt.contains("base content"));
        assert!(prompt.contains("ours content"));
        assert!(prompt.contains("theirs content"));
        assert!(prompt.contains("feat: add login"));
        assert!(prompt.contains("fix: handle errors"));
    }

    #[test]
    fn resolution_prompt_handles_no_task_and_no_commits() {
        let prompt = build_resolution_prompt(
            "lib.rs",
            "base",
            "ours",
            "theirs",
            "session-1",
            None,
            &[],
        );
        assert!(prompt.contains("Task: (none)"));
        assert!(prompt.contains("(none)"));
    }

    #[test]
    fn retry_prompt_includes_original_and_feedback() {
        let original = "original prompt text";
        let feedback = "keep the imports from session-a";
        let prompt = build_retry_prompt(original, feedback);
        assert!(prompt.contains(original));
        assert!(prompt.contains(feedback));
        assert!(prompt.contains("User feedback on previous attempt"));
    }
}
