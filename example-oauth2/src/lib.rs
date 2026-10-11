//! Example: a connector behind OAuth 2.0 sign-in.
//!
//! The sign-in itself lives in `manifest.json` (`auth` with the
//! `authorization_code` grant and PKCE): the host runs the OAuth flow and
//! injects the access token into outgoing requests. The connector code never
//! sees the token — tools just call the API.

/// The signed-in user's profile, as the API returns it.
pub struct Profile {
    /// Display name.
    pub name: String,
    /// Email address.
    pub email: String,
}

/// Renders a profile as a one-line summary.
#[must_use]
pub fn profile_summary(profile: &Profile) -> String {
    format!("{} <{}>", profile.name, profile.email)
}

/// The tools this example connector exposes.
#[must_use]
pub fn tool_names() -> &'static [&'static str] {
    &["getProfile"]
}

/// Runs the tool `tool` for `profile`.
///
/// # Errors
///
/// Returns an error naming the tool when it is not one of [`tool_names`].
pub fn call_tool(tool: &str, profile: &Profile) -> Result<String, String> {
    match tool {
        "getProfile" => Ok(profile_summary(profile)),
        unknown => Err(format!("unknown tool: {unknown}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ada() -> Profile {
        Profile {
            name: "Ada".to_string(),
            email: "ada@example.com".to_string(),
        }
    }

    #[test]
    fn summarizes_a_profile() {
        assert_eq!(profile_summary(&ada()), "Ada <ada@example.com>");
    }

    #[test]
    fn lists_its_tool() {
        assert_eq!(tool_names(), &["getProfile"]);
    }

    #[test]
    fn returns_the_profile() {
        assert_eq!(
            call_tool("getProfile", &ada()),
            Ok("Ada <ada@example.com>".to_string())
        );
    }

    #[test]
    fn rejects_an_unknown_tool() {
        assert_eq!(
            call_tool("nope", &ada()),
            Err("unknown tool: nope".to_string())
        );
    }
}
