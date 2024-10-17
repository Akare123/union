use macros::model;

#[cfg(feature = "ethabi")]
use crate::cometbft::types::{commit::TryFromEthAbiCommitError, header::TryFromEthAbiHeaderError};
use crate::{
    cometbft::types::{
        commit::{Commit, TryFromCommitError},
        header::{Header, TryFromHeaderError},
    },
    errors::{required, MissingField},
};

#[model(proto(raw(protos::cometbft::types::v1::SignedHeader), into, from))]
pub struct SignedHeader {
    pub header: Header,
    pub commit: Commit,
}

impl From<SignedHeader> for protos::cometbft::types::v1::SignedHeader {
    fn from(value: SignedHeader) -> Self {
        Self {
            header: Some(value.header.into()),
            commit: Some(value.commit.into()),
        }
    }
}

impl From<SignedHeader> for protos::tendermint::types::SignedHeader {
    fn from(value: SignedHeader) -> Self {
        Self {
            header: Some(value.header.into()),
            commit: Some(value.commit.into()),
        }
    }
}

#[cfg(feature = "ethabi")]
#[derive(Debug, Clone, PartialEq)]
pub enum TryFromEthAbiSignedHeaderError {
    Header(TryFromEthAbiHeaderError),
    Commit(TryFromEthAbiCommitError),
}

#[cfg(feature = "ethabi")]
impl TryFrom<contracts::glue::TendermintTypesSignedHeaderData> for SignedHeader {
    type Error = TryFromEthAbiSignedHeaderError;

    fn try_from(
        value: contracts::glue::TendermintTypesSignedHeaderData,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            header: value
                .header
                .try_into()
                .map_err(TryFromEthAbiSignedHeaderError::Header)?,
            commit: value
                .commit
                .try_into()
                .map_err(TryFromEthAbiSignedHeaderError::Commit)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum TryFromSignedHeaderError {
    #[error(transparent)]
    MissingField(#[from] MissingField),
    #[error("invalid header")]
    Header(#[source] TryFromHeaderError),
    #[error("invalid commit")]
    Commit(#[source] TryFromCommitError),
}

impl TryFrom<protos::cometbft::types::v1::SignedHeader> for SignedHeader {
    type Error = TryFromSignedHeaderError;

    fn try_from(value: protos::cometbft::types::v1::SignedHeader) -> Result<Self, Self::Error> {
        Ok(Self {
            header: required!(value.header)?
                .try_into()
                .map_err(TryFromSignedHeaderError::Header)?,
            commit: required!(value.commit)?
                .try_into()
                .map_err(TryFromSignedHeaderError::Commit)?,
        })
    }
}

impl TryFrom<protos::tendermint::types::SignedHeader> for SignedHeader {
    type Error = TryFromSignedHeaderError;

    fn try_from(value: protos::tendermint::types::SignedHeader) -> Result<Self, Self::Error> {
        Ok(Self {
            header: required!(value.header)?
                .try_into()
                .map_err(TryFromSignedHeaderError::Header)?,
            commit: required!(value.commit)?
                .try_into()
                .map_err(TryFromSignedHeaderError::Commit)?,
        })
    }
}

#[cfg(feature = "ethabi")]
impl From<SignedHeader> for contracts::glue::TendermintTypesSignedHeaderData {
    fn from(value: SignedHeader) -> Self {
        Self {
            header: value.header.into(),
            commit: value.commit.into(),
        }
    }
}
