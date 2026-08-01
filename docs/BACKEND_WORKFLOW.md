# Backend Workflow & Acceptance Criteria

When making modifications to the backend Rust code of the Familiar project, all AI coding assistants must adhere to the following workflow to ensure code quality and stability.

## Workflow Steps

1. **Implement Changes**: Make the requested modifications to the Rust codebase (e.g., in `crates/` or `app/src-tauri/`).
2. **Compilation Check**: Use the `run_command` tool to execute `cargo check` or `cargo clippy`. You MUST ensure that the code compiles successfully without any errors.
3. **Fix Warnings**: You MUST resolve any and all compilation warnings (such as unused variables, unused imports, deprecation warnings, or Clippy lints). The code should compile completely clean.
4. **Test**: If applicable, run `cargo test` to ensure that logic remains correct and no regressions were introduced.
5. **Deliver**: Present the final working state to the user. Do not claim the task is complete until the compilation checks pass cleanly with zero warnings.
