use std::{collections::VecDeque, sync::Arc};

use chain_utils::{
    ethereum::{EthereumSignerMiddleware, IbcHandlerErrors},
    keyring::{ConcurrentKeyring, KeyringConfig, KeyringEntry},
};
use contracts::{
    ibc_handler::{
        AcknowledgePacketCall, ChannelOpenAckCall, ChannelOpenConfirmCall, ChannelOpenInitCall,
        ChannelOpenTryCall, ConnectionOpenAckCall, ConnectionOpenConfirmCall,
        ConnectionOpenInitCall, ConnectionOpenTryCall, CreateClientCall, IBCHandler,
        RecvPacketCall, TimeoutPacketCall, UpdateClientCall,
    },
    multicall::{Call3, Multicall, MulticallResultFilter},
};
use ethers::{
    abi::AbiDecode,
    contract::{ContractError, EthCall, EthLogDecode, FunctionCall},
    core::k256::ecdsa,
    middleware::{
        nonce_manager::NonceManagerError, signer::SignerMiddlewareError, NonceManagerMiddleware,
        SignerMiddleware,
    },
    providers::{Middleware, Provider, ProviderError, Ws, WsClientError},
    signers::{LocalWallet, Signer},
    types::TransactionReceipt,
    utils::secret_key_to_address,
};
use jsonrpsee::{
    core::{async_trait, RpcResult},
    types::ErrorObject,
};
use queue_msg::{defer_relative, effect, seq, Op};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, error_span, info, info_span, instrument, warn, Instrument};
use unionlabs::{
    hash::{H160, H256},
    uint::U256,
    ErrorReporter,
};
use voyager_message::{
    data::Data,
    effect::{log_msg, Effect, Msg, MsgCreateClientData, WithChainId},
    plugin::{PluginInfo, PluginKind, PluginModuleServer, TransactionSubmissionModuleServer},
    run_module_server, VoyagerMessage,
};

use crate::{aggregate::ModuleAggregate, data::ModuleData, fetch::ModuleFetch};

pub mod aggregate;
pub mod data;
pub mod fetch;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    run_module_server(Module::new, TransactionSubmissionModuleServer::into_rpc).await
}

#[derive(Debug, Clone)]
pub struct Module {
    pub chain_id: U256,

    /// The address of the `IBCHandler` smart contract.
    pub ibc_handler_address: H160,
    pub multicall_address: H160,

    pub provider: Provider<Ws>,

    pub keyring: ConcurrentKeyring<H160, LocalWallet>,

    pub max_gas_price: Option<U256>,
    pub legacy: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// The address of the `IBCHandler` smart contract.
    pub ibc_handler_address: H160,
    pub multicall_address: H160,

    /// The RPC endpoint for the execution chain.
    pub eth_rpc_api: String,

    pub keyring: KeyringConfig,

    #[serde(default)]
    pub max_gas_price: Option<U256>,

    #[serde(default)]
    pub legacy: bool,
}

impl Module {
    fn plugin_name(&self) -> String {
        pub const PLUGIN_NAME: &str = env!("CARGO_PKG_NAME");

        format!("{PLUGIN_NAME}/{}", self.chain_id)
    }

