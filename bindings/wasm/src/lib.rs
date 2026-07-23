use ed25519_dalek::SigningKey;
use gloo_net::http::Request;
use js_sys::Uint8Array;
use keri_core::{
    actor::{parse_event_stream, prelude::Message},
    event_message::signed_event_message::Op,
    oobi::{LocationScheme, Oobi},
    prefix::{
        BasicPrefix, IdentifierPrefix, IndexedSignature, SeedPrefix,
        SelfSigningPrefix,
    },
    query::query_event::{SignedKelQuery, SignedQueryMessage},
    signer::Signer,
};
use keri_sdk::{Controller, Identifier};
use said::SelfAddressingIdentifier;
use teliox::state::vc_state::TelState;
use std::sync::Arc;
use url::Url;
use wasm_bindgen::prelude::*;

pub mod database;
pub mod error;
use crate::database::indexed_db::IndexedDbDatabase as Database;
use crate::error::WasmError;

#[wasm_bindgen]
pub enum VcState {
    Issued,
    Revoked,
    NotIssued,
}

#[wasm_bindgen]
pub struct JsIdentifier {
    inner: Identifier<Database>,
    db: Arc<Database>,
    alias: String,
    signer: Arc<Signer>,
    watcher_oobi: Option<LocationScheme>,
}

impl JsIdentifier {
    pub fn new(alias: String, inner: Identifier<Database>, signer: Arc<Signer>, db: Arc<Database>, watcher_oobi: Option<LocationScheme>) -> Self {
        Self {
            inner,
            db,
            alias,
            signer,
            watcher_oobi,
        }
    }
}

#[wasm_bindgen]
impl JsIdentifier {
    pub fn get_prefix(&self) -> String {
        self.inner.get_prefix().to_string()
    }

    pub fn get_kel(&self) -> Result<String, WasmError> {
        let kel = self.inner.get_own_kel().ok_or_else(|| {
            WasmError::IdentifierNotFound(self.inner.get_prefix().to_string())
        })?;
        Ok(format!("{:?}", kel))
    }

    pub fn set_alias(&mut self, alias: String) -> Result<(), WasmError> {
        self.db.update_identifier_alias(&self.alias, &alias)?;
        self.alias = alias;
        Ok(())
    }

    pub async fn add_watcher(&mut self, url: String) -> Result<(), WasmError> {
        let url = Url::parse(&url)?;
        let res = Request::get(url.join("introduce")?.as_str())
            .send()
            .await?;
        let res_str = res.text().await?;
        let oobi: LocationScheme = serde_json::from_str(&res_str)?;
        self.watcher_oobi = Some(oobi.clone());
        let watcher_prefix = oobi.clone().eid;

        let add_watcher_event = self
            .inner
            .add_watcher(watcher_prefix.clone())
            .map_err(WasmError::Controller)?;

        let signature = self
            .signer
            .sign(add_watcher_event.as_bytes())
            .map_err(|e| WasmError::Signing(e.to_string()))?;
        let sig = SelfSigningPrefix::new(
            cesrox::primitives::codes::self_signing::SelfSigning::Ed25519Sha512,
            signature,
        );
        let (_, messages) = self
            .inner
            .finalize_add_watcher(add_watcher_event.as_bytes(), sig)
            .map_err(WasmError::Controller)?;

        for message in messages {
            let request_url: Option<String> = match message {
                Message::Notice(_) => {
                    Some(url.join("process")?.to_string())
                }
                Message::Op(Op::Reply(_)) => {
                    Some(url.join("register")?.to_string())
                }
                _ => {
                    log::warn!("Unsupported message type: {:?}", message);
                    None
                }
            };
            if let Some(request_url) = request_url {
                let cesr = message
                    .to_cesr()
                    .map_err(|e| WasmError::Cesr(e.to_string()))?;
                let body = Uint8Array::from(cesr.as_slice());
                let _ = Request::post(&request_url)
                    .header("Content-Type", "application/json")
                    .body(&body)?
                    .send()
                    .await?;
            }
        }

        self.db.update_identifier_watcher(&self.alias, oobi.clone())?;

        Ok(())
    }

    pub fn get_watcher(&self) -> Option<String> {
        self.watcher_oobi.as_ref().map(|scheme| scheme.url.to_string())
    }
}

#[wasm_bindgen]
pub struct JsController {
    inner: Controller<Database, Database>,
    db: Arc<Database>,
}

struct KeysConfig {
    pub current: SeedPrefix,
    pub next: SeedPrefix,
}

