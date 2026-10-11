//! Example: MCP prompts.
//!
//! The manifest declares the `summarize` prompt with its arguments. The
//! connector renders a prompt into messages; required arguments are checked
//! before rendering.

/// A single rendered prompt message.
pub struct PromptMessage {
    /// The message role, e.g. `"user"`.
    pub role: &'static str,
    /// The message content.
    pub content: String,
}

/// The prompts this example connector exposes.
#[must_use]
pub fn prompt_names() -> &'static [&'static str] {
    &["summarize"]
}

/// Renders the `summarize` prompt for `text` in `style`.
#[must_use]
pub fn render_summarize(text: &str, style: Option<&str>) -> Vec<PromptMessage> {
    let style = style.unwrap_or("concise");
    vec![PromptMessage {
        role: "user",
        content: format!("Summarize the following text in a {style} style:\n\n{text}"),
    }]
}

/// Renders the prompt `name` with the `text` and `style` arguments.
///
/// # Errors
///
/// Returns an error naming the prompt when it is not one of [`prompt_names`],
/// or when the required `text` argument is empty.
pub fn render_prompt(
    name: &str,
    text: &str,
    style: Option<&str>,
) -> Result<Vec<PromptMessage>, String> {
    if text.is_empty() {
        return Err("the text argument is required".to_string());
    }
    match name {
        "summarize" => Ok(render_summarize(text, style)),
        unknown => Err(format!("unknown prompt: {unknown}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_its_prompt() {
        assert_eq!(prompt_names(), &["summarize"]);
    }

    #[test]
    fn renders_with_an_explicit_style() {
        let messages = render_summarize("hello", Some("detailed"));
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role, "user");
        assert_eq!(
            messages[0].content,
            "Summarize the following text in a detailed style:\n\nhello"
        );
    }

    #[test]
    fn defaults_to_the_concise_style() {
        let messages = render_prompt("summarize", "hello", None).unwrap();
        assert!(messages[0].content.contains("concise style"));
    }

    #[test]
    fn renders_through_the_dispatcher() {
        let messages = render_prompt("summarize", "hello", Some("brief")).unwrap();
        assert_eq!(messages.len(), 1);
        assert!(messages[0].content.contains("brief style"));
    }

    #[test]
    fn rejects_empty_text() {
        assert_eq!(
            render_prompt("summarize", "", None).map(|_| ()),
            Err("the text argument is required".to_string())
        );
    }

    #[test]
    fn rejects_an_unknown_prompt() {
        assert_eq!(
            render_prompt("nope", "hello", None).map(|_| ()),
            Err("unknown prompt: nope".to_string())
        );
    }
}
