use cosmwasm_std::{Deps, Empty};
use ics008_wasm_client::{
    storage_utils::{
        read_client_state, read_consensus_state, save_client_state, save_consensus_state,
    },
    IbcClient, Status,
};
use near_primitives_core::hash::CryptoHash;
use unionlabs::{
    encoding::{DecodeAs, EncodeAs, Proto},
    google::protobuf::any::Any,
    ibc::{
        core::client::height::Height,
        lightclients::{
            cometbls,
            near::{
                client_state::ClientState, consensus_state::ConsensusState, header::Header,
                validator_stake_view::ValidatorStakeView,
            },
            wasm,
        },
    },
    ics24::Path,
    id::ClientId,
    near::raw_state_proof::RawStateProof,
};

use crate::{errors::Error, state::EPOCH_BLOCK_PRODUCERS_MAP};

pub type WasmClientState = wasm::client_state::ClientState<ClientState>;
pub type WasmConsensusState = wasm::consensus_state::ConsensusState<ConsensusState>;

pub struct NearLightClient;

impl IbcClient for NearLightClient {
    type Error = Error;

    type CustomQuery = Empty;

    type Header = Header;

    type Misbehaviour = Header;

    type ClientState = ClientState;

    type ConsensusState = ConsensusState;

    type Encoding = Proto;

    fn verify_membership(
        deps: Deps<Self::CustomQuery>,
        mut height: Height,
        _delay_time_period: u64,
        _delay_block_period: u64,
        proof: Vec<u8>,
        path: unionlabs::ibc::core::commitment::merkle_path::MerklePath,
        value: ics008_wasm_client::StorageState,
    ) -> Result<(), ics008_wasm_client::IbcClientError<Self>> {
        let proof: RawStateProof = serde_json_wasm::from_slice(&proof).unwrap();
        height.revision_height += 1;
        let consensus_state: WasmConsensusState = read_consensus_state(deps, &height)?
            .ok_or(Error::ConsensusStateNotFound(height.revision_height))?;
        let client_state: WasmClientState = read_client_state(deps)?;
        let key = key_from_path(path.key_path.last().ok_or(Error::EmptyPath)?);

        match value {
            ics008_wasm_client::StorageState::Occupied(value) => {
                let path = path.key_path.last().ok_or(Error::EmptyPath)?;
                let path = path
                    .parse::<Path<ClientId, Height>>()
                    .map_err(|_| Error::UnknownIbcPath(path.clone()))?;

                let value = match path {
                    Path::ClientState(_) => {
                        Any::<cometbls::client_state::ClientState>::decode_as::<Proto>(
                            value.as_ref(),
                        )
                        .map_err(|_| Error::ForeignStateDecode(value))?
                        .0
                        .encode_as::<Proto>()
                    }
                    Path::ClientConsensusState(_) => {
                        Any::<
                            wasm::consensus_state::ConsensusState<
                                cometbls::consensus_state::ConsensusState,
                            >,
                        >::decode_as::<Proto>(value.as_ref())
                        .map_err(|_| Error::ForeignStateDecode(value))?
                        .0
                        .data
                        .encode_as::<Proto>()
                    }
                    _ => value,
                };
                near_verifier::verify_state(
                    proof,
                    &consensus_state.data.chunk_prev_state_root,
                    &client_state.data.ibc_account_id,
                    &key,
                    Some(&borsh::to_vec(&value).unwrap()),
                )
            }
            ics008_wasm_client::StorageState::Empty => near_verifier::verify_state(
                proof,
                &consensus_state.data.chunk_prev_state_root,
                &client_state.data.ibc_account_id,
                &key,
                None,
            ),
        }
        .map_err(Into::<Error>::into)?;

        Ok(())
    }

    fn verify_header(
        deps: Deps<Self::CustomQuery>,
        _env: cosmwasm_std::Env,
        header: Self::Header,
    ) -> Result<(), ics008_wasm_client::IbcClientError<Self>> {
        let wasm_consensus_state = read_consensus_state(deps, &height(header.trusted_height))?
            .ok_or(Error::ConsensusStateNotFound(header.trusted_height))?;

        near_verifier::verify_header(
            &NearVerifierCtx { deps },
            wasm_consensus_state.data.state,
            header.new_state.clone(),
        )
        .map_err(Into::<Error>::into)?;

        // verify the `prev_state_root` of the chunk that contains the light client against the merkle root of the `prev_state_root`s of all chunks
        near_verifier::verify_path(
            header.new_state.inner_lite.prev_state_root,
            &header.prev_state_root_proof,
            header.prev_state_root,
        )
        .map_err(Into::<Error>::into)?;

        Ok(())
    }

    fn verify_misbehaviour(
        _deps: Deps<Self::CustomQuery>,
        _env: cosmwasm_std::Env,
        _misbehaviour: Self::Misbehaviour,
    ) -> Result<(), ics008_wasm_client::IbcClientError<Self>> {
        unimplemented!()
    }

    fn update_state(
        mut deps: cosmwasm_std::DepsMut<Self::CustomQuery>,
        _env: cosmwasm_std::Env,
        header: Self::Header,
    ) -> Result<Vec<Height>, ics008_wasm_client::IbcClientError<Self>> {
        let update_height = header.new_state.inner_lite.height;

        let new_consensus_state = ConsensusState {
            state: header.new_state.inner_lite.clone(),
            chunk_prev_state_root: header.prev_state_root,
            timestamp: header.new_state.inner_lite.timestamp_nanosec,
        };

        save_consensus_state::<NearLightClient>(
            deps.branch(),
            WasmConsensusState {
                data: new_consensus_state,
            },
            &height(update_height),
        );

        let mut client_state: WasmClientState = read_client_state(deps.as_ref())?;

        if update_height > client_state.data.latest_height {
            client_state.data.latest_height = update_height;
            client_state.latest_height.revision_height = update_height;
            save_client_state::<NearLightClient>(deps.branch(), client_state);
        }

        if let Some(next_bps) = header.new_state.next_bps {
            EPOCH_BLOCK_PRODUCERS_MAP.save(
                deps.storage,
                header.new_state.inner_lite.next_epoch_id.0,
                &next_bps,
            )?;
        }

        Ok(vec![height(update_height)])
    }

    fn update_state_on_misbehaviour(
        _deps: cosmwasm_std::DepsMut<Self::CustomQuery>,
        _env: cosmwasm_std::Env,
        _client_message: Vec<u8>,
    ) -> Result<(), ics008_wasm_client::IbcClientError<Self>> {
        unimplemented!()
    }

