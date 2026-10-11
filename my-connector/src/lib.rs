//! Example connector logic: a tiny tool-shaped dispatcher built from pure
//! functions.
//!
//! Replace this module with your connector's implementation. The CI gates
//! require `cargo fmt --check` to be clean, `cargo clippy` with zero warnings,
//! and 100% line and function coverage — so keep every function small, avoid
//! `unwrap`/`expect` outside tests, and test every branch.

/// Builds a greeting for `name`.
#[must_use]
pub fn greet(name: &str) -> String {
    format!("Hello, {name}!")
}

/// The tools this example connector exposes.
#[must_use]
pub fn tool_names() -> &'static [&'static str] {
    &["exampleTool"]
}

/// Runs the tool `tool` with the single argument `who`.
///
/// # Errors
///
/// Returns an error naming the tool when it is not one of [`tool_names`].
pub fn call_tool(tool: &str, who: &str) -> Result<String, String> {
    match tool {
        "exampleTool" => Ok(greet(who)),
        unknown => Err(format!("unknown tool: {unknown}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greets_by_name() {
        assert_eq!(greet("Ada"), "Hello, Ada!");
    }

    #[test]
    fn lists_its_tool() {
        assert_eq!(tool_names(), &["exampleTool"]);
    }

    #[test]
    fn runs_the_example_tool() {
        assert_eq!(
            call_tool("exampleTool", "Ada"),
            Ok("Hello, Ada!".to_string())
        );
    }

    #[test]
    fn rejects_an_unknown_tool() {
        assert_eq!(
            call_tool("nope", "Ada"),
            Err("unknown tool: nope".to_string())
        );
    }
}
