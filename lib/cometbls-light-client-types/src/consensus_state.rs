use unionlabs::{
    hash::{hash_v2::HexUnprefixed, H256},
    ibc::core::commitment::merkle_root::MerkleRoot,
};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ConsensusState {
    pub timestamp: u64,
    pub app_hash: MerkleRoot,
    pub next_validators_hash: H256<HexUnprefixed>,
}

#[cfg(feature = "proto")]
pub mod proto {
    use unionlabs::{
        errors::{required, InvalidLength, MissingField},
        ibc::core::commitment::merkle_root::TryFromMerkleRootError,
        impl_proto_via_try_from_into,
    };

    use crate::consensus_state::ConsensusState;

    impl_proto_via_try_from_into!(ConsensusState => protos::union::ibc::lightclients::cometbls::v1::ConsensusState);

    impl TryFrom<protos::union::ibc::lightclients::cometbls::v1::ConsensusState> for ConsensusState {
        type Error = Error;

        fn try_from(
            value: protos::union::ibc::lightclients::cometbls::v1::ConsensusState,
        ) -> Result<Self, Self::Error> {
            Ok(Self {
                timestamp: value.timestamp,
                app_hash: required!(value.root)?.try_into().map_err(Error::Root)?,
                next_validators_hash: value
                    .next_validators_hash
                    .try_into()
                    .map_err(Error::NextValidatorsHash)?,
            })
        }
    }

    #[derive(Debug, PartialEq, Clone, thiserror::Error)]
    pub enum Error {
        #[error(transparent)]
        MissingField(#[from] MissingField),
        #[error("invalid root")]
        Root(#[from] TryFromMerkleRootError),
        #[error("invalid next validators hash")]
        NextValidatorsHash(#[from] InvalidLength),
    }

    impl From<ConsensusState> for protos::union::ibc::lightclients::cometbls::v1::ConsensusState {
        fn from(value: ConsensusState) -> Self {
            Self {
                timestamp: value.timestamp,
                root: Some(value.app_hash.into()),
                next_validators_hash: value.next_validators_hash.into(),
            }
        }
    }
}

#[cfg(feature = "ethabi")]
pub mod ethabi {
    use alloy::sol_types::SolValue;
    use unionlabs::impl_ethabi_via_try_from_into;

    use super::*;

    impl_ethabi_via_try_from_into!(ConsensusState => SolConsensusState);

    alloy::sol! {
        struct SolConsensusState {
            uint64 timestamp;
            bytes32 app_hash;
            bytes32 next_validators_hash;
        }
    }

    impl From<ConsensusState> for SolConsensusState {
        fn from(value: ConsensusState) -> Self {
            Self {
                timestamp: value.timestamp,
                app_hash: value.app_hash.hash.get().into(),
                next_validators_hash: value.next_validators_hash.get().into(),
            }
        }
    }

    impl From<SolConsensusState> for ConsensusState {
        fn from(value: SolConsensusState) -> Self {
            Self {
                timestamp: value.timestamp,
                app_hash: MerkleRoot {
                    hash: H256::new(value.app_hash.0),
                },
                next_validators_hash: H256::new(value.next_validators_hash.0),
            }
        }
    }
}

#[test]
fn encode_consensus() {
    let client_state = ConsensusState {
        timestamp: 20000,
        app_hash: MerkleRoot {
            hash: H256::new([
                0, 1, 2, 3, 4, 5, 6, 7, 0, 1, 2, 3, 4, 5, 6, 7, 0, 1, 2, 3, 4, 5, 6, 7, 0, 1, 2, 3,
                4, 5, 6, 7,
            ]),
        },
        next_validators_hash: H256::new([
            0, 1, 2, 3, 4, 5, 6, 7, 0, 1, 2, 3, 4, 5, 6, 7, 0, 1, 2, 3, 4, 5, 6, 7, 0, 1, 2, 3, 4,
            5, 6, 7,
        ]),
    };

    let mut buf = Vec::new();

    bcs::serialize_into(&mut buf, &client_state).unwrap();
    panic!("fuck {:?}", hex::encode(buf));
}