    pub async fn new(config: Config) -> Result<Self, InitError> {
        let provider = Provider::new(Ws::connect(config.eth_rpc_api).await?);

        let chain_id = provider.get_chainid().await?;

        Ok(Self {
            chain_id: U256(chain_id),
            ibc_handler_address: config.ibc_handler_address,
            multicall_address: config.multicall_address,
            provider,
            keyring: ConcurrentKeyring::new(
                config.keyring.name,
                config.keyring.keys.into_iter().map(|config| {
                    let signing_key = <ecdsa::SigningKey as bip32::PrivateKey>::from_bytes(
                        &config.value().as_slice().try_into().unwrap(),
                    )
                    .unwrap();

                    let address = secret_key_to_address(&signing_key);

                    let wallet =
                        LocalWallet::new_with_signer(signing_key, address, chain_id.as_u64());

                    KeyringEntry {
                        name: config.name(),
                        address: address.into(),
                        signer: wallet,
                    }
                }),
            ),
            max_gas_price: config.max_gas_price,
            legacy: config.legacy,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum InitError {
    #[error("unable to connect to websocket")]
    Ws(#[from] WsClientError),
    #[error("provider error")]
    Provider(#[from] ProviderError),
}

#[async_trait]
impl PluginModuleServer<ModuleData, ModuleFetch, ModuleAggregate> for Module {
    #[instrument(skip_all, fields(chain_id = %self.chain_id))]
    async fn info(&self) -> RpcResult<PluginInfo> {
        Ok(PluginInfo {
            name: self.plugin_name(),
            kind: Some(PluginKind::Transaction),
            interest_filter: None,
        })
    }

    #[instrument(skip_all, fields(chain_id = %self.chain_id))]
    fn handle_aggregate(
        &self,
        aggregate: ModuleAggregate,
        _data: VecDeque<Data<ModuleData>>,
    ) -> RpcResult<Op<VoyagerMessage<ModuleData, ModuleFetch, ModuleAggregate>>> {
        match aggregate {}
    }

    #[instrument(skip_all, fields(chain_id = %self.chain_id))]
    async fn handle_fetch(
        &self,
        msg: ModuleFetch,
    ) -> RpcResult<Op<VoyagerMessage<ModuleData, ModuleFetch, ModuleAggregate>>> {
        match msg {}
    }
}

#[async_trait]
impl TransactionSubmissionModuleServer<ModuleData, ModuleFetch, ModuleAggregate> for Module {
    fn register(&self) -> RpcResult<String> {
        Ok(self.chain_id.to_string())
    }

    async fn send_transaction(
        &self,
        mut msgs: Vec<Msg>,
    ) -> RpcResult<Op<VoyagerMessage<ModuleData, ModuleFetch, ModuleAggregate>>> {
        let res = self.keyring
        .with({
            let msgs = msgs.clone();
            move |wallet| -> _ {
                let signer = Arc::new(SignerMiddleware::new(
                    NonceManagerMiddleware::new(self.provider.clone(), wallet.address()),
                    wallet.clone(),
                ));

                async move {
                    if let Some(max_gas_price) = self.max_gas_price {
                        let gas_price = U256::from(self.provider
                            .get_gas_price()
                            .await
                            .expect("unable to fetch gas price"));

                        if gas_price > max_gas_price {
                            warn!(%max_gas_price, %gas_price, "gas price is too high");
                            return Err(TxSubmitError::GasPriceTooHigh {
                                max: max_gas_price,
                                price: gas_price
                            })
                        }
                    }

                    let multicall = Multicall::new(ethers::types::H160::from(self.multicall_address), signer.clone());

                    let msgs = process_msgs(
                        msgs,
                        &IBCHandler::new(self.ibc_handler_address, signer),
                        wallet.address().into(),
                    );

                    let msg_names = msgs
                        .iter()
                        .map(|x| (x.0.clone(), x.1.function.name.clone()))
                        .collect::<Vec<_>>();

                    let call = multicall.multicall(
                        msgs.into_iter()
                            .map(|(_, x): (_, FunctionCall<_, _, _>)| Call3 {
                                target: self.ibc_handler_address.into(),
                                allow_failure: true,
                                call_data: x.calldata().expect("is a contract call"),
                            })
                            .collect(),
                    );

                    let call = if self.legacy { call.legacy() } else { call };

                    let msg_name = call.function.name.clone();

                    info!("submitting evm tx");

                    match call.estimate_gas().await {
                        Ok(estimated_gas) => {
                            debug!(
                                %estimated_gas,
                                "gas estimation"
                            );

                            // TODO: config
                            match call.gas(estimated_gas + (estimated_gas / 10)).send().await {
                                Ok(ok) => {
                                    let tx_hash = ok.tx_hash();
                                    async move {
                                        let tx_rcp: TransactionReceipt = ok.await?.ok_or(TxSubmitError::NoTxReceipt)?;

                                        let result = MulticallResultFilter::decode_log(
                                            &ethers::abi::RawLog::from(
                                                tx_rcp
                                                    .logs
                                                    .last()
                                                    .expect("multicall event should be last log")
                                                    .clone(),
                                            ),
                                        )
                                        .expect("unable to decode multicall result log");

                                        info!(
                                            gas_used = %tx_rcp.gas_used.unwrap_or_default(),
                                            batch.size = msg_names.len(),
                                            "submitted batched evm messages"
                                        );

                                        for (idx, (result, (msg, msg_name))) in result.0.into_iter().zip(msg_names).enumerate() {
                                            if result.success {
                                                info_span!(
                                                    "evm tx",
                                                    msg = msg_name,
                                                    %idx,
                                                )
                                                .in_scope(|| log_msg(&self.chain_id.to_string(), msg));
                                            } else if let Ok(known_revert) =
                                                IbcHandlerErrors::decode(&*result.return_data.clone())
                                            {
                                                error_span!(
                                                    "evm message failed",
                                                    msg = %msg_name,
                                                    %idx,
                                                    revert = ?known_revert,
                                                    well_known = true,
                                                )
                                                .in_scope(|| log_msg(&self.chain_id.to_string(), msg));
                                            } else if result.return_data.is_empty() {
                                                error_span!(
                                                    "evm message failed",
                                                    msg = %msg_name,
                                                    %idx,
                                                    revert = %serde_utils::to_hex(result.return_data),
                                                    well_known = false,
                                                )
                                                .in_scope(|| log_msg(&self.chain_id.to_string(), msg));

                                                return Err(TxSubmitError::EmptyRevert)
                                            } else {
                                                error_span!(
                                                    "evm message failed",
                                                    msg = %msg_name,
                                                    %idx,
                                                    revert = %serde_utils::to_hex(result.return_data),
                                                    well_known = false,
                                                )
                                                .in_scope(|| log_msg(&self.chain_id.to_string(), msg));
                                            }
                                        }

                                        Ok(())
                                    }
                                    .instrument(info_span!(
                                        "evm tx",
                                        tx_hash = %H256::from(tx_hash),
                                    )).await
                                }
                                // Err(ContractError::Revert(revert)) => {
                                //     error!(?revert, "evm transaction failed");
                                //     let err = <IbcHandlerErrors as ethers::abi::AbiDecode>::decode(
                                //         revert.clone(),
                                //     )
                                //     .map_err(|_| TxSubmitError::InvalidRevert(revert.clone()))?;
                                //     error!(
                                //         msg = %msg_name,
                                //         ?revert,
                                //         ?err,
                                //         "evm transaction failed"
                                //     );

                                //     Ok(())
                                // }
                                Err(ContractError::ProviderError {
                                    e: ProviderError::JsonRpcClientError(e),
                                })
                                | Err(ContractError::MiddlewareError {
                                    e:
                                        SignerMiddlewareError::MiddlewareError(NonceManagerError::MiddlewareError(
                                            ProviderError::JsonRpcClientError(e),
                                        )),
                                }) if e.as_error_response().is_some_and(|e| {
                                    e.message
                                        .contains("insufficient funds for gas * price + value")
                                }) =>
                                {
                                    error!("out of gas");
                                    Err(TxSubmitError::OutOfGas)
                                }
                                err => {
                                    panic!("evm transaction non-recoverable failure: {err:?}")
                                }
                            }
                        }
                        Err(err) => {
                            let _span = error_span!(
                                "tx error",
                                msg = %msg_name,
                                err = %ErrorReporter(&err)
                            );

                            match err {
                                ContractError::Revert(revert) => {
                                    error!(?revert, "evm gas estimation failed");

                                    match <IbcHandlerErrors as ethers::abi::AbiDecode>::decode(
                                        &revert,
                                    ) {
                                        Ok(known_err) => {
                                            // REVIEW: Are any of these recoverable?
                                            // match known_err {
                                            //     IbcHandlerErrors::PacketErrors(_) => todo!(),
                                            //     IbcHandlerErrors::ConnectionErrors(_) => todo!(),
                                            //     IbcHandlerErrors::ChannelErrors(_) => todo!(),
                                            //     IbcHandlerErrors::ClientErrors(_) => todo!(),
                                            //     IbcHandlerErrors::CometblsClientErrors(_) => todo!(),
                                            // }

                                            error!(?revert, ?known_err, "evm estimation failed");
                                        }
                                        Err(_) => {
                                            error!(
                                                "evm estimation failed with unknown revert code"
                                            );
                                        }
                                    }

                                    Ok(())
                                }
                                _ => {
                                    error!("evm tx recoverable error");
                                    panic!();
                                }
                            }
                        }
                    }
                }
            }
        })
        .await;

        let rewrap_msg = || {
            if msgs.len() > 1 {
                Effect::Batch(WithChainId {
                    chain_id: self.chain_id.to_string(),
                    message: msgs,
                })
            } else {
                Effect::Single(WithChainId {
                    chain_id: self.chain_id.to_string(),
                    message: msgs.pop().unwrap(),
                })
            }
        };

        match res {
            Some(Ok(())) => Ok(Op::Noop),
            Some(Err(TxSubmitError::GasPriceTooHigh { .. })) => {
                Ok(seq([defer_relative(6), effect(rewrap_msg())]))
            }
            Some(Err(TxSubmitError::OutOfGas)) => {
                Ok(seq([defer_relative(12), effect(rewrap_msg())]))
            }
            Some(Err(TxSubmitError::EmptyRevert)) => {
                Ok(seq([defer_relative(12), effect(rewrap_msg())]))
            }
            Some(Err(err)) => Err(ErrorObject::owned(
                -1,
                ErrorReporter(err).to_string(),
                None::<()>,
            )),
            None => Ok(effect(rewrap_msg())),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TxSubmitError {
    #[error(transparent)]
    Contract(#[from] ContractError<EthereumSignerMiddleware>),
    #[error(transparent)]
    Provider(#[from] ProviderError),
    #[error("no tx receipt from tx")]
    NoTxReceipt,
    #[error("invalid revert code: {0}")]
    InvalidRevert(ethers::types::Bytes),
    #[error("out of gas")]
    OutOfGas,
    #[error("0x revert")]
    EmptyRevert,
    #[error("gas price is too high: max {max}, price {price}")]
    GasPriceTooHigh { max: U256, price: U256 },
}

#[allow(clippy::type_complexity)]
fn process_msgs<M: Middleware>(
    msgs: Vec<Msg>,
    ibc_handler: &IBCHandler<M>,
    relayer: H160,
) -> Vec<(Msg, FunctionCall<Arc<M>, M, ()>)> {
    pub fn mk_function_call<Call: EthCall, M: Middleware>(
        ibc_handler: &IBCHandler<M>,
        data: Call,
    ) -> ethers::contract::FunctionCall<Arc<M>, M, ()> {
        ibc_handler
            .method_hash(<Call as EthCall>::selector(), data)
            .expect("method selector is generated; qed;")
    }

    msgs.clone()
        .into_iter()
        .map(|msg| match msg.clone() {
            Msg::ConnectionOpenInit(data) => (
                msg,
                mk_function_call(
                    ibc_handler,
                    ConnectionOpenInitCall(contracts::ibc_handler::MsgConnectionOpenInit {
                        client_id: data.client_id.to_string(),
                        version: data.version.into(),
                        counterparty: data.counterparty.into(),
                        delay_period: data.delay_period,
                        relayer: relayer.into(),
                    }),
                ),
            ),
            Msg::ConnectionOpenTry(data) => (
                msg,
                mk_function_call(
                    ibc_handler,
                    ConnectionOpenTryCall(contracts::ibc_handler::MsgConnectionOpenTry {
                        counterparty: data.counterparty.into(),
                        delay_period: data.delay_period,
                        client_id: data.client_id.to_string(),
                        client_state_bytes: data.client_state.into(),
                        counterparty_versions: data
                            .counterparty_versions
                            .into_iter()
                            .map(Into::into)
                            .collect(),
                        proof_init: data.proof_init.into(),
                        proof_client: data.proof_client.into(),
                        proof_consensus: data.proof_consensus.into(),
                        proof_height: data.proof_height.into(),
                        consensus_height: data.consensus_height.into(),
                        relayer: relayer.into(),
                    }),
                ),
            ),
            Msg::ConnectionOpenAck(data) => (
                msg,
                mk_function_call(
                    ibc_handler,
                    ConnectionOpenAckCall(contracts::ibc_handler::MsgConnectionOpenAck {
                        connection_id: data.connection_id.to_string(),
                        counterparty_connection_id: data.counterparty_connection_id.to_string(),
                        version: data.version.into(),
                        client_state_bytes: data.client_state.into(),
                        proof_height: data.proof_height.into(),
                        proof_try: data.proof_try.into(),
                        proof_client: data.proof_client.into(),
                        proof_consensus: data.proof_consensus.into(),
                        consensus_height: data.consensus_height.into(),
                        relayer: relayer.into(),
                    }),
                ),
            ),
            Msg::ConnectionOpenConfirm(data) => (
                msg,
                mk_function_call(
                    ibc_handler,
                    ConnectionOpenConfirmCall(contracts::ibc_handler::MsgConnectionOpenConfirm {
                        connection_id: data.connection_id.to_string(),
                        proof_ack: data.proof_ack.into(),
                        proof_height: data.proof_height.into(),
                        relayer: relayer.into(),
                    }),
                ),
            ),
            Msg::ChannelOpenInit(data) => (
                msg,
                mk_function_call(
                    ibc_handler,
                    ChannelOpenInitCall(contracts::ibc_handler::MsgChannelOpenInit {
                        port_id: data.port_id.to_string(),
                        channel: data.channel.into(),
                        relayer: relayer.into(),
                    }),
                ),
            ),
            Msg::ChannelOpenTry(data) => (
                msg,
                mk_function_call(
                    ibc_handler,
                    ChannelOpenTryCall(contracts::ibc_handler::MsgChannelOpenTry {
                        port_id: data.port_id.to_string(),
                        channel: data.channel.into(),
                        counterparty_version: data.counterparty_version,
                        proof_init: data.proof_init.into(),
                        proof_height: data.proof_height.into(),
                        relayer: relayer.into(),
                    }),
                ),
            ),
            Msg::ChannelOpenAck(data) => (
                msg,
                mk_function_call(
                    ibc_handler,
                    ChannelOpenAckCall(contracts::ibc_handler::MsgChannelOpenAck {
                        port_id: data.port_id.to_string(),
                        channel_id: data.channel_id.to_string(),
                        counterparty_version: data.counterparty_version,
                        counterparty_channel_id: data.counterparty_channel_id.to_string(),
                        proof_try: data.proof_try.into(),
                        proof_height: data.proof_height.into(),
                        relayer: relayer.into(),
                    }),
                ),
            ),
            Msg::ChannelOpenConfirm(data) => (
                msg,
                mk_function_call(
                    ibc_handler,
                    ChannelOpenConfirmCall(contracts::ibc_handler::MsgChannelOpenConfirm {
                        port_id: data.port_id.to_string(),
                        channel_id: data.channel_id.to_string(),
                        proof_ack: data.proof_ack.into(),
                        proof_height: data.proof_height.into(),
                        relayer: relayer.into(),
                    }),
                ),
            ),
            Msg::RecvPacket(data) => (
                msg,
                mk_function_call(
                    ibc_handler,
                    RecvPacketCall(contracts::ibc_handler::MsgPacketRecv {
                        packet: data.packet.into(),
                        proof: data.proof_commitment.into(),
                        proof_height: data.proof_height.into(),
                        relayer: relayer.into(),
                    }),
                ),
            ),
            Msg::AckPacket(data) => (
                msg,
                mk_function_call(
                    ibc_handler,
                    AcknowledgePacketCall(contracts::ibc_handler::MsgPacketAcknowledgement {
                        packet: data.packet.into(),
                        acknowledgement: data.acknowledgement.into(),
                        proof: data.proof_acked.into(),
                        proof_height: data.proof_height.into(),
                        relayer: relayer.into(),
                    }),
                ),
            ),
            Msg::TimeoutPacket(data) => (
                msg,
                mk_function_call(
                    ibc_handler,
                    TimeoutPacketCall(contracts::ibc_handler::MsgPacketTimeout {
                        packet: data.packet.into(),
                        proof: data.proof_unreceived.into(),
                        proof_height: data.proof_height.into(),
                        next_sequence_recv: data.next_sequence_recv.get(),
                        relayer: relayer.into(),
                    }),
                ),
            ),
            Msg::CreateClient(MsgCreateClientData {
                msg: data,
                client_type,
            }) => (
                msg,
                mk_function_call(
                    ibc_handler,
                    CreateClientCall(contracts::shared_types::MsgCreateClient {
                        client_type: client_type.to_string(),
                        client_state_bytes: data.client_state.into(),
                        consensus_state_bytes: data.consensus_state.into(),
                        relayer: relayer.into(),
                    }),
                ),
            ),
            Msg::UpdateClient(data) => (
                msg,
                mk_function_call(
                    ibc_handler,
                    UpdateClientCall(contracts::shared_types::MsgUpdateClient {
                        client_id: data.client_id.to_string(),
                        client_message: data.client_message.into(),
                        relayer: relayer.into(),
                    }),
                ),
            ),
        })
        .collect()
}