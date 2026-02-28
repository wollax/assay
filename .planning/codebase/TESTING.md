# Testing Patterns

**Analysis Date:** 2026-02-28

## Test Framework

- **Standard library**: Use Rust's built-in testing framework (`#[test]`, `#[cfg(test)]`)
- **No external test framework**: Currently no specialized testing crates in dependencies (no `pytest`, `criterion`, `proptest`)
- **Expected dependencies**: For future testing, consider:
  - `tokio::test` for async tests (tokio is already in workspace)
  - `criterion` for benchmarking
  - `proptest` or `quickcheck` for property-based testing

## Test File Organization

- **Unit tests**: Place `#[cfg(test)] mod tests { }` blocks in the same file as the code being tested
- **Integration tests**: Place in `tests/` directory at crate root (currently no tests present)
- **No dedicated test files**: Follow Rust convention of collocated unit tests
- **Test discovery**: Tests should be discoverable by `cargo test` command

## Test Structure

- Use `#[test]` attribute for individual test functions
- Test function naming convention: `test_<function_name>_<scenario>` (e.g., `test_spec_creation_succeeds`)
- Group related tests using `mod tests { }` within `#[cfg(test)]`
- Use `assert!`, `assert_eq!`, `assert_ne!` macros for assertions
- Structure: Arrange → Act → Assert pattern
- Example structure (to implement):
  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;

      #[test]
      fn test_spec_creation_succeeds() {
          // Arrange
          let spec = Spec {
              name: "test".to_string(),
              description: "test spec".to_string(),
          };

          // Act
          let result = validate_spec(&spec);

          // Assert
          assert!(result.is_ok());
      }
  }
  ```

## Mocking

- **No mocking framework**: Currently no mock libraries in dependencies
- **Preferred approach**: Use dependency injection and concrete test implementations
- **Traits**: Define traits for testable interfaces
- **Test doubles**: Create test-specific types that implement traits (stub/fake pattern)
- When needed, consider `mockall` crate for complex mocking scenarios

## Fixtures and Factories

- **No factory crate**: Currently no factory pattern infrastructure
- **Recommend**: Create builder patterns or factory functions within test modules
- **Shared test data**: Define test fixtures as module-level constants in test blocks
- Example approach (to implement):
  ```rust
  #[cfg(test)]
  mod tests {
      fn sample_spec() -> Spec {
          Spec {
              name: "sample".to_string(),
              description: "sample description".to_string(),
          }
      }
  }
  ```

## Coverage

- **No coverage tooling**: Currently no code coverage integration
- **Enforcement**: Not required by CI (see CLAUDE.md - only `just ready` checks fmt/lint/test/deny)
- **Recommendation**: When mature, use `tarpaulin` or `llvm-cov` for coverage reporting
- **Minimum coverage**: No baseline enforced currently

## Test Types

- **Unit tests**: Test individual functions/methods in isolation
  - Location: Inline in source files via `#[cfg(test)]` modules
  - Dependencies: Direct, minimal external calls

- **Integration tests**: Test module interactions and public API contracts
  - Location: `tests/` directory (to be created as tests grow)
  - Dependencies: Use public APIs of crates

- **Doc tests**: Use `///` doc comments with code examples (recommended for public APIs)
  - Enable via inline code blocks in documentation
  - Automatically discovered and run by `cargo test --doc`

- **No property-based tests**: Not currently in use; consider for data validation

## Common Patterns

- **Result assertions**: Test error cases alongside success cases
  ```rust
  #[test]
  fn test_spec_validation_fails_on_empty_name() {
      let spec = Spec {
          name: "".to_string(),
          description: "description".to_string(),
      };
      assert!(validate_spec(&spec).is_err());
  }
  ```

- **Type serialization tests**: For `assay-types` crate, test serde/schema functionality
  ```rust
  #[test]
  fn test_spec_serializes_to_json() {
      let spec = sample_spec();
      let json = serde_json::to_string(&spec).unwrap();
      assert!(!json.is_empty());
  }
  ```

- **Workflow verification**: Test workflow orchestration logic flows correctly

- **Gate evaluation**: Test gate conditions and pass/fail logic

- **Review correctness**: Test review comment aggregation and approval decisions

- **Run tests via**: `cargo test` or `just test` (from root)
  - Single crate: `cargo test -p assay-core`
  - With output: `cargo test -- --nocapture`
  - Specific test: `cargo test test_spec_creation`

---
*Testing analysis: 2026-02-28*