impl Default for KeysConfig {
    fn default() -> Self {
        let current = SigningKey::generate(&mut rand::rngs::OsRng);
        let next = SigningKey::generate(&mut rand::rngs::OsRng);
        Self {
            current: SeedPrefix::RandomSeed256Ed25519(
                current.as_bytes().to_vec(),
            ),
            next: SeedPrefix::RandomSeed256Ed25519(next.as_bytes().to_vec()),
        }
    }
}

#[wasm_bindgen]
impl JsController {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<JsController, WasmError> {
        // Setting the logger fails when one is already installed (e.g. a
        // second controller is constructed) — keep the existing logger then.
        let _ = log::set_logger(&wasm_bindgen_console_logger::DEFAULT_LOGGER);
        log::set_max_level(log::LevelFilter::Info);

        let event_database = Arc::new(Database::new());
        let tel_database = Arc::new(Database::new());
        let inner = Controller::new(event_database, tel_database);

        let identifiers_db = Arc::new(Database::new());

        Ok(Self {
            inner,
            db: identifiers_db,
        })
    }

    pub fn load_identifier(
        &self,
        alias: String,
    ) -> Result<JsIdentifier, WasmError> {
        let id_record = self
            .db
            .get_identifier(&alias)
            .ok_or_else(|| WasmError::IdentifierNotFound(alias.clone()))?;

        let identifier = self
            .inner
            .load_identifier(&id_record.said)
            .map_err(WasmError::Controller)?;
        let signer = Signer::new_with_seed(&id_record.seed)
            .map_err(|e| WasmError::Signing(e.to_string()))?;

        Ok(JsIdentifier::new(alias, identifier, Arc::new(signer), self.db.clone(), id_record.watcher_oobi))
    }

    pub fn get_identifier_aliases(&self) -> Result<Vec<JsValue>, WasmError> {
        let aliases: Vec<JsValue> = self
            .db
            .get_identifiers()
            .iter()
            .map(|(alias, _)| JsValue::from_str(alias))
            .collect();
        Ok(aliases)
    }

    pub fn incept(&self) -> Result<JsIdentifier, WasmError> {
        let keys = KeysConfig::default();
        let (next_pub_key, _next_secret_keys) =
            keys.next.derive_key_pair().map_err(|e| {
                WasmError::Signing(format!("failed to derive keys: {}", e))
            })?;

        let signer = Signer::new_with_seed(&keys.current)
            .map(Arc::new)
            .map_err(|e| WasmError::Signing(e.to_string()))?;

        let next_pub_keys = vec![BasicPrefix::Ed25519NT(next_pub_key)];
        let public_keys = vec![BasicPrefix::Ed25519(signer.public_key())];

        let signing_inception =
            self.inner.incept(public_keys, next_pub_keys).map_err(|()| {
                WasmError::Controller(
                    "inception event generation failed".to_string(),
                )
            })?;
        let signature = signer
            .sign(signing_inception.as_bytes())
            .map_err(|e| WasmError::Signing(e.to_string()))?;
        let signature = SelfSigningPrefix::new(
            cesrox::primitives::codes::self_signing::SelfSigning::Ed25519Sha512,
            signature,
        );
        let signing_identifier = self
            .inner
            .finalize_incept(signing_inception.as_bytes(), &signature)
            .map_err(|()| {
                WasmError::Controller("inception finalization failed".to_string())
            })?;

        let kel = signing_identifier.get_own_kel().ok_or_else(|| {
            WasmError::IdentifierNotFound(
                signing_identifier.get_prefix().to_string(),
            )
        })?;
        self.process_kel(format!("{:?}", kel), None, None)?;

        let prefix = signing_identifier.get_prefix();
        let alias = prefix.to_string();
        self.db.add_identifier(&alias, &prefix.clone(), &keys.current)?;

        Ok(JsIdentifier::new(alias, signing_identifier, signer.clone(), self.db.clone(), None))
    }

    pub fn process_kel(
        &self,
        kel: String,
        from: Option<u64>,
        limit: Option<u64>,
    ) -> Result<(), WasmError> {
        let mut parsed_kel: Vec<Message> = parse_event_stream(kel.as_bytes())
            .map_err(|e| {
                WasmError::Cesr(format!("failed to parse KEL: {}", e))
            })?;
        if let Some(from) = from {
            parsed_kel = parsed_kel
                .into_iter()
                .skip(from as usize)
                .collect();
        }
        if let Some(limit) = limit {
            parsed_kel = parsed_kel
                .into_iter()
                .take(limit as usize + 1)
                .collect();
        }

        self.inner
            .process_kel(&parsed_kel)
            .map_err(WasmError::Controller)?;

        Ok(())
    }

    pub fn process_tel(&self, tel: String) -> Result<(), WasmError> {
        self.inner
            .process_tel(tel.as_bytes())
            .map_err(WasmError::Controller)?;

        Ok(())
    }

