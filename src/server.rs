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
        use crate::tools::changed_files::GetChangedFilesCoverageInput;
        use crate::tools::commit::GetCommitCoverageInput;
        use crate::tools::suggest::SuggestTestTargetsInput;

        fn make_schema<T: schemars::JsonSchema>() -> serde_json::Map<String, serde_json::Value> {
            let value = serde_json::to_value(schema_for!(T)).unwrap_or_default();
            match value {
                serde_json::Value::Object(map) => map,
                _ => serde_json::Map::new(),
            }
        }

        vec![
            Tool::new(
                "get_commit_coverage",
                "Get coverage data for a specific commit SHA from Codecov. \
                 Returns overall coverage percentage, line counts, and optionally \
                 per-file breakdown.",
                Arc::new(make_schema::<GetCommitCoverageInput>()),
            ),
            Tool::new(
                "get_changed_files_coverage",
                "Get patch coverage for a pull request from Codecov. \
                 Returns base/head/patch coverage totals and per-file patch \
                 coverage breakdown for all changed files.",
                Arc::new(make_schema::<GetChangedFilesCoverageInput>()),
            ),
            Tool::new(
                "suggest_test_targets",
                "Rank changed files in a pull request by how urgently they need tests. \
                 Uses a weighted scoring formula based on patch miss rate, uncovered lines, \
                 whether the file is new, and overall coverage. Supports filtering by file \
                 extension and minimum uncovered lines.",
                Arc::new(make_schema::<SuggestTestTargetsInput>()),
            ),
        ]
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
            "get_changed_files_coverage" => {
                let input: crate::tools::changed_files::GetChangedFilesCoverageInput =
                    serde_json::from_value(serde_json::Value::Object(
                        request.arguments.unwrap_or_default(),
                    ))
                    .map_err(AppError::Serialization)?;

                let result =
                    crate::tools::changed_files::get_changed_files_coverage(&self.client, input)
                        .await
                        .map_err(|e| rmcp::Error::from(rmcp::model::ErrorData::from(e)))?;

                let text =
                    serde_json::to_string_pretty(&result).map_err(AppError::Serialization)?;

                Ok(CallToolResult {
                    content: vec![Content::text(text)],
                    is_error: Some(false),
                })
            }
            "suggest_test_targets" => {
                let input: crate::tools::suggest::SuggestTestTargetsInput =
                    serde_json::from_value(serde_json::Value::Object(
                        request.arguments.unwrap_or_default(),
                    ))
                    .map_err(AppError::Serialization)?;

                let result =
                    crate::tools::suggest::suggest_test_targets(&self.client, input)
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