    fn check_for_misbehaviour_on_header(
        _deps: Deps<Self::CustomQuery>,
        _header: Self::Header,
    ) -> Result<bool, ics008_wasm_client::IbcClientError<Self>> {
        Ok(false)
    }

    fn check_for_misbehaviour_on_misbehaviour(
        _deps: Deps<Self::CustomQuery>,
        _misbehaviour: Self::Misbehaviour,
    ) -> Result<bool, ics008_wasm_client::IbcClientError<Self>> {
        unimplemented!()
    }

    fn verify_upgrade_and_update_state(
        _deps: cosmwasm_std::DepsMut<Self::CustomQuery>,
        _upgrade_client_state: Self::ClientState,
        _upgrade_consensus_state: Self::ConsensusState,
        _proof_upgrade_client: Vec<u8>,
        _proof_upgrade_consensus_state: Vec<u8>,
    ) -> Result<(), ics008_wasm_client::IbcClientError<Self>> {
        todo!()
    }

    fn migrate_client_store(
        _deps: cosmwasm_std::DepsMut<Self::CustomQuery>,
    ) -> Result<(), ics008_wasm_client::IbcClientError<Self>> {
        todo!()
    }

    fn status(
        deps: Deps<Self::CustomQuery>,
        _env: &cosmwasm_std::Env,
    ) -> Result<Status, ics008_wasm_client::IbcClientError<Self>> {
        let client_state: WasmClientState = read_client_state(deps)?;

        if client_state.data.frozen_height != 0 {
            return Ok(Status::Frozen);
        }

        Ok(Status::Active)
    }

    fn export_metadata(
        _deps: Deps<Self::CustomQuery>,
        _env: &cosmwasm_std::Env,
    ) -> Result<
        Vec<unionlabs::ibc::core::client::genesis_metadata::GenesisMetadata>,
        ics008_wasm_client::IbcClientError<Self>,
    > {
        unimplemented!()
    }

    fn timestamp_at_height(
        deps: Deps<Self::CustomQuery>,
        height: Height,
    ) -> Result<u64, ics008_wasm_client::IbcClientError<Self>> {
        Ok(read_consensus_state::<Self>(deps, &height)?
            .ok_or(Error::ConsensusStateNotFound(height.revision_height))?
            .data
            .timestamp)
    }
}

pub struct NearVerifierCtx<'a> {
    deps: Deps<'a>,
}

impl<'a> near_verifier::NearVerifierCtx for NearVerifierCtx<'a> {
    fn get_epoch_block_producers(&self, epoch_id: CryptoHash) -> Option<Vec<ValidatorStakeView>> {
        match EPOCH_BLOCK_PRODUCERS_MAP.load(self.deps.storage, epoch_id.0) {
            Ok(bp) => Some(bp),
            Err(_) => None,
        }
    }

    fn ed25519_verify(
        &self,
        public_key: &[u8],
        signature: &[u8],
        message: &[u8],
    ) -> Result<(), near_verifier::error::Error> {
        match self.deps.api.ed25519_verify(message, signature, public_key) {
            Ok(true) => Ok(()),
            _ => Err(near_verifier::error::Error::VerificationFailure(
                public_key.into(),
                signature.into(),
                message.into(),
            )),
        }
    }
}