    pub fn get_vc_state(&self, prefix: String) -> Result<VcState, WasmError> {
        let said: SelfAddressingIdentifier = prefix.parse().map_err(|e| {
            WasmError::InvalidInput(format!("invalid prefix: {}", e))
        })?;

        let state = self
            .inner
            .get_vc_state(&said)
            .map_err(WasmError::Controller)?;
        Ok(match state {
            Some(TelState::Issued(_)) => VcState::Issued,
            Some(TelState::Revoked) => VcState::Revoked,
            None | Some(TelState::NotIssued) => VcState::NotIssued,
        })
    }

    pub async fn verify(
        &self,
        identifier: &JsIdentifier,
        oobi_array: JsValue,
        message: String,
    ) -> Result<JsValue, WasmError> {
        let (_rest, cesr) = cesrox::parse(message.as_bytes()).map_err(|e| {
            WasmError::Cesr(format!("failed to parse CESR: {}", e))
        })?;
        let att: acdc::Attestation = match cesr.payload {
            cesrox::payload::Payload::JSON(items) => {
                serde_json::from_slice(&items)?
            }
            cesrox::payload::Payload::CBOR(items) => {
                serde_cbor::from_slice(&items)?
            }
            cesrox::payload::Payload::MGPK(_items) => {
                return Err(WasmError::UnsupportedPayload("MGPK"))
            }
        };

        let vc_said = att.digest.clone().ok_or(WasmError::MissingSaid)?;
        let current_vc_state = self.get_vc_state(vc_said.to_string())?;
        if let VcState::Revoked = current_vc_state {
            let result: VerificationResult = VcState::Revoked.into();
            return Ok(result.into());
        }

        let oobis: Vec<Oobi> = if oobi_array.is_null()
            || oobi_array.is_undefined()
        {
            vec![]
        } else {
            serde_wasm_bindgen::from_value(oobi_array).map_err(|e| {
                WasmError::InvalidInput(format!("invalid OOBI array: {}", e))
            })?
        };
        let watcher_url = identifier
            .watcher_oobi
            .clone()
            .ok_or(WasmError::WatcherNotSet)?
            .url;
        self.resolve_oobis(&watcher_url.to_string(), oobis.clone())
            .await?;

        let issuer_id: IdentifierPrefix = att.issuer.parse().map_err(|e| {
            WasmError::InvalidInput(format!("failed to parse issuer ID: {}", e))
        })?;
        let current_state = self
            .inner
            .get_state(&issuer_id);
        let current_sn = current_state.clone().map(|s| s.sn);
        let kel = self.query_kel(identifier, issuer_id, current_sn).await?;
        let skip_first = current_state.as_ref().map(|_| 1);
        self.process_kel(kel, skip_first, None)?;

        let tel = self.query_tel(identifier, att.clone()).await?;
        self.process_tel(tel)?;

        let vc_state = self.get_vc_state(vc_said.to_string())?;
        let result: VerificationResult = vc_state.into();
        Ok(result.into())
    }
}

impl JsController {
    async fn resolve_oobis(
        &self,
        watcher_url: &str,
        oobis: Vec<Oobi>,
    ) -> Result<(), WasmError> {
        for oobi in oobis {
            Request::post(&format!("{}resolve", watcher_url))
                .header("Content-Type", "application/json")
                .body(serde_json::to_string(&oobi)?)?
                .send()
                .await?;
        }
        Ok(())
    }

    async fn query_kel(
        &self,
        signing_id: &JsIdentifier,
        id: IdentifierPrefix,
        from_sn: Option<u64>,
    ) -> Result<String, WasmError> {
        let watcher_oobi = signing_id
            .watcher_oobi
            .clone()
            .ok_or(WasmError::WatcherNotSet)?;
        let watcher_url = watcher_oobi.url;
        let watcher_id = watcher_oobi.eid;
        let qry = signing_id.inner.get_log_query(id, watcher_id, from_sn, None);
        let signer = signing_id.signer.clone();

        let encoded_qry = qry
            .encode()
            .map_err(|e| WasmError::Cesr(e.to_string()))?;
        let signature = signer
            .sign(encoded_qry)
            .map_err(|e| WasmError::Signing(e.to_string()))?;
        let sig = SelfSigningPrefix::new(
            cesrox::primitives::codes::self_signing::SelfSigning::Ed25519Sha512,
            signature,
        );
        let signatures = vec![IndexedSignature::new_both_same(sig, 0)];
        let singed_kel_qry = SignedKelQuery::new_trans(
            qry.clone(),
            signing_id.inner.get_prefix().clone(),
            signatures,
        );

        let mut delay = std::time::Duration::from_secs(1);
        let mut kel = "".to_string();
        for _i in 0..5 {
            let signed_qry =
                SignedQueryMessage::KelQuery(singed_kel_qry.clone());

            let body_msg = Message::Op(Op::Query(signed_qry))
                .to_cesr()
                .map_err(|e| WasmError::Cesr(e.to_string()))?;
            let body = js_sys::Uint8Array::from(body_msg.as_slice());
            let response = Request::post(watcher_url.join("query")?.as_str())
                .header("Content-Type", "application/json")
                .body(&body)?
                .send()
                .await?;

            let code = response.status();
            if code == 200 {
                kel = response.text().await?;
                break;
            } else {
                gloo_timers::future::TimeoutFuture::new(
                    delay.as_millis() as u32
                )
                .await;
                delay *= 2;
            }
        }

        Ok(kel)
    }

