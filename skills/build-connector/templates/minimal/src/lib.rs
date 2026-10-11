//! A starting point for a connector: one tool, one resource and one prompt,
//! with no network. Rename it, then replace each part with your own.

use connectors_sdk::content::{text_resource, user_prompt};
use connectors_sdk::{Connector, ConnectorError, ToolRequest, ToolResponse};
use serde::Deserialize;

connectors_sdk::manifest!("manifest.json");

const README_URI: &str = "minimal-starter://readme";

/// The connector.
#[derive(Debug)]
pub struct Minimal;

#[derive(Debug, Deserialize)]
struct Named {
    name: String,
}

impl Connector for Minimal {
    fn call_tool(request: ToolRequest) -> Result<ToolResponse, ConnectorError> {
        match request.tool.as_str() {
            "greet" => {
                let input: Named = request.json()?;
                Ok(ToolResponse::text(format!("Hello, {}!", input.name)))
            }
            other => Err(ConnectorError::NotFound(format!("no tool {other}"))),
        }
    }

    fn read_resource(uri: &str) -> Result<String, ConnectorError> {
        if uri == README_URI {
            Ok(text_resource(
                uri,
                "text/plain",
                "This connector greets people.",
            ))
        } else {
            Err(ConnectorError::NotFound(format!("no resource {uri}")))
        }
    }

    fn get_prompt(name: &str, args: &str) -> Result<String, ConnectorError> {
        if name != "welcome" {
            return Err(ConnectorError::NotFound(format!("no prompt {name}")));
        }
        let args: Named = serde_json::from_str(args)
            .map_err(|err| ConnectorError::InvalidInput(format!("arguments: {err}")))?;
        Ok(user_prompt(
            "Ask the assistant to welcome someone",
            &format!("Write a short, warm welcome for {}.", args.name),
        ))
    }
}

connectors_sdk::export_connector!(Minimal);

#[cfg(test)]
mod tests {
    use super::*;

    fn request(tool: &str, body: &str) -> ToolRequest {
        ToolRequest {
            tool: tool.into(),
            body: body.into(),
            ..ToolRequest::default()
        }
    }

    #[test]
    fn greets() {
        let greeted = Minimal::call_tool(request("greet", r#"{"name":"Ada"}"#)).unwrap();
        assert_eq!(greeted, ToolResponse::text("Hello, Ada!"));
        let unknown = Minimal::call_tool(request("nope", "{}"));
        assert_eq!(
            unknown,
            Err(ConnectorError::NotFound("no tool nope".into()))
        );
        let bad = Minimal::call_tool(request("greet", "{}"));
        assert!(
            matches!(bad, Err(ConnectorError::InvalidInput(_))),
            "{bad:?}"
        );
    }

    #[test]
    fn reads_its_resource_and_renders_its_prompt() {
        assert!(
            Minimal::read_resource(README_URI)
                .unwrap()
                .contains("greets people")
        );
        assert!(matches!(
            Minimal::read_resource("minimal-starter://nope"),
            Err(ConnectorError::NotFound(_))
        ));
        let prompt = Minimal::get_prompt("welcome", r#"{"name":"Ada"}"#).unwrap();
        assert!(prompt.contains("welcome for Ada"), "{prompt}");
        assert!(matches!(
            Minimal::get_prompt("welcome", "[]"),
            Err(ConnectorError::InvalidInput(_))
        ));
        assert!(matches!(
            Minimal::get_prompt("nope", "{}"),
            Err(ConnectorError::NotFound(_))
        ));
    }

    #[test]
    fn manifest_is_embedded() {
        assert!(MANIFEST.contains("\"name\": \"minimal-starter\""));
    }
}
