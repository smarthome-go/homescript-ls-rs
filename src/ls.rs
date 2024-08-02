use anyhow::{bail, Context};
use serde::Deserialize;
use smarthome_sdk_rs::HmsRunMode;
use std::fs;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

struct Backend {
    client: Client,
    smarthome_client: smarthome_sdk_rs::Client,
    // workspace_context: Arc<Mutex<RefCell<HomescriptMetadata>>>,
}

#[derive(Deserialize, Debug)]
pub struct HomescriptMetadata {
    pub id: String,
    pub is_driver: bool,
}

const WORKSPACE_TOML_NAME: &str = ".hms.toml";

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                ..ServerCapabilities::default()
            },
            server_info: Some(ServerInfo {
                name: env!("CARGO_PKG_NAME").to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.create_diagnostics(TextDocumentItem {
            language_id: "homescript".to_string(),
            uri: params.text_document.uri,
            text: params.text_document.text,
            version: params.text_document.version,
        })
        .await
    }

    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        self.create_diagnostics(TextDocumentItem {
            language_id: "homescript".to_string(),
            uri: params.text_document.uri,
            version: params.text_document.version,
            text: std::mem::take(&mut params.content_changes[0].text),
        })
        .await
    }
}

impl Backend {
    fn try_read_workspace_toml_unwrapped(
        &self,
        file_path: Url,
    ) -> anyhow::Result<HomescriptMetadata> {
        let Ok(file_path) = file_path.to_file_path() else {
            bail!("invalid document file path");
        };

        let mut workspace_toml_path = file_path;
        workspace_toml_path.set_file_name(WORKSPACE_TOML_NAME);

        let workspace_toml_str = fs::read_to_string(workspace_toml_path)
            .with_context(|| format!("read {WORKSPACE_TOML_NAME}"))?;

        let manifest: HomescriptMetadata = toml::from_str(&workspace_toml_str)
            .with_context(|| format!("invalid workspace file {WORKSPACE_TOML_NAME}"))?;

        Ok(manifest)
    }

    async fn send_error(&self, uri: Url, message: String) {
        self.client
            .publish_diagnostics(
                uri,
                vec![Diagnostic::new(
                    Range::new(Position::new(0, 0), Position::new(0, 0)),
                    Some(DiagnosticSeverity::ERROR),
                    None,
                    Some("homescript-analyzer".to_string()),
                    message,
                    None,
                    None,
                )],
                None,
            )
            .await;
    }

    async fn create_diagnostics(&self, params: TextDocumentItem) {
        let ctx = match self.try_read_workspace_toml_unwrapped(params.uri.clone()) {
            Ok(ctx) => ctx,
            Err(err) => {
                self.send_error(params.uri, format!("index workspace: {err}"))
                    .await;
                return;
            }
        };

        let raw_diagnostics = match self
            .smarthome_client
            .exec_homescript_code(
                &params.text,
                vec![],
                HmsRunMode::Lint {
                    module_name: &ctx.id,
                    is_driver: ctx.is_driver,
                },
            )
            .await
        {
            Ok(res) => res.errors,
            Err(err) => panic!("{err}"),
        };

        // transform the errors / diagnostics into the LSP form
        let diagnostics = raw_diagnostics
            .iter()
            .map(|diagnostic| {
                let (message, level) =
                    match (&diagnostic.syntax_error, &diagnostic.diagnostic_error) {
                        (Some(syntax), None) => (syntax.message.clone(), DiagnosticSeverity::ERROR),
                        (None, Some(diagnostic)) => (
                            diagnostic.message.clone(),
                            match diagnostic.kind {
                                0 => DiagnosticSeverity::HINT,
                                1 => DiagnosticSeverity::INFORMATION,
                                2 => DiagnosticSeverity::WARNING,
                                3 => DiagnosticSeverity::ERROR,
                                _ => unreachable!("Illegal kind"),
                            },
                        ),
                        _ => unreachable!("Illegal state"),
                    };

                Diagnostic::new(
                    Range::new(
                        Position::new(
                            (diagnostic.span.start.line - 1) as u32,
                            (diagnostic.span.start.column - 1) as u32,
                        ),
                        Position::new(
                            (diagnostic.span.end.line - 1) as u32,
                            (diagnostic.span.end.column) as u32,
                        ),
                    ),
                    Some(level),
                    None,
                    Some("homescript-analyzer".to_string()),
                    message,
                    None,
                    None,
                )
            })
            .collect();

        self.client
            .publish_diagnostics(params.uri.clone(), diagnostics, Some(params.version))
            .await;
    }
}

pub async fn start_service(smarthome_client: smarthome_sdk_rs::Client) {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|lsp_client| Backend {
        client: lsp_client,
        smarthome_client,
    });

    Server::new(stdin, stdout, socket).serve(service).await;
}