fn height(height: u64) -> Height {
    Height {
        revision_number: 0,
        revision_height: height,
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use ics008_wasm_client::InstantiateMsg;
    use near_jsonrpc_client::{methods, JsonRpcClient};
    use near_primitives::{
        types::{BlockHeight, BlockId, BlockReference},
        views::{BlockHeaderInnerLiteView, LightClientBlockView},
    };
    use unionlabs::{
        encoding::{DecodeAs, EncodeAs},
        ibc::{core::connection::connection_end::ConnectionEnd, lightclients::near},
        near::types::{self, MerklePathItem},
    };

    use super::*;
    use crate::contract::instantiate;

    async fn initialize() -> (
        near_jsonrpc_client::JsonRpcClient,
        cosmwasm_std::OwnedDeps<
            cosmwasm_std::MemoryStorage,
            cosmwasm_std::testing::MockApi,
            cosmwasm_std::testing::MockQuerier,
        >,
        cosmwasm_std::Env,
        cosmwasm_std::MessageInfo,
        ClientState,
        ConsensusState,
    ) {
        let rpc = near_jsonrpc_client::JsonRpcClient::connect("http://localhost:3030");
        let chain_id = rpc
            .call(methods::status::RpcStatusRequest)
            .await
            .unwrap()
            .chain_id;

        let block = rpc
            .call(methods::block::RpcBlockRequest {
                block_reference: BlockReference::Finality(near_primitives::types::Finality::Final),
            })
            .await
            .unwrap();

        let validators = rpc
            .call(
                methods::EXPERIMENTAL_validators_ordered::RpcValidatorsOrderedRequest {
                    block_id: Some(BlockId::Height(block.header.height)),
                },
            )
            .await
            .unwrap();

        println!("chain id {:?}", chain_id);

        let chunk_prev_state_root = block.header.prev_state_root;
        let timestamp = block.header.timestamp_nanosec;

        (
            rpc,
            mock_dependencies(),
            mock_env(),
            mock_info("thisaddr", &[]),
            ClientState {
                chain_id: "hello".to_string(),
                latest_height: block.header.height - 1,
                ibc_account_id: "acc.near".to_string().try_into().unwrap(),
                // TODO(aeryz): this is only valid in this sandboxed environment where the validator set is not changing. For a real environment,
                // the relayer must read the block producers using another endpoint.
                initial_block_producers: convert_block_producers(validators),
                frozen_height: 0,
            },
            ConsensusState {
                state: block_header_to_inner_lite(block.header),
                chunk_prev_state_root,
                timestamp,
            },
        )
    }

    #[test]
    fn test_verify_membership() {
        let proof = [
            123, 34, 115, 116, 97, 116, 101, 95, 112, 114, 111, 111, 102, 34, 58, 91, 91, 51, 44,
            49, 44, 48, 44, 48, 44, 48, 44, 49, 54, 44, 49, 49, 44, 57, 51, 44, 49, 48, 50, 44, 54,
            49, 44, 49, 48, 56, 44, 49, 52, 57, 44, 49, 54, 50, 44, 50, 50, 55, 44, 51, 55, 44, 49,
            54, 57, 44, 49, 49, 53, 44, 49, 49, 49, 44, 49, 48, 44, 56, 52, 44, 51, 52, 44, 55, 54,
            44, 51, 50, 44, 57, 52, 44, 49, 48, 49, 44, 52, 44, 49, 53, 54, 44, 49, 53, 51, 44, 57,
            56, 44, 49, 54, 56, 44, 50, 50, 52, 44, 55, 48, 44, 50, 49, 52, 44, 49, 57, 55, 44, 55,
            44, 57, 49, 44, 49, 53, 50, 44, 50, 48, 48, 44, 53, 55, 44, 49, 48, 53, 44, 49, 49, 44,
            48, 44, 48, 44, 48, 44, 48, 44, 48, 93, 44, 91, 49, 44, 55, 44, 50, 44, 50, 50, 53, 44,
            50, 51, 56, 44, 55, 48, 44, 49, 51, 52, 44, 49, 44, 57, 55, 44, 49, 56, 53, 44, 57, 52,
            44, 49, 57, 50, 44, 49, 54, 49, 44, 51, 49, 44, 50, 44, 50, 48, 44, 49, 51, 55, 44, 52,
            53, 44, 50, 54, 44, 54, 48, 44, 56, 48, 44, 49, 50, 52, 44, 49, 55, 54, 44, 49, 57, 44,
            49, 51, 44, 49, 52, 55, 44, 54, 44, 49, 48, 54, 44, 49, 50, 54, 44, 52, 52, 44, 50, 54,
            44, 49, 50, 49, 44, 57, 55, 44, 52, 50, 44, 49, 56, 53, 44, 49, 51, 56, 44, 54, 48, 44,
            50, 48, 48, 44, 53, 52, 44, 50, 50, 53, 44, 49, 56, 49, 44, 49, 54, 56, 44, 52, 51, 44,
            49, 49, 56, 44, 53, 49, 44, 54, 44, 50, 52, 54, 44, 49, 52, 56, 44, 50, 51, 52, 44, 57,
            55, 44, 50, 50, 51, 44, 49, 56, 49, 44, 57, 54, 44, 49, 56, 49, 44, 53, 57, 44, 53, 52,
            44, 49, 48, 55, 44, 49, 51, 49, 44, 55, 51, 44, 50, 49, 54, 44, 49, 50, 48, 44, 49, 55,
            49, 44, 50, 50, 44, 49, 50, 57, 44, 49, 49, 54, 44, 49, 48, 49, 44, 55, 55, 44, 51, 57,
            44, 50, 48, 48, 44, 49, 53, 57, 44, 49, 49, 57, 44, 49, 54, 49, 44, 50, 53, 53, 44, 49,
            56, 51, 44, 50, 54, 44, 50, 51, 50, 44, 49, 44, 55, 51, 44, 57, 57, 44, 50, 49, 55, 44,
            50, 51, 50, 44, 52, 57, 44, 50, 48, 55, 44, 50, 50, 52, 44, 50, 50, 57, 44, 49, 57, 48,
            44, 50, 52, 57, 44, 50, 51, 57, 44, 49, 53, 56, 44, 49, 50, 54, 44, 54, 54, 44, 49, 53,
            54, 44, 50, 48, 51, 44, 49, 53, 44, 50, 51, 56, 44, 50, 52, 53, 44, 49, 54, 54, 44, 50,
            48, 52, 44, 50, 53, 51, 44, 49, 48, 53, 44, 56, 50, 44, 51, 51, 44, 50, 48, 57, 44, 49,
            50, 48, 44, 49, 56, 44, 49, 56, 53, 44, 49, 48, 50, 44, 50, 52, 54, 44, 51, 52, 44, 49,
            50, 53, 44, 56, 52, 44, 57, 44, 50, 52, 51, 44, 50, 53, 44, 55, 57, 44, 49, 56, 53, 44,
            49, 50, 54, 44, 50, 50, 49, 44, 49, 52, 51, 44, 49, 52, 56, 44, 49, 52, 48, 44, 49, 54,
            56, 44, 50, 49, 53, 44, 49, 48, 52, 44, 55, 51, 44, 50, 52, 44, 50, 52, 52, 44, 57, 51,
            44, 49, 53, 48, 44, 49, 57, 49, 44, 49, 54, 44, 53, 44, 49, 48, 53, 44, 49, 49, 44, 48,
            44, 48, 44, 48, 44, 48, 44, 48, 93, 44, 91, 51, 44, 49, 44, 48, 44, 48, 44, 48, 44, 50,
            50, 44, 51, 52, 44, 50, 49, 54, 44, 49, 56, 50, 44, 50, 49, 49, 44, 49, 52, 52, 44, 53,
            56, 44, 54, 50, 44, 56, 49, 44, 57, 52, 44, 50, 51, 50, 44, 49, 55, 55, 44, 50, 49, 48,
            44, 53, 49, 44, 52, 49, 44, 50, 50, 50, 44, 50, 50, 49, 44, 49, 53, 55, 44, 50, 49, 52,
            44, 49, 48, 51, 44, 50, 49, 48, 44, 49, 51, 52, 44, 50, 56, 44, 57, 56, 44, 50, 49, 50,
            44, 49, 50, 57, 44, 50, 48, 51, 44, 53, 54, 44, 49, 53, 50, 44, 56, 44, 57, 56, 44, 50,
            48, 50, 44, 53, 56, 44, 49, 49, 57, 44, 49, 51, 44, 48, 44, 48, 44, 48, 44, 48, 44, 48,
            44, 48, 93, 44, 91, 49, 44, 56, 44, 50, 44, 57, 44, 50, 52, 49, 44, 51, 52, 44, 56, 55,
            44, 49, 56, 44, 49, 48, 49, 44, 49, 50, 49, 44, 49, 49, 56, 44, 49, 56, 51, 44, 56, 48,
            44, 50, 55, 44, 49, 53, 44, 50, 48, 52, 44, 49, 50, 51, 44, 50, 51, 52, 44, 50, 51, 49,
            44, 52, 52, 44, 49, 48, 57, 44, 57, 51, 44, 49, 57, 52, 44, 50, 44, 49, 51, 54, 44, 49,
            53, 51, 44, 51, 53, 44, 54, 44, 49, 57, 53, 44, 49, 53, 53, 44, 52, 49, 44, 50, 48, 53,
            44, 50, 49, 49, 44, 50, 52, 44, 54, 57, 44, 52, 54, 44, 57, 51, 44, 49, 57, 49, 44, 49,
            56, 56, 44, 55, 54, 44, 52, 55, 44, 49, 56, 54, 44, 49, 48, 48, 44, 50, 50, 44, 50, 54,
            44, 53, 54, 44, 49, 52, 44, 49, 55, 44, 49, 48, 55, 44, 49, 50, 49, 44, 50, 51, 53, 44,
            50, 50, 48, 44, 49, 52, 54, 44, 49, 52, 56, 44, 50, 49, 57, 44, 56, 56, 44, 51, 49, 44,
            49, 49, 57, 44, 51, 55, 44, 50, 53, 44, 49, 51, 53, 44, 49, 53, 48, 44, 49, 53, 48, 44,
            57, 56, 44, 57, 57, 44, 49, 53, 56, 44, 49, 48, 51, 44, 54, 55, 44, 49, 51, 44, 48, 44,
            48, 44, 48, 44, 48, 44, 48, 44, 48, 93, 44, 91, 51, 44, 49, 54, 44, 48, 44, 48, 44, 48,
            44, 48, 44, 57, 56, 44, 57, 57, 44, 52, 53, 44, 49, 49, 55, 44, 49, 49, 48, 44, 49, 48,
            53, 44, 49, 49, 49, 44, 49, 49, 48, 44, 52, 54, 44, 49, 49, 48, 44, 49, 49, 49, 44, 49,
            48, 48, 44, 49, 48, 49, 44, 52, 56, 44, 52, 52, 44, 53, 50, 44, 51, 53, 44, 49, 54, 53,
            44, 49, 55, 44, 49, 54, 52, 44, 49, 57, 52, 44, 50, 52, 53, 44, 49, 49, 52, 44, 49, 55,
            51, 44, 49, 53, 49, 44, 51, 56, 44, 50, 48, 55, 44, 50, 53, 48, 44, 52, 53, 44, 57, 44,
            57, 55, 44, 50, 53, 49, 44, 49, 48, 44, 52, 48, 44, 49, 52, 44, 51, 44, 54, 48, 44, 50,
            53, 50, 44, 49, 55, 49, 44, 54, 52, 44, 57, 52, 44, 50, 50, 54, 44, 50, 53, 44, 57, 55,
            44, 57, 56, 44, 49, 53, 52, 44, 50, 48, 54, 44, 50, 51, 52, 44, 49, 48, 44, 48, 44, 48,
            44, 48, 44, 48, 44, 48, 44, 48, 93, 44, 91, 49, 44, 57, 54, 44, 48, 44, 49, 53, 48, 44,
            49, 52, 54, 44, 49, 51, 53, 44, 49, 49, 54, 44, 49, 48, 50, 44, 49, 50, 44, 51, 55, 44,
            50, 54, 44, 50, 50, 49, 44, 49, 48, 55, 44, 50, 51, 54, 44, 57, 57, 44, 50, 51, 50, 44,
            50, 51, 50, 44, 49, 51, 55, 44, 50, 49, 56, 44, 51, 56, 44, 50, 48, 48, 44, 49, 49, 48,
            44, 50, 52, 55, 44, 49, 44, 50, 53, 52, 44, 50, 52, 50, 44, 49, 49, 48, 44, 51, 55, 44,
            55, 55, 44, 50, 50, 44, 50, 50, 54, 44, 49, 50, 50, 44, 50, 48, 51, 44, 52, 49, 44, 57,
            57, 44, 50, 52, 49, 44, 50, 51, 52, 44, 50, 48, 53, 44, 55, 50, 44, 49, 50, 53, 44, 57,
            57, 44, 57, 56, 44, 53, 55, 44, 49, 55, 49, 44, 50, 49, 49, 44, 50, 51, 57, 44, 49, 48,
            51, 44, 49, 49, 55, 44, 49, 57, 44, 49, 51, 55, 44, 50, 52, 53, 44, 55, 54, 44, 49, 56,
            53, 44, 49, 57, 48, 44, 49, 48, 55, 44, 50, 50, 57, 44, 51, 48, 44, 50, 52, 54, 44, 50,
            49, 48, 44, 49, 50, 51, 44, 50, 48, 54, 44, 49, 57, 52, 44, 50, 48, 57, 44, 54, 53, 44,
            50, 53, 52, 44, 49, 54, 50, 44, 50, 50, 57, 44, 49, 53, 50, 44, 49, 48, 44, 48, 44, 48,
            44, 48, 44, 48, 44, 48, 44, 48, 93, 44, 91, 49, 44, 49, 48, 44, 48, 44, 57, 44, 50, 49,
            55, 44, 49, 50, 44, 49, 54, 49, 44, 49, 53, 56, 44, 49, 48, 57, 44, 49, 55, 54, 44, 54,
            44, 50, 50, 53, 44, 49, 52, 49, 44, 52, 51, 44, 49, 48, 49, 44, 50, 50, 44, 49, 56, 50,
            44, 50, 50, 56, 44, 49, 57, 51, 44, 49, 54, 54, 44, 50, 48, 57, 44, 57, 50, 44, 54, 53,
            44, 49, 56, 49, 44, 49, 50, 54, 44, 49, 48, 53, 44, 49, 54, 53, 44, 50, 52, 48, 44, 50,
            48, 48, 44, 49, 56, 48, 44, 50, 51, 54, 44, 49, 49, 51, 44, 49, 50, 50, 44, 49, 56, 49,
            44, 50, 51, 48, 44, 49, 54, 52, 44, 49, 54, 57, 44, 57, 52, 44, 49, 51, 50, 44, 49, 52,
            52, 44, 56, 51, 44, 54, 48, 44, 50, 52, 57, 44, 50, 51, 57, 44, 50, 52, 56, 44, 49, 55,
            49, 44, 49, 56, 48, 44, 50, 48, 53, 44, 49, 53, 54, 44, 49, 49, 49, 44, 49, 55, 55, 44,
            57, 54, 44, 49, 56, 53, 44, 49, 55, 44, 49, 57, 55, 44, 57, 56, 44, 53, 55, 44, 50, 50,
            50, 44, 50, 52, 56, 44, 49, 48, 49, 44, 50, 50, 49, 44, 54, 53, 44, 55, 48, 44, 49, 55,
            44, 50, 49, 52, 44, 49, 50, 50, 44, 49, 48, 56, 44, 49, 56, 51, 44, 57, 44, 48, 44, 48,
            44, 48, 44, 48, 44, 48, 44, 48, 93, 44, 91, 51, 44, 49, 44, 48, 44, 48, 44, 48, 44, 50,
            50, 44, 53, 51, 44, 54, 54, 44, 50, 52, 48, 44, 49, 56, 49, 44, 49, 53, 50, 44, 57, 49,
            44, 49, 54, 55, 44, 49, 51, 57, 44, 50, 49, 48, 44, 49, 48, 44, 54, 44, 49, 55, 44, 51,
            57, 44, 49, 54, 54, 44, 49, 51, 48, 44, 49, 54, 50, 44, 49, 51, 54, 44, 50, 50, 56, 44,
            49, 52, 56, 44, 49, 54, 57, 44, 50, 49, 53, 44, 50, 44, 49, 55, 51, 44, 49, 57, 54, 44,
            50, 49, 54, 44, 49, 53, 50, 44, 50, 50, 56, 44, 50, 51, 51, 44, 53, 57, 44, 51, 44, 57,
            44, 50, 53, 52, 44, 50, 49, 50, 44, 56, 44, 48, 44, 48, 44, 48, 44, 48, 44, 48, 44, 48,
            93, 44, 91, 49, 44, 48, 44, 49, 52, 52, 44, 51, 51, 44, 50, 53, 53, 44, 49, 49, 51, 44,
            49, 57, 52, 44, 51, 48, 44, 49, 50, 56, 44, 49, 50, 49, 44, 49, 56, 44, 50, 49, 57, 44,
            49, 51, 48, 44, 49, 53, 48, 44, 49, 55, 53, 44, 49, 49, 52, 44, 50, 48, 51, 44, 55, 56,
            44, 52, 44, 50, 50, 49, 44, 49, 52, 44, 50, 49, 52, 44, 49, 56, 48, 44, 49, 50, 44, 57,
            53, 44, 49, 54, 53, 44, 50, 50, 48, 44, 56, 53, 44, 54, 44, 49, 50, 52, 44, 49, 49, 54,
            44, 50, 50, 52, 44, 54, 49, 44, 50, 50, 50, 44, 49, 55, 57, 44, 49, 50, 51, 44, 53, 57,
            44, 50, 50, 44, 49, 49, 57, 44, 52, 55, 44, 49, 50, 51, 44, 52, 51, 44, 49, 49, 53, 44,
            49, 53, 56, 44, 50, 49, 49, 44, 49, 55, 54, 44, 50, 44, 50, 51, 54, 44, 50, 53, 50, 44,
            49, 48, 49, 44, 49, 50, 52, 44, 50, 52, 48, 44, 49, 55, 53, 44, 50, 48, 52, 44, 50, 49,
            56, 44, 52, 49, 44, 49, 49, 53, 44, 49, 57, 56, 44, 54, 48, 44, 52, 57, 44, 50, 49, 55,
            44, 49, 49, 51, 44, 49, 51, 53, 44, 50, 52, 57, 44, 50, 50, 55, 44, 52, 55, 44, 49, 52,
            52, 44, 49, 54, 48, 44, 56, 44, 48, 44, 48, 44, 48, 44, 48, 44, 48, 44, 48, 93, 44, 91,
            51, 44, 49, 48, 44, 48, 44, 48, 44, 48, 44, 48, 44, 49, 48, 57, 44, 49, 48, 57, 44, 49,
            48, 53, 44, 49, 49, 54, 44, 49, 48, 57, 44, 49, 48, 49, 44, 49, 49, 48, 44, 49, 49, 54,
            44, 49, 49, 53, 44, 50, 51, 52, 44, 49, 50, 44, 49, 57, 48, 44, 52, 56, 44, 52, 56, 44,
            49, 51, 50, 44, 49, 55, 44, 50, 48, 54, 44, 49, 53, 55, 44, 50, 50, 54, 44, 49, 53, 49,
            44, 49, 57, 54, 44, 49, 54, 57, 44, 50, 50, 44, 49, 52, 52, 44, 50, 49, 49, 44, 49, 57,
            51, 44, 49, 56, 49, 44, 52, 48, 44, 49, 50, 54, 44, 51, 44, 49, 55, 49, 44, 52, 55, 44,
            53, 55, 44, 53, 57, 44, 49, 44, 49, 50, 54, 44, 50, 51, 54, 44, 57, 44, 48, 44, 50, 53,
            52, 44, 49, 51, 53, 44, 50, 49, 56, 44, 54, 44, 48, 44, 48, 44, 48, 44, 48, 44, 48, 44,
            48, 93, 44, 91, 49, 44, 54, 44, 48, 44, 49, 52, 44, 49, 55, 50, 44, 49, 57, 50, 44, 49,
            48, 49, 44, 49, 51, 52, 44, 49, 49, 53, 44, 49, 54, 54, 44, 50, 51, 49, 44, 49, 52, 44,
            50, 53, 53, 44, 53, 44, 49, 50, 56, 44, 53, 48, 44, 52, 53, 44, 55, 56, 44, 49, 48, 53,
            44, 50, 49, 51, 44, 56, 56, 44, 49, 48, 54, 44, 49, 56, 55, 44, 50, 48, 56, 44, 50, 52,
            49, 44, 49, 52, 52, 44, 55, 56, 44, 49, 53, 56, 44, 48, 44, 49, 54, 48, 44, 49, 51, 55,
            44, 49, 57, 55, 44, 49, 50, 50, 44, 49, 56, 54, 44, 49, 53, 50, 44, 50, 49, 48, 44, 49,
            49, 51, 44, 50, 51, 55, 44, 50, 48, 55, 44, 55, 49, 44, 53, 53, 44, 49, 57, 52, 44, 56,
            56, 44, 49, 56, 50, 44, 55, 51, 44, 49, 57, 55, 44, 49, 51, 51, 44, 52, 50, 44, 49, 52,
            50, 44, 49, 54, 49, 44, 49, 52, 51, 44, 49, 57, 50, 44, 56, 57, 44, 50, 48, 54, 44, 49,
            57, 56, 44, 51, 57, 44, 49, 53, 50, 44, 49, 54, 44, 49, 53, 53, 44, 49, 53, 53, 44, 50,
            51, 54, 44, 49, 50, 53, 44, 57, 44, 49, 48, 54, 44, 49, 50, 44, 49, 55, 55, 44, 49, 51,
            52, 44, 49, 52, 56, 44, 54, 44, 48, 44, 48, 44, 48, 44, 48, 44, 48, 44, 48, 93, 44, 91,
            49, 44, 48, 44, 54, 53, 44, 49, 56, 49, 44, 54, 55, 44, 50, 50, 53, 44, 50, 52, 50, 44,
            49, 54, 54, 44, 50, 48, 53, 44, 56, 52, 44, 50, 51, 51, 44, 53, 49, 44, 52, 49, 44, 50,
            52, 55, 44, 51, 55, 44, 49, 57, 54, 44, 49, 55, 53, 44, 50, 57, 44, 49, 54, 52, 44, 52,
            57, 44, 51, 57, 44, 49, 55, 48, 44, 49, 48, 50, 44, 49, 57, 49, 44, 49, 48, 51, 44, 53,
            52, 44, 53, 51, 44, 49, 48, 48, 44, 51, 50, 44, 53, 44, 55, 50, 44, 50, 48, 54, 44, 52,
            53, 44, 50, 52, 51, 44, 48, 44, 52, 55, 44, 49, 54, 51, 44, 49, 52, 49, 44, 57, 54, 44,
            51, 55, 44, 56, 53, 44, 49, 53, 53, 44, 49, 55, 51, 44, 50, 48, 52, 44, 51, 50, 44, 49,
            54, 53, 44, 50, 51, 52, 44, 49, 49, 51, 44, 51, 50, 44, 50, 51, 51, 44, 50, 49, 55, 44,
            49, 51, 55, 44, 56, 49, 44, 57, 49, 44, 49, 48, 49, 44, 52, 56, 44, 49, 50, 54, 44, 49,
            56, 51, 44, 50, 49, 52, 44, 53, 48, 44, 49, 52, 49, 44, 51, 55, 44, 49, 49, 54, 44, 49,
            56, 57, 44, 50, 53, 50, 44, 50, 52, 57, 44, 55, 56, 44, 54, 44, 52, 44, 48, 44, 48, 44,
            48, 44, 48, 44, 48, 44, 48, 93, 44, 91, 51, 44, 50, 49, 44, 48, 44, 48, 44, 48, 44, 49,
            54, 44, 48, 44, 48, 44, 54, 44, 53, 52, 44, 49, 57, 56, 44, 49, 53, 48, 44, 56, 54, 44,
            50, 51, 49, 44, 55, 49, 44, 53, 48, 44, 50, 52, 54, 44, 53, 52, 44, 50, 52, 54, 44, 50,
            49, 52, 44, 56, 55, 44, 55, 48, 44, 51, 56, 44, 49, 57, 57, 44, 53, 48, 44, 50, 49, 49,
            44, 50, 50, 56, 44, 55, 50, 44, 50, 50, 50, 44, 49, 52, 50, 44, 49, 53, 44, 54, 54, 44,
            49, 51, 55, 44, 50, 53, 53, 44, 56, 51, 44, 56, 49, 44, 49, 56, 44, 54, 52, 44, 49, 50,
            56, 44, 49, 49, 55, 44, 50, 49, 48, 44, 57, 48, 44, 51, 53, 44, 49, 48, 50, 44, 50, 48,
            50, 44, 55, 54, 44, 49, 48, 50, 44, 50, 51, 54, 44, 49, 53, 49, 44, 55, 50, 44, 50, 49,
            54, 44, 49, 52, 52, 44, 49, 53, 53, 44, 53, 48, 44, 55, 52, 44, 50, 49, 51, 44, 50, 48,
            56, 44, 49, 56, 44, 50, 52, 54, 44, 49, 44, 48, 44, 48, 44, 48, 44, 48, 44, 48, 44, 48,
            93, 44, 91, 49, 44, 54, 44, 48, 44, 56, 44, 50, 49, 56, 44, 49, 56, 55, 44, 49, 57, 53,
            44, 50, 52, 44, 49, 48, 49, 44, 50, 52, 53, 44, 50, 50, 44, 51, 57, 44, 50, 53, 49, 44,
            49, 56, 55, 44, 55, 44, 57, 54, 44, 49, 52, 54, 44, 51, 52, 44, 56, 44, 49, 53, 55, 44,
            51, 54, 44, 50, 49, 54, 44, 50, 48, 50, 44, 50, 57, 44, 49, 57, 53, 44, 54, 55, 44, 49,
            52, 48, 44, 50, 52, 50, 44, 53, 49, 44, 53, 55, 44, 49, 57, 55, 44, 50, 50, 51, 44, 49,
            50, 48, 44, 50, 48, 53, 44, 49, 48, 57, 44, 49, 50, 48, 44, 50, 50, 53, 44, 56, 50, 44,
            49, 49, 55, 44, 49, 55, 57, 44, 49, 55, 57, 44, 49, 49, 57, 44, 51, 56, 44, 50, 50, 56,
            44, 53, 50, 44, 50, 50, 53, 44, 50, 50, 52, 44, 49, 51, 55, 44, 49, 49, 53, 44, 50, 50,
            54, 44, 56, 54, 44, 50, 51, 44, 49, 50, 53, 44, 50, 50, 56, 44, 50, 52, 51, 44, 49, 50,
            44, 57, 49, 44, 56, 48, 44, 49, 57, 55, 44, 49, 57, 53, 44, 51, 52, 44, 51, 48, 44, 51,
            54, 44, 49, 56, 49, 44, 56, 52, 44, 52, 48, 44, 49, 56, 53, 44, 49, 53, 52, 44, 49, 44,
            48, 44, 48, 44, 48, 44, 48, 44, 48, 44, 48, 93, 44, 91, 48, 44, 49, 51, 44, 48, 44, 48,
            44, 48, 44, 51, 50, 44, 52, 55, 44, 57, 57, 44, 49, 48, 56, 44, 49, 48, 53, 44, 49, 48,
            49, 44, 49, 49, 48, 44, 49, 49, 54, 44, 56, 51, 44, 49, 49, 54, 44, 57, 55, 44, 49, 49,
            54, 44, 49, 48, 49, 44, 53, 52, 44, 48, 44, 48, 44, 48, 44, 55, 44, 49, 51, 54, 44, 50,
            50, 44, 50, 49, 53, 44, 50, 48, 49, 44, 50, 51, 48, 44, 50, 49, 55, 44, 50, 52, 53, 44,
            57, 44, 50, 50, 54, 44, 49, 48, 50, 44, 49, 52, 57, 44, 50, 48, 50, 44, 49, 49, 48, 44,
            56, 50, 44, 49, 49, 54, 44, 50, 50, 54, 44, 49, 52, 52, 44, 49, 54, 53, 44, 50, 49, 54,
            44, 52, 56, 44, 57, 56, 44, 49, 57, 56, 44, 49, 51, 54, 44, 50, 48, 56, 44, 50, 52, 50,
            44, 51, 53, 44, 49, 48, 48, 44, 56, 55, 44, 50, 51, 49, 44, 50, 53, 44, 49, 57, 53, 44,
            49, 56, 48, 44, 48, 44, 48, 44, 48, 44, 48, 44, 48, 44, 48, 44, 48, 93, 93, 125,
        ];
        let mut deps = mock_dependencies();

        save_client_state::<NearLightClient>(
            deps.as_mut(),
            WasmClientState {
                data: ClientState {
                    chain_id: "".to_string(),
                    latest_height: 8,
                    ibc_account_id: "ibc-union.node0".parse().unwrap(),
                    initial_block_producers: vec![],
                    frozen_height: 0,
                },
                checksum: [0; 32].into(),
                latest_height: height(8),
            },
        );

        save_consensus_state::<NearLightClient>(
            deps.as_mut(),
            WasmConsensusState {
                data: ConsensusState {
                    state: serde_json::from_str(
                         r#"{"height":3375,"epoch_id":"4hfNTi6XHVo34VY7k4h7GJBWE8xbEJAk92c2ttf8xoVf","next_epoch_id":"2acN456fNCK8hy5V6T6qodE4AGYmYyWd17Jy3FYeGomk","prev_state_root":"J33gHY214xtdYYopfvohsu5JeRvdpUytxvrkCw3Ues4s","outcome_root":"7tkzFg8RHBmMw1ncRJZCCZAizgq4rwCftTKYLce8RU8t","timestamp":1721755287056479127,"timestamp_nanosec":1721755287056479127,"next_bp_hash":"N674zAVEovLgQJsLWqavBj9sCaTWh6XRcEaL1fcSv9F","block_merkle_root":"4xTs6nVJtYeovyszCDbM2B6unLX9HB6a8Dc955gcPdPY"}"#
                    ).unwrap(),
                    chunk_prev_state_root: CryptoHash([193, 114, 116, 236, 32, 52, 48, 3, 63, 141, 251, 243, 43, 183, 171, 164, 249, 121, 112, 90, 162, 153, 194, 14, 105, 139, 16, 121, 64, 69, 102, 34])
                    ,
                    timestamp: 10000,
                },
            },
            &height(8),
        );

        let value: Vec<u8> = [
            10, 10, 99, 111, 109, 101, 116, 98, 108, 115, 45, 50, 18, 20, 10, 1, 49, 18, 15, 79,
            82, 68, 69, 82, 95, 85, 78, 79, 82, 68, 69, 82, 69, 68, 24, 1, 34, 18, 10, 9, 48, 56,
            45, 119, 97, 115, 109, 45, 49, 26, 5, 10, 3, 105, 98, 99,
        ]
        .to_vec();

        let mut conn = ConnectionEnd::<String, String, String>::decode_as::<Proto>(&value).unwrap();
        conn.client_id = "cometbls-2".to_string();
        conn.counterparty.client_id = "08-wasm-5".to_string();

        // println!(
        //     "DECODED: {:?}",
        //     ConnectionEnd::<String, String, String>::decode_as::<Proto>(&value).unwrap()
        // );

        NearLightClient::verify_membership(
            deps.as_ref(),
            Height {
                revision_number: 0,
                revision_height: 7,
            },
            0,
            0,
            proof.to_vec(),
            unionlabs::ibc::core::commitment::merkle_path::MerklePath {
                key_path: vec!["ibc".to_string(), "connections/connection-2".to_string()],
            },
            ics008_wasm_client::StorageState::Occupied(conn.encode_as::<Proto>()),
        )
        .unwrap();
    }

    #[tokio::test]
    async fn create_client() {
        let (rpc, mut deps, env, info, client_state, consensus_state) = initialize().await;

        instantiate(
            deps.as_mut(),
            env,
            info,
            InstantiateMsg {
                client_state: client_state.encode_as::<Proto>().into(),
                consensus_state: consensus_state.encode_as::<Proto>().into(),
                checksum: [0; 32].into(),
            },
        )
        .unwrap();

        let acc = rpc
            .call(methods::query::RpcQueryRequest {
                block_reference: BlockReference::Finality(near_primitives::types::Finality::Final),
                request: near_primitives::views::QueryRequest::ViewAccount {
                    account_id: String::from("dev-1721650593739.node0").try_into().unwrap(),
                },
            })
            .await
            .unwrap();

        println!("Account: {acc:?}");
    }

    #[tokio::test]
    async fn update_client() {
        let (rpc, mut deps, env, info, client_state, consensus_state) = initialize().await;

        instantiate(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            InstantiateMsg {
                client_state: client_state.clone().encode_as::<Proto>().into(),
                consensus_state: consensus_state.clone().encode_as::<Proto>().into(),
                checksum: [0; 32].into(),
            },
        )
        .unwrap();

        for _ in 0..30 {
            let wasm_client_state: WasmClientState =
                read_client_state::<NearLightClient>(deps.as_ref()).unwrap();
            let wasm_consensus_state: WasmConsensusState = read_consensus_state::<NearLightClient>(
                deps.as_ref(),
                &Height {
                    revision_number: 0,
                    revision_height: wasm_client_state.data.latest_height,
                },
            )
            .unwrap()
            .unwrap();
            println!(
                "HEIGHT: {}, EPOCH: {}",
                wasm_client_state.data.latest_height, wasm_consensus_state.data.state.epoch_id
            );

            let block = rpc
                .call(methods::block::RpcBlockRequest {
                    block_reference: BlockReference::BlockId(BlockId::Height(
                        wasm_client_state.data.latest_height,
                    )),
                })
                .await
                .unwrap();

            let lc_block = loop {
                tokio::time::sleep(Duration::from_millis(100)).await;
                let lc_block = rpc
                    .call(
                        methods::next_light_client_block::RpcLightClientNextBlockRequest {
                            last_block_hash: block.header.hash,
                        },
                    )
                    .await;

                if let Ok(Some(lc_block)) = lc_block {
                    if lc_block.inner_lite.height > wasm_client_state.data.latest_height + 1 {
                        println!(
                            "current height: {}, lc block height: {}",
                            wasm_client_state.data.latest_height, lc_block.inner_lite.height
                        );
                        break lc_block;
                    }
                }
            };

            let current_height = lc_block.inner_lite.height;

            let (prev_state_root, prev_state_root_proof) = chunk_proof(&rpc, current_height).await;

            let header = Header {
                new_state: convert_light_client_block_view(lc_block),
                trusted_height: wasm_client_state.data.latest_height,
                prev_state_root_proof,
                prev_state_root,
            };

            NearLightClient::verify_header(deps.as_ref(), env.clone(), header.clone()).unwrap();

            NearLightClient::update_state(deps.as_mut(), env.clone(), header).unwrap();
        }
    }

    pub fn convert_block_header_inner(
        block_view: BlockHeaderInnerLiteView,
    ) -> near::block_header_inner::BlockHeaderInnerLiteView {
        near::block_header_inner::BlockHeaderInnerLiteView {
            height: block_view.height,
            epoch_id: CryptoHash(block_view.epoch_id.0),
            next_epoch_id: CryptoHash(block_view.next_epoch_id.0),
            prev_state_root: CryptoHash(block_view.prev_state_root.0),
            outcome_root: CryptoHash(block_view.outcome_root.0),
            timestamp: block_view.timestamp,
            timestamp_nanosec: block_view.timestamp_nanosec,
            next_bp_hash: CryptoHash(block_view.next_bp_hash.0),
            block_merkle_root: CryptoHash(block_view.block_merkle_root.0),
        }
    }

    pub fn convert_light_client_block_view(
        light_client_block: LightClientBlockView,
    ) -> near::light_client_block::LightClientBlockView {
        near::light_client_block::LightClientBlockView {
            inner_lite: convert_block_header_inner(light_client_block.inner_lite),
            prev_block_hash: near_primitives_core::hash::CryptoHash(
                light_client_block.prev_block_hash.0,
            ),
            next_block_inner_hash: near_primitives_core::hash::CryptoHash(
                light_client_block.next_block_inner_hash.0,
            ),
            inner_rest_hash: near_primitives_core::hash::CryptoHash(
                light_client_block.inner_rest_hash.0,
            ),
            next_bps: light_client_block.next_bps.map(|bps| {
                bps.into_iter()
                    .map(|stake| {
                        let near_primitives::views::validator_stake_view::ValidatorStakeView::V1(
                            stake,
                        ) = stake;
                        near::validator_stake_view::ValidatorStakeView::V1(
                            near::validator_stake_view::ValidatorStakeViewV1 {
                                account_id: stake.account_id,
                                public_key: types::PublicKey::Ed25519(
                                    stake.public_key.key_data().try_into().unwrap(),
                                ),
                                stake: stake.stake,
                            },
                        )
                    })
                    .collect()
            }),
            approvals_after_next: light_client_block
                .approvals_after_next
                .into_iter()
                .map(|sig| {
                    sig.map(|s| match s.as_ref() {
                        near_crypto::Signature::ED25519(sig) => {
                            Box::new(types::Signature::Ed25519(sig.to_bytes().to_vec()))
                        }
                        near_crypto::Signature::SECP256K1(_) => {
                            Box::new(types::Signature::Secp256k1(Vec::new()))
                        }
                    })
                })
                .collect(),
        }
    }

    pub async fn chunk_proof(
        rpc: &JsonRpcClient,
        block_height: BlockHeight,
    ) -> (CryptoHash, types::MerklePath) {
        let chunks = rpc
            .call(methods::block::RpcBlockRequest {
                block_reference: BlockReference::BlockId(BlockId::Height(block_height)),
            })
            .await
            .unwrap()
            .chunks;

        let prev_state_root = CryptoHash(chunks[0].prev_state_root.0);

        let (_, merkle_path) = near_primitives::merkle::merklize(
            &chunks
                .into_iter()
                .map(|chunk| CryptoHash(chunk.prev_state_root.0))
                .collect::<Vec<CryptoHash>>(),
        );

        let prev_state_root_proof = merkle_path[0]
            .clone()
            .into_iter()
            .map(|item| MerklePathItem {
                hash: near_primitives_core::hash::CryptoHash(item.hash.0),
                direction: match item.direction {
                    near_primitives::merkle::Direction::Left => types::Direction::Left,
                    near_primitives::merkle::Direction::Right => types::Direction::Right,
                },
            })
            .collect();

        (prev_state_root, prev_state_root_proof)
    }

    pub fn convert_block_producers(
        bps: Vec<near_primitives::views::validator_stake_view::ValidatorStakeView>,
    ) -> Vec<near::validator_stake_view::ValidatorStakeView> {
        bps.into_iter()
            .map(|stake| {
                let near_primitives::views::validator_stake_view::ValidatorStakeView::V1(stake) =
                    stake;
                let stake = near::validator_stake_view::ValidatorStakeView::V1(
                    near::validator_stake_view::ValidatorStakeViewV1 {
                        account_id: stake.account_id,
                        public_key: unionlabs::near::types::PublicKey::Ed25519(
                            stake.public_key.key_data().try_into().unwrap(),
                        ),
                        stake: stake.stake,
                    },
                );
                stake
            })
            .collect()
    }

    pub fn block_header_to_inner_lite(
        header: near_primitives::views::BlockHeaderView,
    ) -> near::block_header_inner::BlockHeaderInnerLiteView {
        use near_primitives_core::hash::CryptoHash;
        near::block_header_inner::BlockHeaderInnerLiteView {
            height: header.height,
            epoch_id: CryptoHash(header.epoch_id.0),
            next_epoch_id: CryptoHash(header.next_epoch_id.0),
            prev_state_root: CryptoHash(header.prev_state_root.0),
            outcome_root: CryptoHash(header.outcome_root.0),
            timestamp: header.timestamp,
            timestamp_nanosec: header.timestamp_nanosec,
            next_bp_hash: CryptoHash(header.next_bp_hash.0),
            block_merkle_root: CryptoHash(header.block_merkle_root.0),
        }
    }
}

fn key_from_path(path: &str) -> Vec<u8> {
    let mut commitments: Vec<u8> = Vec::new();
    commitments.extend(b"commitments");
    commitments.extend(borsh::to_vec(path).unwrap());
    commitments
}