    async fn query_tel(
        &self,
        id: &JsIdentifier,
        acdc_attestation: acdc::Attestation,
    ) -> Result<String, WasmError> {
        let watcher_url = id
            .watcher_oobi
            .clone()
            .ok_or(WasmError::WatcherNotSet)?
            .url;
        let vc_said = acdc_attestation.digest.ok_or(WasmError::MissingSaid)?;
        let registry_id: said::SelfAddressingIdentifier = acdc_attestation
            .registry_identifier
            .parse()
            .map_err(|e| {
                WasmError::InvalidInput(format!(
                    "invalid registry identifier: {}",
                    e
                ))
            })?;
        let signer = id.signer.clone();

        let tel_qry = id
            .inner
            .get_tel_query(
                IdentifierPrefix::SelfAddressing(registry_id.into()),
                IdentifierPrefix::SelfAddressing(vc_said.clone().into()),
            )
            .map_err(WasmError::Controller)?;

        let encoded_qry = tel_qry
            .encode()
            .map_err(|e| WasmError::Cesr(e.to_string()))?;
        let signature = signer
            .sign(encoded_qry)
            .map_err(|e| WasmError::Signing(e.to_string()))?;
        let signature_tel_query = SelfSigningPrefix::new(
            cesrox::primitives::codes::self_signing::SelfSigning::Ed25519Sha512,
            signature,
        );

        let tel_query = match &id.inner.id {
            IdentifierPrefix::Basic(bp) => {
                teliox::query::SignedTelQuery::new_nontrans(
                    tel_qry.clone(),
                    bp.clone(),
                    signature_tel_query,
                )
            }
            _ => {
                let signatures =
                    vec![keri_core::prefix::IndexedSignature::new_both_same(
                        signature_tel_query,
                        0,
                    )];
                teliox::query::SignedTelQuery::new_trans(
                    tel_qry.clone(),
                    id.inner.id.clone(),
                    signatures,
                )
            }
        };

        let mut delay = std::time::Duration::from_secs(1);
        let mut tel = "".to_string();
        for _i in 0..5 {
            let body_msg = tel_query
                .to_cesr()
                .map_err(|e| WasmError::Cesr(e.to_string()))?;
            let body = js_sys::Uint8Array::from(body_msg.as_slice());
            let response =
                Request::post(watcher_url.join("query/tel")?.as_str())
                    .header("Content-Type", "application/json")
                    .body(&body)?
                    .send()
                    .await?;

            let code = response.status();
            if code == 200 {
                tel = response.text().await?;
                break;
            } else {
                gloo_timers::future::TimeoutFuture::new(
                    delay.as_millis() as u32
                )
                .await;
                delay *= 2;
            }
        }

        Ok(tel)
    }
}

pub struct VerificationResult {
    verified: bool,
    status: String,
}

impl From<VerificationResult> for JsValue {
    fn from(val: VerificationResult) -> Self {
        let obj = js_sys::Object::new();

        // Reflect::set on a freshly created object cannot fail — these
        // expects guard an invariant, not fallible input.
        js_sys::Reflect::set(&obj, &JsValue::from_str("verified"), &JsValue::from_bool(val.verified))
            .expect("setting verified failed");

        js_sys::Reflect::set(&obj, &JsValue::from_str("status"), &JsValue::from_str(&val.status))
            .expect("setting status failed");

        obj.into()
    }
}

impl From<VcState> for VerificationResult {
    fn from(val: VcState) -> Self {
        match val {
            VcState::Issued => VerificationResult {
                verified: true,
                status: "issued".to_string(),
            },
            VcState::Revoked => VerificationResult {
                verified: false,
                status: "revoked".to_string(),
            },
            VcState::NotIssued => VerificationResult {
                verified: false,
                status: "not issued".to_string(),
            },
        }
    }
}
