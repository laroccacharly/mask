use anyhow::Result;
use std::sync::mpsc;
use std::thread;

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tower_http::services::{ServeDir, ServeFile};

use crate::inspect::occupations;
use crate::inspect::{self, Finding};
use crate::{TokenFormatter, Vault, VaultMapping, ui::Ui};

enum VaultCommand {
    Encode {
        text: String,
        findings: Vec<Finding>,
        reply: tokio::sync::oneshot::Sender<Result<String>>,
    },
    Decode {
        text: String,
        reply: tokio::sync::oneshot::Sender<Result<String>>,
    },
    Mappings {
        reply: tokio::sync::oneshot::Sender<Result<Vec<VaultMapping>>>,
    },
    Reset {
        reply: tokio::sync::oneshot::Sender<Result<()>>,
    },
}

type InspectRequest = (String, tokio::sync::oneshot::Sender<Result<Vec<Finding>>>);

#[derive(Clone)]
struct AppState {
    vault: VaultService,
    inspect: InspectService,
}

#[derive(Clone)]
struct VaultService {
    requests: mpsc::Sender<VaultCommand>,
}

#[derive(Clone)]
struct InspectService {
    requests: mpsc::Sender<InspectRequest>,
}

#[derive(Deserialize)]
struct EncodeRequest {
    text: String,
    #[serde(default)]
    findings: Vec<Finding>,
}

#[derive(Serialize)]
struct InspectResponse {
    findings: Vec<Finding>,
}

#[derive(Serialize)]
struct VaultMappingsResponse {
    mappings: Vec<VaultMapping>,
}

impl VaultService {
    fn start() -> Result<Self> {
        let (requests, receiver) = mpsc::channel::<VaultCommand>();
        let (ready, initialized) = mpsc::channel();
        thread::spawn(move || {
            let vault = Vault::open(crate::vault::DEFAULT_SNAPSHOT)
                .and_then(|vault| {
                    vault.with_labels(&["person", "organization", "location", "email"])
                })
                .map(|vault| vault.with_token_formatter(TokenFormatter));
            match vault {
                Ok(vault) => {
                    let _ = ready.send(Ok(()));
                    for command in receiver {
                        match command {
                            VaultCommand::Encode {
                                text,
                                findings,
                                reply,
                            } => {
                                let result = vault.encode_with_findings(&text, &findings);
                                let _ = reply.send(result);
                            }
                            VaultCommand::Decode { text, reply } => {
                                let result = vault.decode(&text);
                                let _ = reply.send(result);
                            }
                            VaultCommand::Mappings { reply } => {
                                let result = vault.mappings();
                                let _ = reply.send(result);
                            }
                            VaultCommand::Reset { reply } => {
                                let result = vault.reset_snapshot();
                                let _ = reply.send(result);
                            }
                        }
                    }
                }
                Err(error) => {
                    let _ = ready.send(Err(error));
                }
            }
        });
        initialized.recv()??;
        Ok(Self { requests })
    }

    async fn encode(&self, text: String, findings: Vec<Finding>) -> Result<String> {
        let (reply, encoded) = tokio::sync::oneshot::channel();
        self.requests.send(VaultCommand::Encode {
            text,
            findings,
            reply,
        })?;
        encoded.await?
    }

    async fn decode(&self, text: String) -> Result<String> {
        let (reply, decoded) = tokio::sync::oneshot::channel();
        self.requests.send(VaultCommand::Decode { text, reply })?;
        decoded.await?
    }

    async fn mappings(&self) -> Result<Vec<VaultMapping>> {
        let (reply, mappings) = tokio::sync::oneshot::channel();
        self.requests.send(VaultCommand::Mappings { reply })?;
        mappings.await?
    }

    async fn reset(&self) -> Result<()> {
        let (reply, reset) = tokio::sync::oneshot::channel();
        self.requests.send(VaultCommand::Reset { reply })?;
        reset.await?
    }
}

impl InspectService {
    fn start() -> Self {
        let (requests, receiver) = mpsc::channel::<InspectRequest>();
        thread::spawn(move || {
            for (text, response) in receiver {
                let result = occupations::inspect(&text).and_then(|inspection| {
                    let artifact = std::path::Path::new(occupations::ARTIFACT);
                    inspect::write_artifact(artifact, &text, &inspection.findings)?;
                    Ok(inspection.findings)
                });
                let _ = response.send(result);
            }
        });
        Self { requests }
    }

    async fn inspect(&self, text: String) -> Result<Vec<Finding>> {
        let (response, findings) = tokio::sync::oneshot::channel();
        self.requests.send((text, response))?;
        findings.await?
    }
}

pub fn run(port: u16) -> Result<()> {
    let ui = Ui::from_workspace()?;
    ui.build()?;
    let rt = tokio::runtime::Runtime::new()?;
    let server = serve(port, ui);
    rt.block_on(server)
}

async fn serve(port: u16, ui: Ui) -> Result<()> {
    let vault = VaultService::start()?;
    let inspect = InspectService::start();
    let health_handler = get(health);
    let encode_handler = post(encode);
    let decode_handler = post(decode);
    let inspect_occupations_handler = post(inspect_occupations);
    let vault_mappings_handler = get(vault_mappings);
    let reset_vault_handler = post(reset_vault);
    let mut app = Router::new()
        .route("/health", health_handler)
        .route("/api/encode", encode_handler)
        .route("/api/decode", decode_handler)
        .route("/api/inspect/occupations", inspect_occupations_handler)
        .route("/api/vault", vault_mappings_handler)
        .route("/api/vault/reset", reset_vault_handler)
        .with_state(AppState { vault, inspect });
    if ui.assets_exist() {
        let dist = ui.dist_dir();
        let index = dist.join("index.html");
        let fallback = ServeFile::new(index);
        let static_files = ServeDir::new(dist)
            .append_index_html_on_directories(true)
            .fallback(fallback);
        app = app.fallback_service(static_files);
    }
    let addr = format!("127.0.0.1:{port}");
    let listener = TcpListener::bind(&addr).await?;
    println!("Listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> &'static str {
    "ok"
}

async fn encode(
    State(state): State<AppState>,
    Json(body): Json<EncodeRequest>,
) -> Result<String, (StatusCode, String)> {
    state
        .vault
        .encode(body.text, body.findings)
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))
}

async fn decode(
    State(state): State<AppState>,
    text: String,
) -> Result<String, (StatusCode, String)> {
    state
        .vault
        .decode(text)
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))
}

async fn vault_mappings(
    State(state): State<AppState>,
) -> Result<Json<VaultMappingsResponse>, (StatusCode, String)> {
    state
        .vault
        .mappings()
        .await
        .map(|mappings| Json(VaultMappingsResponse { mappings }))
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))
}

async fn reset_vault(
    State(state): State<AppState>,
) -> Result<Json<VaultMappingsResponse>, (StatusCode, String)> {
    state
        .vault
        .reset()
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    Ok(Json(VaultMappingsResponse {
        mappings: Vec::new(),
    }))
}

async fn inspect_occupations(
    State(state): State<AppState>,
    text: String,
) -> Result<Json<InspectResponse>, (StatusCode, String)> {
    state
        .inspect
        .inspect(text)
        .await
        .map(|findings| Json(InspectResponse { findings }))
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))
}
