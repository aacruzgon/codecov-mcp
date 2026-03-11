use std::sync::Arc;

use rmcp::{
    ServerHandler,
    model::{
        CallToolRequestParam, CallToolResult, Content, Implementation, ListToolsResult,
        PaginatedRequestParam, ServerCapabilities, ServerInfo, Tool,
    },
    schemars,
};
use schemars::schema_for;

use crate::{codecov_client::CodecovClient, error::AppError};

#[derive(Clone)]
pub struct CodecovMcpServer {
    client: Arc<CodecovClient>,
}

impl CodecovMcpServer {
    pub fn new(client: Arc<CodecovClient>) -> Self {
        Self { client }
    }

    fn tools() -> Vec<Tool> {
        use crate::tools::commit::GetCommitCoverageInput;
        let schema = schema_for!(GetCommitCoverageInput);
        let schema_value = serde_json::to_value(&schema).unwrap_or_default();
        let schema_obj = match schema_value {
            serde_json::Value::Object(map) => map,
            _ => serde_json::Map::new(),
        };
        vec![Tool::new(
            "get_commit_coverage",
            "Get coverage data for a specific commit SHA from Codecov. \
             Returns overall coverage percentage, line counts, and optionally \
             per-file breakdown.",
            Arc::new(schema_obj),
        )]
    }
}

impl ServerHandler for CodecovMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Codecov MCP server. Query coverage data for commits and pull requests.".into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: env!("CARGO_PKG_NAME").to_owned(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
            },
            ..Default::default()
        }
    }

    async fn list_tools(
        &self,
        _request: PaginatedRequestParam,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<ListToolsResult, rmcp::Error> {
        Ok(ListToolsResult {
            tools: Self::tools(),
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<CallToolResult, rmcp::Error> {
        match request.name.as_ref() {
            "get_commit_coverage" => {
                let input: crate::tools::commit::GetCommitCoverageInput =
                    serde_json::from_value(serde_json::Value::Object(
                        request.arguments.unwrap_or_default(),
                    ))
                    .map_err(AppError::Serialization)?;

                let result = crate::tools::commit::get_commit_coverage(&self.client, input)
                    .await
                    .map_err(|e| rmcp::Error::from(rmcp::model::ErrorData::from(e)))?;

                let text =
                    serde_json::to_string_pretty(&result).map_err(AppError::Serialization)?;

                Ok(CallToolResult {
                    content: vec![Content::text(text)],
                    is_error: Some(false),
                })
            }
            name => Err(rmcp::Error::invalid_params(
                format!("unknown tool: {name}"),
                None,
            )),
        }
    }
}
