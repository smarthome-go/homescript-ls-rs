use smarthome_sdk_rs::HmsRunMode;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

struct Backend {
    client: Client,
    smarthome_client: smarthome_sdk_rs::Client,
}

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
            language_id: "rush".to_string(),
            uri: params.text_document.uri,
            text: params.text_document.text,
            version: params.text_document.version,
        })
        .await
    }

    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        self.create_diagnostics(TextDocumentItem {
            language_id: "rush".to_string(),
            uri: params.text_document.uri,
            version: params.text_document.version,
            text: std::mem::take(&mut params.content_changes[0].text),
        })
        .await
    }
}

impl Backend {
    async fn create_diagnostics(&self, params: TextDocumentItem) {
        let raw_diagnostics = match self
            .smarthome_client
            .exec_homescript_code(&params.text, vec![], HmsRunMode::Lint)
            .await
        {
            Ok(res) => res.errors,
            Err(err) => panic!("{err}"),
        };

        // transform the diagnostics into the LSP form
        let diagnostics = raw_diagnostics
            .iter()
            .map(|diagnostic| {
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
                    Some(match diagnostic.kind.as_str() {
                        "Info" => DiagnosticSeverity::INFORMATION,
                        "Warning" => DiagnosticSeverity::WARNING,
                        _ => DiagnosticSeverity::ERROR,
                    }),
                    None,
                    Some("homescript-analyzer".to_string()),
                    format!("{}: {}", diagnostic.kind, diagnostic.message.to_string()),
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
