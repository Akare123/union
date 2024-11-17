use std::collections::VecDeque;

use chain_utils::{
    cosmos_sdk::{
        cosmos_sdk_error::{ChannelError, ClientError, CosmosSdkError, IbcWasmError, SdkError},
        CosmosKeyring, GasConfig,
    },
    keyring::{KeyringConfig, KeyringEntry},
    BoxDynError,
};
use jsonrpsee::{
    core::{async_trait, RpcResult},
    types::ErrorObject,
    Extensions,
};
use prost::Message;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use tracing::{debug, error, info, instrument, warn};
use unionlabs::{
    self,
    bounded::BoundedI64,
    cosmos::{
        auth::base_account::BaseAccount,
        base::abci::gas_info::GasInfo,
        crypto::{secp256k1, AnyPubKey},
        tx::{
            auth_info::AuthInfo, mode_info::ModeInfo, sign_doc::SignDoc, signer_info::SignerInfo,
            signing::sign_info::SignMode, tx::Tx, tx_body::TxBody, tx_raw::TxRaw,
        },
    },
    encoding::{EncodeAs, Proto},
    google::protobuf::any::{mk_any, Any},
    hash::H256,
    signer::CosmosSigner,
    ErrorReporter,
};
use voyager_message::{
    core::ChainId,
    data::{Data, WithChainId},
    ibc_union, ibc_v1,
    module::{PluginInfo, PluginServer},
    DefaultCmd, Plugin, PluginMessage, VoyagerMessage, FATAL_JSONRPC_ERROR_CODE,
};
use voyager_vm::{call, noop, pass::PassResult, Op};

use crate::{
    call::{IbcMessage, ModuleCall},
    callback::ModuleCallback,
};

pub mod call;
pub mod callback;
pub mod data;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    Module::run().await
}

#[derive(Debug, Clone)]
pub struct Module {
    pub chain_id: ChainId,
    pub ibc_union_contract_address: String,
    pub keyring: CosmosKeyring,
    pub tm_client: cometbft_rpc::Client,
    pub grpc_url: String,
    pub gas_config: GasConfig,
    pub bech32_prefix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub chain_id: ChainId,
    pub ibc_union_contract_address: String,
    pub keyring: KeyringConfig,
    pub ws_url: String,
    pub grpc_url: String,
    pub gas_config: GasConfig,
}

impl Plugin for Module {
    type Call = ModuleCall;
    type Callback = ModuleCallback;

    type Config = Config;
    type Cmd = DefaultCmd;

    async fn new(config: Self::Config) -> Result<Self, BoxDynError> {
        let tm_client = cometbft_rpc::Client::new(config.ws_url).await?;

        let chain_id = tm_client.status().await?.node_info.network.to_string();

        let bech32_prefix = protos::cosmos::auth::v1beta1::query_client::QueryClient::connect(
            config.grpc_url.clone(),
        )
        .await
        .unwrap()
        .bech32_prefix(protos::cosmos::auth::v1beta1::Bech32PrefixRequest {})
        .await
        .unwrap()
        .into_inner()
        .bech32_prefix;

        Ok(Self {
            ibc_union_contract_address: config.ibc_union_contract_address,
            keyring: CosmosKeyring::new(
                config.keyring.name,
                config.keyring.keys.into_iter().map(|entry| {
                    let signer = CosmosSigner::new(
                        bip32::secp256k1::ecdsa::SigningKey::from_bytes(
                            entry.value().as_slice().into(),
                        )
                        .expect("invalid private key"),
                        bech32_prefix.clone(),
                    );

                    KeyringEntry {
                        name: entry.name(),
                        address: signer.to_string(),
                        signer,
                    }
                }),
            ),
            tm_client,
            chain_id: ChainId::new(chain_id),
            grpc_url: config.grpc_url,
            gas_config: config.gas_config,
            bech32_prefix,
        })
    }

