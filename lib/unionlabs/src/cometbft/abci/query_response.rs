use macros::model;

use crate::{
    bounded::{BoundedI64, BoundedIntError},
    cometbft::crypto::proof_ops::ProofOps,
};

#[model(proto(raw(protos::cometbft::abci::v1::QueryResponse), into, from))]
pub struct QueryResponse {
    pub code: u32,
    /// nondeterministic
    pub log: String,
    /// nondeterministic
    pub info: String,
    pub index: i64,
    #[serde(with = "::serde_utils::hex_string")]
    #[debug(wrap = ::serde_utils::fmt::DebugAsHex)]
    pub key: Vec<u8>,
    #[serde(with = "::serde_utils::hex_string")]
    #[debug(wrap = ::serde_utils::fmt::DebugAsHex)]
    pub value: Vec<u8>,
    pub proof_ops: Option<ProofOps>,
    pub height: BoundedI64<0, { i64::MAX }>,
    pub codespace: String,
}

impl From<QueryResponse> for protos::cometbft::abci::v1::QueryResponse {
    fn from(value: QueryResponse) -> Self {
        Self {
            code: value.code,
            log: value.log,
            info: value.info,
            index: value.index,
            key: value.key,
            value: value.value,
            proof_ops: value.proof_ops.map(Into::into),
            height: value.height.inner(),
            codespace: value.codespace,
        }
    }
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum TryFromQueryResponseError {
    #[error("invalid height")]
    Height(#[source] BoundedIntError<i64>),
}

impl TryFrom<protos::cometbft::abci::v1::QueryResponse> for QueryResponse {
    type Error = TryFromQueryResponseError;

    fn try_from(value: protos::cometbft::abci::v1::QueryResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            code: value.code,
            log: value.log,
            info: value.info,
            index: value.index,
            key: value.key,
            value: value.value,
            proof_ops: value.proof_ops.map(Into::into),
            height: value
                .height
                .try_into()
                .map_err(TryFromQueryResponseError::Height)?,
            codespace: value.codespace,
        })
    }
}
