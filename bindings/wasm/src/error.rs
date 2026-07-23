use wasm_bindgen::{JsError, JsValue};

use crate::database::indexed_db::IndexedDbError;

/// Errors that can cross the wasm boundary.
///
/// Every public binding method returns `Result<_, WasmError>`; wasm-bindgen
/// converts the `Err` variant into a thrown JS `Error` via the `From` impl
/// below, so JS callers get `instanceof Error` with a readable message
/// instead of an aborted wasm instance.
#[derive(Debug, thiserror::Error)]
pub enum WasmError {
    #[error("invalid URL: {0}")]
    Url(#[from] url::ParseError),

    #[error("network request failed: {0}")]
    Network(#[from] gloo_net::Error),

    #[error("JSON (de)serialization failed: {0}")]
    Json(#[from] serde_json::Error),

    #[error("CBOR deserialization failed: {0}")]
    Cbor(#[from] serde_cbor::Error),

    #[error("CESR processing failed: {0}")]
    Cesr(String),

    #[error("signing failed: {0}")]
    Signing(String),

    #[error("controller error: {0}")]
    Controller(String),

    #[error("database error: {0}")]
    Database(#[from] IndexedDbError),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("identifier not found: {0}")]
    IdentifierNotFound(String),

    #[error("attestation does not contain a SAID digest")]
    MissingSaid,

    #[error("watcher for identifier is not configured")]
    WatcherNotSet,

    #[error("{0} payload format is not supported")]
    UnsupportedPayload(&'static str),
}

impl From<WasmError> for JsValue {
    fn from(e: WasmError) -> JsValue {
        JsError::new(&e.to_string()).into()
    }
}