    fn info(config: Self::Config) -> PluginInfo {
        PluginInfo {
            name: plugin_name(&config.chain_id),
            interest_filter: format!(
                r#"
if ."@type" == "data" then
    ."@value" as $data |

    # pull all transaction data messages
    ($data."@type" == "identified_ibc_datagram_batch" or $data."@type" == "identified_ibc_datagram")
        and $data."@value".chain_id == "{chain_id}"
else
    false
end
"#,
                chain_id = config.chain_id,
            ),
        }
    }

    async fn cmd(_config: Self::Config, cmd: Self::Cmd) {
        match cmd {}
    }
}

fn plugin_name(chain_id: &ChainId) -> String {
    pub const PLUGIN_NAME: &str = env!("CARGO_PKG_NAME");

    format!("{PLUGIN_NAME}/{}", chain_id)
}

impl Module {
    fn plugin_name(&self) -> String {
        plugin_name(&self.chain_id)
    }

    pub async fn do_send_transaction(
        &self,
        msgs: Vec<IbcMessage>,
    ) -> Result<Op<VoyagerMessage>, BroadcastTxCommitError> {
        let res = self
            .keyring
            .with(|signer| {
                let msg = msgs.clone();

                async move {
                    // TODO: Figure out a way to thread this value through
                    let memo = format!("Voyager {}", env!("CARGO_PKG_VERSION"));

                    let msgs = process_msgs(msg, signer, self.ibc_union_contract_address.clone());

                    // let simulation_results = stream::iter(msgs.clone().into_iter().enumerate())
                    //     .then(move |(idx, (effect, msg))| async move {
                    //         let type_url = msg.type_url.clone();

                    //         self.simulate_tx(
                    //             signer,
                    //             [msg],
                    //             format!("Voyager {}", env!("CARGO_PKG_VERSION"))
                    //         )
                    //         .map(move |res| (idx, type_url, effect, res))
                    //         .await
                    //     })
                    //     .collect::<Vec<(usize, String, _, Result<_, _>)>>()
                    //     .await;

                    // // iterate backwards such that when we remove items from msgs, we don't shift the relative indices
                    // for (idx, type_url, msg, simulation_result) in simulation_results.into_iter().rev() {
                    //     let _span = info_span!(
                    //         "simulation result",
                    //         msg = type_url,
                    //         idx,
                    //     )
                    //     .entered();

                    //     match simulation_result {
                    //         Ok((_, _, gas_info)) => {
                    //             info!(
                    //                 gas_wanted = %gas_info.gas_wanted,
                    //                 gas_used = %gas_info.gas_used,
                    //                 "individual message simulation successful",
                    //             );

                    //             log_msg(&self.chain_id, msg);
                    //         }
                    //         Err(error) => {
                    //             if error.message().contains("account sequence mismatch") {
                    //                 warn!("account sequence mismatch on individual message simulation, treating this message as successful");
                    //                 log_msg(&self.chain_id, msg);
                    //             } else {
                    //                 error!(
                    //                     %error,
                    //                     "individual message simulation failed"
                    //                 );

                    //                 log_msg(&self.chain_id, msg);

                    //                 msgs.remove(idx);
                    //             }
                    //         }
                    //     }
                    // }

                    // if msgs.is_empty() {
                    //     info!(
                    //         "no messages remaining to submit after filtering out failed transactions"
                    //     );
                    //     return Ok(());
                    // }

                    let batch_size = msgs.len();
                    let msg_names = msgs.iter().map(move |x| x.1.type_url.clone()).collect::<Vec<_>>();

                    match self.broadcast_tx_commit(
                        signer,
                        msgs.iter().map(move |x| x.1.clone()).collect::<Vec<_>>(),
                        memo
                    ).await {
                        Ok((tx_hash, gas_used)) => {
                            info!(
                                %tx_hash,
                                %gas_used,
                                batch.size = %batch_size,
                                "submitted cosmos transaction"
                            );

                            for msg in msg_names {
                                info!(%tx_hash, %msg, "cosmos tx");
                            }

                            Ok(())
                        }
                        Err(err) => match err {
                            BroadcastTxCommitError::Tx(CosmosSdkError::ChannelError(
                                ChannelError::ErrRedundantTx,
                            )) => {
                                info!("packet messages are redundant");
                                Ok(())
                            }
                            // BroadcastTxCommitError::Tx(CosmosSdkError::SdkError(
                            //     SdkError::ErrOutOfGas
                            // )) => {
                            //     error!("out of gas");
                            //     Err(BroadcastTxCommitError::OutOfGas)
                            // }
                            BroadcastTxCommitError::Tx(CosmosSdkError::SdkError(
                                SdkError::ErrWrongSequence
                            )) => {
                                warn!("account sequence mismatch on tx submission, message will be requeued and retried");
                                Err(BroadcastTxCommitError::AccountSequenceMismatch(None))
                            }
                            BroadcastTxCommitError::SimulateTx(err) if err.message().contains("account sequence mismatch") => {
                                warn!("account sequence mismatch on simulation, message will be requeued and retried");
                                Err(BroadcastTxCommitError::AccountSequenceMismatch(Some(err)))
                            }
                            err => Err(err),
                        },
                    }
                }
            })
            .await;

        let rewrap_msg =
            || PluginMessage::new(self.plugin_name(), ModuleCall::SubmitTransaction(msgs));

        match res {
            Some(Err(BroadcastTxCommitError::AccountSequenceMismatch(_))) => Ok(call(rewrap_msg())),
            Some(Err(BroadcastTxCommitError::OutOfGas)) => Ok(call(rewrap_msg())),
            Some(Err(BroadcastTxCommitError::SimulateTx(err))) => {
                error!(
                    error = %ErrorReporter(err),
                    "transaction simulation failed, message will be requeued and retried"
                );

                Ok(call(rewrap_msg()))
            }
            Some(Err(BroadcastTxCommitError::QueryLatestHeight(err))) => {
                error!(error = %ErrorReporter(err), "error querying latest height");

                Ok(call(rewrap_msg()))
            }
            Some(res) => res.map(|()| noop()),
            // None => Ok(seq([defer_relative(1), effect(WithChainId{chain_id: self.chain_id.clone(), message: msg})])),
            None => Ok(call(rewrap_msg())),
        }
    }

    /// - simulate tx
    /// - submit tx
    /// - wait for inclusion
    /// - return (tx_hash, gas_used)
    pub async fn broadcast_tx_commit(
        &self,
        signer: &CosmosSigner,
        messages: impl IntoIterator<Item = protos::google::protobuf::Any> + Clone,
        memo: String,
    ) -> Result<(H256, BoundedI64<0, { i64::MAX }>), BroadcastTxCommitError> {
        let account = self.account_info(&signer.to_string()).await;

        let (tx_body, mut auth_info, simulation_gas_info) =
            match self.simulate_tx(signer, messages, memo).await {
                Ok((tx_body, auth_info, simulation_gas_info)) => {
                    (tx_body, auth_info, simulation_gas_info)
                }
                Err((tx_body, auth_info, _err)) => (
                    tx_body,
                    auth_info,
                    GasInfo {
                        gas_wanted: u64::MAX,
                        gas_used: u64::MAX,
                    },
                ),
            };
        // .map_err(BroadcastTxCommitError::SimulateTx)?;

        info!(
            gas_used = %simulation_gas_info.gas_used,
            gas_wanted = %simulation_gas_info.gas_wanted,
            "tx simulation successful"
        );

        auth_info.fee = self.gas_config.mk_fee(simulation_gas_info.gas_used);

        // dbg!(&auth_info.fee);

        info!(
            fee = %auth_info.fee.amount[0].amount,
            gas_multiplier = %self.gas_config.gas_multiplier,
            "submitting transaction with gas"
        );

        // re-sign the new auth info with the simulated gas
        let signature = signer
            .try_sign(
                &SignDoc {
                    body_bytes: tx_body.clone().encode_as::<Proto>(),
                    auth_info_bytes: auth_info.clone().encode_as::<Proto>(),
                    chain_id: self.chain_id.to_string(),
                    account_number: account.account_number,
                }
                .encode_as::<Proto>(),
            )
            .expect("signing failed")
            .to_vec();

        let tx_raw_bytes = TxRaw {
            body_bytes: tx_body.clone().encode_as::<Proto>(),
            auth_info_bytes: auth_info.clone().encode_as::<Proto>(),
            signatures: [signature].to_vec(),
        }
        .encode_as::<Proto>();

        let tx_hash: H256 = sha2::Sha256::new()
            .chain_update(&tx_raw_bytes)
            .finalize()
            .into();

        if let Ok(tx) = self.tm_client.tx(tx_hash, false).await {
            debug!(%tx_hash, "tx already included");
            return Ok((tx_hash, tx.tx_result.gas_used));
        }

        let response = self
            .tm_client
            .broadcast_tx_sync(&tx_raw_bytes)
            .await
            .map_err(BroadcastTxCommitError::BroadcastTxSync)
            .unwrap();

        assert_eq!(tx_hash, response.hash, "tx hash calculated incorrectly");

        info!(
            check_tx_code = %response.code,
            codespace = %response.codespace,
            check_tx_log = %response.log
        );

        if response.code > 0 {
            let error = CosmosSdkError::from_code_and_codespace(&response.codespace, response.code);

            error!(%error, "cosmos tx failed");

            return Err(BroadcastTxCommitError::Tx(error));
        };

        let mut target_height = self
            .tm_client
            .block(None)
            .await
            .map_err(BroadcastTxCommitError::QueryLatestHeight)?
            .block
            .header
            .height;

        // TODO: Do this in the queue
        let mut i = 0;
        loop {
            let reached_height = 'l: loop {
                let current_height = self
                    .tm_client
                    .block(None)
                    .await
                    .map_err(BroadcastTxCommitError::QueryLatestHeight)?
                    .block
                    .header
                    .height;

                if current_height >= target_height {
                    break 'l current_height;
                }
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            };

            let tx_inclusion = self.tm_client.tx(tx_hash, false).await;

            debug!(?tx_inclusion);

            match tx_inclusion {
                Ok(tx) => {
                    if tx.tx_result.code == 0 {
                        break Ok((tx_hash, tx.tx_result.gas_used));
                    } else {
                        let error = CosmosSdkError::from_code_and_codespace(
                            &tx.tx_result.codespace,
                            tx.tx_result.code,
                        );
                        warn!(
                            %error,
                            %tx_hash,

                            %tx.tx_result.code,
                            ?tx.tx_result.data,
                            %tx.tx_result.log,
                            %tx.tx_result.info,
                            %tx.tx_result.gas_wanted,
                            %tx.tx_result.gas_used,
                            ?tx.tx_result.events,
                            %tx.tx_result.codespace,

                            "cosmos transaction failed"
                        );
                        break Err(BroadcastTxCommitError::Tx(error));
                    }
                }
                Err(err) if i > 5 => {
                    warn!("tx inclusion couldn't be retrieved after {} try", i);
                    break Err(BroadcastTxCommitError::Inclusion(err));
                }
                Err(_) => {
                    target_height = reached_height.add(&1);
                    i += 1;
                    continue;
                }
            }
        }
    }

    pub async fn simulate_tx(
        &self,
        signer: &CosmosSigner,
        messages: impl IntoIterator<Item = protos::google::protobuf::Any> + Clone,
        memo: String,
    ) -> Result<(TxBody, AuthInfo, GasInfo), (TxBody, AuthInfo, tonic::Status)> {
        use protos::cosmos::tx;

        let account = self.account_info(&signer.to_string()).await;

        let mut client = tx::v1beta1::service_client::ServiceClient::connect(self.grpc_url.clone())
            .await
            .unwrap();

        let tx_body = TxBody {
            // TODO: Use RawAny here
            messages: messages.clone().into_iter().map(Into::into).collect(),
            memo,
            timeout_height: 0,
            extension_options: vec![],
            non_critical_extension_options: vec![],
            unordered: false,
            timeout_timestamp: None,
        };

        let auth_info = AuthInfo {
            signer_infos: [SignerInfo {
                public_key: Some(AnyPubKey::Secp256k1(secp256k1::PubKey {
                    key: signer.public_key(),
                })),
                mode_info: ModeInfo::Single {
                    mode: SignMode::Direct,
                },
                sequence: account.sequence,
            }]
            .to_vec(),
            fee: self.gas_config.mk_fee(self.gas_config.max_gas).clone(),
        };

        let simulation_signature = signer
            .try_sign(
                &SignDoc {
                    body_bytes: tx_body.clone().encode_as::<Proto>(),
                    auth_info_bytes: auth_info.clone().encode_as::<Proto>(),
                    chain_id: self.chain_id.to_string(),
                    account_number: account.account_number,
                }
                .encode_as::<Proto>(),
            )
            .expect("signing failed")
            .to_vec();

        let result = client
            .simulate(tx::v1beta1::SimulateRequest {
                tx_bytes: Tx {
                    body: tx_body.clone(),
                    auth_info: auth_info.clone(),
                    signatures: [simulation_signature.clone()].to_vec(),
                }
                .encode_as::<Proto>(),
                ..Default::default()
            })
            .await;

        match result {
            Ok(ok) => Ok((
                tx_body,
                auth_info,
                ok.into_inner()
                    .gas_info
                    .expect("gas info is present on successful simulation result")
                    .into(),
            )),
            Err(err) => {
                info!(error = %ErrorReporter(&err), "tx simulation failed");
                Err((tx_body, auth_info, err))
            }
        }
    }

    async fn account_info(&self, account: &str) -> BaseAccount {
        debug!(%account, "fetching account");

        let Any(account) = protos::cosmos::auth::v1beta1::query_client::QueryClient::connect(
            self.grpc_url.clone(),
        )
        .await
        .unwrap()
        .account(protos::cosmos::auth::v1beta1::QueryAccountRequest {
            address: account.to_string(),
        })
        .await
        .unwrap()
        .into_inner()
        .account
        .unwrap()
        .try_into()
        .unwrap();

        account
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BroadcastTxCommitError {
    #[error("error querying latest height")]
    QueryLatestHeight(#[source] cometbft_rpc::JsonRpcError),
    #[error("error sending broadcast_tx_sync")]
    BroadcastTxSync(#[source] cometbft_rpc::JsonRpcError),
    #[error("tx was not included")]
    Inclusion(#[source] cometbft_rpc::JsonRpcError),
    #[error("tx failed: {0:?}")]
    Tx(CosmosSdkError),
    #[error("tx simulation failed")]
    SimulateTx(#[source] tonic::Status),
    #[error("account sequence mismatch")]
    AccountSequenceMismatch(#[source] Option<tonic::Status>),
    #[error("out of gas")]
    OutOfGas,
}

#[async_trait]
impl PluginServer<ModuleCall, ModuleCallback> for Module {
    #[instrument(skip_all)]
    async fn run_pass(
        &self,
        _: &Extensions,
        msgs: Vec<Op<VoyagerMessage>>,
    ) -> RpcResult<PassResult<VoyagerMessage>> {
        Ok(PassResult {
            optimize_further: vec![],
            ready: msgs
                .into_iter()
                .enumerate()
                .map(|(idx, msg)| {
                    Ok((
                        vec![idx],
                        match msg {
                            Op::Data(Data::IdentifiedIbcDatagram(WithChainId {
                                chain_id,
                                message,
                            })) => {
                                assert_eq!(chain_id, self.chain_id);

                                call(PluginMessage::new(
                                    self.plugin_name(),
                                    ModuleCall::SubmitTransaction(vec![
                                        IbcMessage::from_raw_datagram(message)?,
                                    ]),
                                ))
                            }
                            Op::Data(Data::IdentifiedIbcDatagramBatch(WithChainId {
                                chain_id,
                                message,
                            })) => {
                                assert_eq!(chain_id, self.chain_id);

                                call(PluginMessage::new(
                                    self.plugin_name(),
                                    ModuleCall::SubmitTransaction(
                                        message
                                            .into_iter()
                                            .map(IbcMessage::from_raw_datagram)
                                            .collect::<Result<_, _>>()?,
                                    ),
                                ))
                            }
                            _ => panic!("unexpected message: {msg:?}"),
                        },
                    ))
                })
                .collect::<RpcResult<_>>()?,
        })
    }

    #[instrument(skip_all, fields(chain_id = %self.chain_id))]
    #[allow(clippy::collapsible_match)]
    async fn call(&self, _: &Extensions, msg: ModuleCall) -> RpcResult<Op<VoyagerMessage>> {
        match msg {
            ModuleCall::SubmitTransaction(msgs) => {
                self.do_send_transaction(msgs)
                    .await
                    .map_err(|err| match &err {
                        BroadcastTxCommitError::Tx(tx_err) => match tx_err {
                            CosmosSdkError::CapabilityError(capability_error) => {
                                ErrorObject::owned(
                                    FATAL_JSONRPC_ERROR_CODE,
                                    ErrorReporter(capability_error).to_string(),
                                    None::<()>,
                                )
                            }
                            CosmosSdkError::IbcWasmError(IbcWasmError::ErrInvalidChecksum) => {
                                ErrorObject::owned(
                                    FATAL_JSONRPC_ERROR_CODE,
                                    ErrorReporter(err).to_string(),
                                    None::<()>,
                                )
                            }
                            CosmosSdkError::ClientError(ClientError::ErrClientNotFound) => {
                                ErrorObject::owned(
                                    FATAL_JSONRPC_ERROR_CODE,
                                    ErrorReporter(err).to_string(),
                                    None::<()>,
                                )
                            }
                            _ => ErrorObject::owned(-1, ErrorReporter(err).to_string(), None::<()>),
                        },
                        _ => ErrorObject::owned(-1, ErrorReporter(err).to_string(), None::<()>),
                    })
            }
        }
    }

    #[instrument(skip_all)]
    async fn callback(
        &self,
        _: &Extensions,
        cb: ModuleCallback,
        _data: VecDeque<Data>,
    ) -> RpcResult<Op<VoyagerMessage>> {
        match cb {}
    }
}

fn process_msgs(
    msgs: Vec<IbcMessage>,
    signer: &CosmosSigner,
    ibc_union_contract_address: String,
) -> Vec<(IbcMessage, protos::google::protobuf::Any)> {
    msgs.into_iter()
        .map(|msg| {
            let encoded = match msg.clone() {
                IbcMessage::IbcV1(msg) => match msg {
                    ibc_v1::IbcMessage::ConnectionOpenInit(message) => {
                        mk_any(&protos::ibc::core::connection::v1::MsgConnectionOpenInit {
                            client_id: message.client_id.to_string(),
                            counterparty: Some(message.counterparty.into()),
                            version: Some(message.version.into()),
                            signer: signer.to_string(),
                            delay_period: message.delay_period,
                        })
                    }
                    ibc_v1::IbcMessage::ConnectionOpenTry(message) => {
                        mk_any(&protos::ibc::core::connection::v1::MsgConnectionOpenTry {
                            client_id: message.client_id.to_string(),
                            counterparty: Some(message.counterparty.into()),
                            delay_period: message.delay_period,
                            counterparty_versions: message
                                .counterparty_versions
                                .into_iter()
                                .map(Into::into)
                                .collect(),
                            proof_height: Some(message.proof_height.into()),
                            proof_init: message.proof_init.into(),
                            signer: signer.to_string(),
                            ..Default::default()
                        })
                    }
                    #[allow(deprecated)]
                    ibc_v1::IbcMessage::ConnectionOpenAck(message) => {
                        mk_any(&protos::ibc::core::connection::v1::MsgConnectionOpenAck {
                            client_state: Some(
                                protos::google::protobuf::Any::decode(&*message.client_state)
                                    .expect("value should be encoded as an `Any`"),
                            ),
                            proof_height: Some(message.proof_height.into()),
                            proof_client: message.proof_client.into(),
                            proof_consensus: message.proof_consensus.into(),
                            consensus_height: Some(message.consensus_height.into()),
                            signer: signer.to_string(),
                            host_consensus_state_proof: vec![],
                            connection_id: message.connection_id.to_string(),
                            counterparty_connection_id: message
                                .counterparty_connection_id
                                .to_string(),
                            version: Some(message.version.into()),
                            proof_try: message.proof_try.into(),
                        })
                    }
                    ibc_v1::IbcMessage::ConnectionOpenConfirm(message) => mk_any(
                        &protos::ibc::core::connection::v1::MsgConnectionOpenConfirm {
                            connection_id: message.connection_id.to_string(),
                            proof_ack: message.proof_ack.into(),
                            proof_height: Some(message.proof_height.into()),
                            signer: signer.to_string(),
                        },
                    ),
                    ibc_v1::IbcMessage::ChannelOpenInit(message) => {
                        mk_any(&protos::ibc::core::channel::v1::MsgChannelOpenInit {
                            port_id: message.port_id.to_string(),
                            channel: Some(message.channel.into()),
                            signer: signer.to_string(),
                        })
                    }
                    ibc_v1::IbcMessage::ChannelOpenTry(message) => {
                        mk_any(&protos::ibc::core::channel::v1::MsgChannelOpenTry {
                            port_id: message.port_id.to_string(),
                            channel: Some(message.channel.into()),
                            counterparty_version: message.counterparty_version,
                            proof_init: message.proof_init.into(),
                            proof_height: Some(message.proof_height.into()),
                            signer: signer.to_string(),
                            ..Default::default()
                        })
                    }
                    ibc_v1::IbcMessage::ChannelOpenAck(message) => {
                        mk_any(&protos::ibc::core::channel::v1::MsgChannelOpenAck {
                            port_id: message.port_id.to_string(),
                            channel_id: message.channel_id.to_string(),
                            counterparty_version: message.counterparty_version,
                            counterparty_channel_id: message.counterparty_channel_id.to_string(),
                            proof_try: message.proof_try.into(),
                            proof_height: Some(message.proof_height.into()),
                            signer: signer.to_string(),
                        })
                    }
                    ibc_v1::IbcMessage::ChannelOpenConfirm(message) => {
                        mk_any(&protos::ibc::core::channel::v1::MsgChannelOpenConfirm {
                            port_id: message.port_id.to_string(),
                            channel_id: message.channel_id.to_string(),
                            proof_height: Some(message.proof_height.into()),
                            signer: signer.to_string(),
                            proof_ack: message.proof_ack.into(),
                        })
                    }
                    ibc_v1::IbcMessage::RecvPacket(message) => {
                        mk_any(&protos::ibc::core::channel::v1::MsgRecvPacket {
                            packet: Some(message.packet.into()),
                            proof_height: Some(message.proof_height.into()),
                            signer: signer.to_string(),
                            proof_commitment: message.proof_commitment.into(),
                        })
                    }
                    ibc_v1::IbcMessage::AcknowledgePacket(message) => {
                        mk_any(&protos::ibc::core::channel::v1::MsgAcknowledgement {
                            packet: Some(message.packet.into()),
                            acknowledgement: message.acknowledgement.into(),
                            proof_acked: message.proof_acked.into(),
                            proof_height: Some(message.proof_height.into()),
                            signer: signer.to_string(),
                        })
                    }
                    ibc_v1::IbcMessage::TimeoutPacket(message) => {
                        mk_any(&protos::ibc::core::channel::v1::MsgTimeout {
                            packet: Some(message.packet.into()),
                            proof_unreceived: message.proof_unreceived,
                            proof_height: Some(message.proof_height.into()),
                            next_sequence_recv: message.next_sequence_recv.get(),
                            signer: signer.to_string(),
                        })
                    }
                    ibc_v1::IbcMessage::CreateClient(message) => {
                        mk_any(&protos::ibc::core::client::v1::MsgCreateClient {
                            client_state: Some(
                                protos::google::protobuf::Any::decode(&*message.msg.client_state)
                                    .expect("value should be encoded as an `Any`"),
                            ),
                            consensus_state: Some(
                                protos::google::protobuf::Any::decode(
                                    &*message.msg.consensus_state,
                                )
                                .expect("value should be encoded as an `Any`"),
                            ),
                            signer: signer.to_string(),
                        })
                    }
                    ibc_v1::IbcMessage::UpdateClient(message) => {
                        mk_any(&protos::ibc::core::client::v1::MsgUpdateClient {
                            signer: signer.to_string(),
                            client_id: message.client_id.to_string(),
                            client_message: Some(
                                protos::google::protobuf::Any::decode(&*message.client_message)
                                    .expect("value should be encoded as an `Any`"),
                            ),
                        })
                    }
                },
                IbcMessage::IbcUnion(msg) => match msg {
                    ibc_union::IbcMsg::CreateClient(msg_create_client) => {
                        mk_any(&protos::cosmwasm::wasm::v1::MsgExecuteContract {
                            sender: signer.to_string(),
                            contract: ibc_union_contract_address.clone(),
                            msg: serde_json::to_vec(&union_ibc_msg::msg::ExecuteMsg::CreateClient(
                                ibc_solidity::cosmwasm::types::ibc::MsgCreateClient {
                                    clientType: msg_create_client.client_type.to_string(),
                                    clientStateBytes: msg_create_client.client_state_bytes.into(),
                                    consensusStateBytes: msg_create_client
                                        .consensus_state_bytes
                                        .into(),
                                    relayer: signer.to_string(),
                                },
                            ))
                            .unwrap(),
                            funds: vec![],
                        })
                    }
                    ibc_union::IbcMsg::UpdateClient(_msg_update_client) => todo!(),
                    ibc_union::IbcMsg::ConnectionOpenInit(msg_connection_open_init) => {
                        mk_any(&protos::cosmwasm::wasm::v1::MsgExecuteContract {
                            sender: signer.to_string(),
                            contract: ibc_union_contract_address.clone(),
                            msg: serde_json::to_vec(
                                &union_ibc_msg::msg::ExecuteMsg::ConnectionOpenInit(
                                    ibc_solidity::cosmwasm::types::ibc::MsgConnectionOpenInit {
                                        clientId: msg_connection_open_init.client_id,
                                        counterpartyClientId: msg_connection_open_init
                                            .counterparty_client_id,
                                        relayer: signer.to_string(),
                                    },
                                ),
                            )
                            .unwrap(),
                            funds: vec![],
                        })
                    }
                    ibc_union::IbcMsg::ConnectionOpenTry(_msg_connection_open_try) => todo!(),
                    ibc_union::IbcMsg::ConnectionOpenAck(_msg_connection_open_ack) => todo!(),
                    ibc_union::IbcMsg::ConnectionOpenConfirm(_msg_connection_open_confirm) => {
                        todo!()
                    }
                    ibc_union::IbcMsg::ChannelOpenInit(_msg_channel_open_init) => todo!(),
                    ibc_union::IbcMsg::ChannelOpenTry(_msg_channel_open_try) => todo!(),
                    ibc_union::IbcMsg::ChannelOpenAck(_msg_channel_open_ack) => todo!(),
                    ibc_union::IbcMsg::ChannelOpenConfirm(_msg_channel_open_confirm) => todo!(),
                    ibc_union::IbcMsg::ChannelCloseInit(_msg_channel_close_init) => todo!(),
                    ibc_union::IbcMsg::ChannelCloseConfirm(_msg_channel_close_confirm) => todo!(),
                    ibc_union::IbcMsg::PacketRecv(_msg_packet_recv) => todo!(),
                    ibc_union::IbcMsg::PacketAcknowledgement(_msg_packet_acknowledgement) => {
                        todo!()
                    }
                    ibc_union::IbcMsg::PacketTimeout(_msg_packet_timeout) => todo!(),
                    ibc_union::IbcMsg::IntentPacketRecv(_msg_intent_packet_recv) => todo!(),
                    ibc_union::IbcMsg::BatchSend(_msg_batch_send) => todo!(),
                    ibc_union::IbcMsg::BatchAcks(_msg_batch_acks) => todo!(),
                },
            };

            (msg, encoded)
        })
        .collect()
}
