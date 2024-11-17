// #![warn(clippy::unwrap_used)]

use std::{
    collections::VecDeque,
    error::Error,
    fmt::{Debug, Display},
    num::{NonZeroU32, NonZeroU8, ParseIntError},
    sync::Arc,
};

use dashmap::DashMap;
use ibc_events::{
    ChannelOpenAck, ChannelOpenConfirm, ChannelOpenInit, ChannelOpenTry, ClientMisbehaviour,
    ConnectionOpenAck, ConnectionOpenConfirm, ConnectionOpenInit, ConnectionOpenTry, CreateClient,
    IbcEvent, SubmitEvidence, UpdateClient,
};
use jsonrpsee::{
    core::{async_trait, RpcResult},
    types::{ErrorObject, ErrorObjectOwned},
    Extensions,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::{debug, error, info, instrument};
use unionlabs::{
    hash::{hash_v2::HexUnprefixed, H256},
    ibc::core::{
        channel::{self},
        client::height::Height,
    },
    ics24::{ChannelEndPath, ConnectionPath},
    id::{ChannelId, ClientId, ConnectionId, PortId},
    option_unwrap, parse_wasm_client_type, ErrorReporter, WasmClientType,
};
use voyager_message::{
    call::{Call, WaitForHeight},
    core::{ChainId, ClientInfo, ClientType, QueryHeight},
    data::{ChainEvent, Data},
    ibc_union::{self, IbcUnion},
    ibc_v1::{self, IbcV1},
    into_value,
    module::{PluginInfo, PluginServer},
    rpc::{json_rpc_error_to_error_object, missing_state, VoyagerRpcClient},
    ExtensionsExt, IbcSpec, Plugin, PluginMessage, RawClientId, VoyagerClient, VoyagerMessage,
};
use voyager_vm::{call, conc, data, pass::PassResult, seq, BoxDynError, Op};

use crate::{
    call::{FetchBlocks, FetchTransactions, MakeChainEvent, ModuleCall, RawEvent},
    callback::ModuleCallback,
};

pub mod call;
pub mod callback;
pub mod data;

const PER_PAGE_LIMIT: NonZeroU8 = option_unwrap!(NonZeroU8::new(10));

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    Module::run().await
}

#[derive(clap::Subcommand)]
pub enum Cmd {
    ChainId,
    LatestHeight,
}

#[derive(Debug, Clone)]
pub struct Module {
    pub chain_id: ChainId,
    pub chain_revision: u64,

    pub tm_client: cometbft_rpc::Client,
    pub grpc_url: String,

    pub checksum_cache: Arc<DashMap<H256, WasmClientType>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub chain_id: ChainId,
    pub ws_url: String,
    pub grpc_url: String,
}

impl Plugin for Module {
    type Call = ModuleCall;
    type Callback = ModuleCallback;

    type Config = Config;
    type Cmd = Cmd;

    async fn new(config: Self::Config) -> Result<Self, BoxDynError> {
        let tm_client = cometbft_rpc::Client::new(config.ws_url).await?;

        let chain_id = tm_client.status().await?.node_info.network;

        let chain_revision = chain_id
            .split('-')
            .last()
            .ok_or_else(|| ChainIdParseError {
                found: chain_id.clone(),
                source: None,
            })?
            .parse()
            .map_err(|err| ChainIdParseError {
                found: chain_id.clone(),
                source: Some(err),
            })?;

        Ok(Self {
            tm_client,
            chain_id: ChainId::new(chain_id),
            chain_revision,
            grpc_url: config.grpc_url,
            checksum_cache: Arc::new(DashMap::default()),
        })
    }

    fn info(config: Self::Config) -> PluginInfo {
        PluginInfo {
            name: plugin_name(&config.chain_id),
            interest_filter: format!(
                r#"[.. | ."@type"? == "fetch_blocks" and ."@value".chain_id == "{}"] | any"#,
                config.chain_id
            ),
        }
    }

    async fn cmd(config: Self::Config, cmd: Self::Cmd) {
        let module = Self::new(config).await.unwrap();

        match cmd {
            Cmd::ChainId => println!("{}", module.chain_id),
            Cmd::LatestHeight => println!("{}", module.latest_height().await.unwrap()),
        }
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

    #[must_use]
    pub fn make_height(&self, height: u64) -> Height {
        Height::new_with_revision(self.chain_revision, height)
    }

    async fn client_type_of_checksum(&self, checksum: H256) -> RpcResult<Option<WasmClientType>> {
        if let Some(ty) = self.checksum_cache.get(&checksum) {
            debug!(
                %checksum,
                ty = ?*ty,
                "cache hit for checksum"
            );

            return Ok(Some(*ty));
        };

        info!(
            %checksum,
            "cache miss for checksum"
        );

        let bz = protos::ibc::lightclients::wasm::v1::query_client::QueryClient::connect(
            self.grpc_url.clone(),
        )
        .await
        .map_err(rpc_error(
            "error connecting to grpc server",
            Some(json!({
                "grpc_url": self.grpc_url
            })),
        ))?
        .code(protos::ibc::lightclients::wasm::v1::QueryCodeRequest {
            checksum: checksum.into_encoding::<HexUnprefixed>().to_string(),
        })
        .await
        .map_err(rpc_error(
            "error querying wasm code",
            Some(json!({
                "checksum": checksum,
                "grpc_url": self.grpc_url
            })),
        ))?
        .into_inner()
        .data;

        match parse_wasm_client_type(bz) {
            Ok(Some(ty)) => {
                info!(
                    %checksum,
                    ?ty,
                    "parsed checksum"
                );

                self.checksum_cache.insert(checksum, ty);

                Ok(Some(ty))
            }
            Ok(None) => Ok(None),
            Err(err) => {
                error!(
                    %checksum,
                    %err,
                    "unable to parse wasm client type"
                );

                Ok(None)
            }
        }
    }

    #[instrument(skip_all, fields(%client_id))]
    async fn checksum_of_client_id(&self, client_id: ClientId) -> RpcResult<H256> {
        type WasmClientState = protos::ibc::lightclients::wasm::v1::ClientState;

        let client_state = protos::ibc::core::client::v1::query_client::QueryClient::connect(
            self.grpc_url.clone(),
        )
        .await
        .map_err(rpc_error(
            "error connecting to grpc server",
            Some(json!({ "client_id": client_id })),
        ))?
        .client_state(protos::ibc::core::client::v1::QueryClientStateRequest {
            client_id: client_id.to_string(),
        })
        .await
        .map_err(rpc_error(
            "error querying client state",
            Some(json!({ "client_id": client_id })),
        ))?
        .into_inner()
        .client_state
        .ok_or_else(|| {
            // lol
            rpc_error(
                "error fetching client state",
                Some(json!({ "client_id": client_id })),
            )(&*Box::<dyn Error>::from("client state field is empty"))
        })?;

        assert!(
            client_state.type_url == <WasmClientState as prost::Name>::type_url(),
            "attempted to get the wasm blob checksum of a non-wasm \
            light client. this is a bug, please report this at \
            `https://github.com/unionlabs/union`."
        );

        // NOTE: We only need the checksum, so we don't need to decode the inner state contained in .data
        <WasmClientState as prost::Message>::decode(&*client_state.value)
            .map_err(rpc_error(
                "error decoding client state",
                Some(json!({ "client_id": client_id })),
            ))?
            .checksum
            .try_into()
            .map_err(rpc_error(
                "invalid checksum",
                Some(json!({ "client_id": client_id })),
            ))
    }

    // async fn fetch_connection(&self, connection_id: ConnectionId) -> (ConnectionEnd, Height) {
    //     let inner = protos::ibc::core::connection::v1::query_client::QueryClient::connect(
    //         self.grpc_url.clone(),
    //     )
    //     .await
    //     .unwrap()
    //     .connection(protos::ibc::core::connection::v1::QueryConnectionRequest {
    //         connection_id: connection_id.to_string(),
    //     })
    //     .await
    //     .unwrap()
    //     .into_inner();

    //     (
    //         inner.connection.unwrap().try_into().unwrap(),
    //         inner.proof_height.unwrap().into(),
    //     )
    // }

    // async fn fetch_client(&self, client_id: ClientId) -> (Vec<u8>, Height) {
    //     let inner = protos::ibc::core::client::v1::query_client::QueryClient::connect(
    //         self.grpc_url.clone(),
    //     )
    //     .await
    //     .unwrap()
    //     .client_state(protos::ibc::core::client::v1::QueryClientStateRequest {
    //         client_id: client_id.to_string(),
    //     })
    //     .await
    //     .unwrap()
    //     .into_inner();

    //     (
    //         inner.client_state.unwrap().try_into().unwrap(),
    //         inner.proof_height.unwrap().into(),
    //     )
    // }

    async fn latest_height(&self) -> Result<Height, cometbft_rpc::JsonRpcError> {
        let commit_response = self.tm_client.commit(None).await?;

        let mut height = commit_response
            .signed_header
            .header
            .height
            .inner()
            .try_into()
            .expect("value is >= 0; qed;");

        if !commit_response.canonical {
            debug!("commit is not canonical, latest finalized height is the previous block");
            height -= 1;
        }

        debug!(height, "latest height");

        Ok(self.make_height(height))
    }

    #[allow(clippy::too_many_arguments)] // pls
    async fn make_packet_metadata(
        &self,
        event_height: Height,
        self_connection_id: ConnectionId,
        self_port_id: PortId,
        self_channel_id: ChannelId,
        other_port_id: PortId,
        other_channel_id: ChannelId,
        voyager_rpc_client: &VoyagerClient,
    ) -> RpcResult<(
        ChainId,
        ClientInfo,
        ibc_v1::ChannelMetadata,
        ibc_v1::ChannelMetadata,
        channel::order::Order,
    )> {
        let self_connection = voyager_rpc_client
            .query_spec_ibc_state(
                self.chain_id.clone(),
                event_height.into(),
                ConnectionPath {
                    connection_id: self_connection_id.clone(),
                },
            )
            .await?
            .state
            .ok_or_else(missing_state("connection must exist", None))?;

        let client_info = voyager_rpc_client
            .client_info(
                self.chain_id.clone(),
                IbcV1::ID,
                RawClientId::new(self_connection.client_id.clone()),
            )
            .await
            .map_err(json_rpc_error_to_error_object)?;

        let client_meta = voyager_rpc_client
            .client_meta(
                self.chain_id.clone(),
                IbcV1::ID,
                event_height.into(),
                RawClientId::new(self_connection.client_id.clone()),
            )
            .await
            .map_err(json_rpc_error_to_error_object)?;

        let this_channel = voyager_rpc_client
            .query_spec_ibc_state(
                self.chain_id.clone(),
                event_height.into(),
                ChannelEndPath {
                    port_id: self_port_id.clone(),
                    channel_id: self_channel_id.clone(),
                },
            )
            .await?
            .state
            .ok_or_else(missing_state("channel must exist", None))?;

        let counterparty_channel = voyager_rpc_client
            .query_spec_ibc_state(
                client_meta.chain_id.clone(),
                QueryHeight::Latest,
                ChannelEndPath {
                    port_id: other_port_id.clone(),
                    channel_id: other_channel_id.clone(),
                },
            )
            .await?
            .state
            .ok_or_else(missing_state("channel must exist", None))?;

        let source_channel = ibc_v1::ChannelMetadata {
            port_id: self_port_id.clone(),
            channel_id: self_channel_id.clone(),
            version: this_channel.version,
            connection: ibc_v1::ConnectionMetadata {
                client_id: self_connection.client_id,
                connection_id: self_connection_id.clone(),
            },
        };
        let destination_channel = ibc_v1::ChannelMetadata {
            port_id: other_port_id.clone(),
            channel_id: other_channel_id.clone(),
            version: counterparty_channel.version,
            connection: ibc_v1::ConnectionMetadata {
                client_id: self_connection.counterparty.client_id,
                connection_id: self_connection
                    .counterparty
                    .connection_id
                    .expect("counterparty connection id should be set"),
            },
        };

        Ok((
            client_meta.chain_id,
            client_info,
            source_channel,
            destination_channel,
            this_channel.ordering,
        ))
    }
}

#[derive(Debug, thiserror::Error)]
#[error("unable to parse chain id: expected format `<chain>-<revision-number>`, found `{found}`")]
pub struct ChainIdParseError {
    found: String,
    #[source]
    source: Option<ParseIntError>,
}

#[async_trait]
impl PluginServer<ModuleCall, ModuleCallback> for Module {
    #[instrument(skip_all, fields(chain_id = %self.chain_id))]
    async fn run_pass(
        &self,
        _: &Extensions,
        msgs: Vec<Op<VoyagerMessage>>,
    ) -> RpcResult<PassResult<VoyagerMessage>> {
        Ok(PassResult {
            optimize_further: vec![],
            ready: msgs
                .into_iter()
                .map(|op| match op {
                    Op::Call(Call::FetchBlocks(fetch)) if fetch.chain_id == self.chain_id => {
                        call(PluginMessage::new(
                            self.plugin_name(),
                            ModuleCall::from(FetchBlocks {
                                height: fetch.start_height,
                            }),
                        ))
                    }
                    op => op,
                })
                .enumerate()
                .map(|(i, op)| (vec![i], op))
                .collect(),
        })
    }

    #[instrument(skip_all, fields(chain_id = %self.chain_id))]
    async fn callback(
        &self,
        _: &Extensions,
        cb: ModuleCallback,
        _data: VecDeque<Data>,
    ) -> RpcResult<Op<VoyagerMessage>> {
        match cb {}
    }

    #[instrument(skip_all, fields(chain_id = %self.chain_id))]
    async fn call(&self, e: &Extensions, msg: ModuleCall) -> RpcResult<Op<VoyagerMessage>> {
        match msg {
            ModuleCall::FetchTransactions(FetchTransactions { height, page }) => {
                info!(%height, %page, "fetching events in block");

                let response = self
                    .tm_client
                    .tx_search(
                        format!("tx.height={}", height.height()),
                        false,
                        page,
                        PER_PAGE_LIMIT,
                        cometbft_rpc::rpc_types::Order::Desc,
                    )
                    .await
                    .map_err(rpc_error(
                        format_args!("error fetching transactions at height {height}"),
                        Some(json!({ "height": height })),
                    ))?;

                Ok(conc(
                    response
                        .txs
                        .into_iter()
                        .flat_map(|txr| {
                            txr.tx_result.events.into_iter().filter_map(move |event| {
                                debug!(%event.ty, "observed event");
                                IbcEvent::try_from_tendermint_event(event.clone())
                                    .map(|r| r.map(RawEvent::IbcV1))
                                    .or_else(|| {
                                        ibc_events::union_ibc::IbcEvent::try_from_tendermint_event(
                                            event,
                                        )
                                        .map(|r| r.map(RawEvent::IbcUnion))
                                    })
                                    .map(|event| event.map(|event| (event, txr.hash)))
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|err| {
                            ErrorObject::owned(
                                -1,
                                ErrorReporter(err).to_string(),
                                Some(json!({
                                    "height": height,
                                    "page": page
                                })),
                            )
                        })?
                        .into_iter()
                        .map(|(ibc_event, tx_hash)| {
                            debug!(event = %ibc_event.name(), "observed IBC event");
                            call(PluginMessage::new(
                                self.plugin_name(),
                                ModuleCall::from(MakeChainEvent {
                                    height,
                                    tx_hash: tx_hash.into_encoding(),
                                    event: ibc_event,
                                }),
                            ))
                        })
                        .chain(
                            ((page.get() * PER_PAGE_LIMIT.get() as u32) < response.total_count)
                                .then(|| {
                                    call(PluginMessage::new(
                                        self.plugin_name(),
                                        ModuleCall::from(FetchTransactions {
                                            height,
                                            page: page.checked_add(1).expect("too many pages?"),
                                        }),
                                    ))
                                }),
                        ),
                ))
            }
            ModuleCall::FetchBlocks(FetchBlocks { height }) => Ok(conc([
                call(PluginMessage::new(
                    self.plugin_name(),
                    ModuleCall::from(FetchTransactions {
                        height,
                        page: const { option_unwrap!(NonZeroU32::new(1_u32)) },
                    }),
                )),
                seq([
                    // TODO: Make this a config param
                    call(WaitForHeight {
                        chain_id: self.chain_id.clone(),
                        height: height.increment(),
                        finalized: true,
                    }),
                    call(PluginMessage::new(
                        self.plugin_name(),
                        ModuleCall::from(FetchBlocks {
                            height: height.increment(),
                        }),
                    )),
                ]),
            ])),
            ModuleCall::MakeChainEvent(MakeChainEvent {
                height,
                tx_hash,
                event,
            }) => {
                // events at height N are provable at height N+k where k<0
                let provable_height = height.increment();
                let voyager_client = e.try_get::<VoyagerClient>()?;

                match event {
                    RawEvent::IbcV1(IbcEvent::SubmitEvidence(SubmitEvidence { .. })) => {
                        // TODO: Not sure how to handle this one, since it only contains the hash
                        panic!()
                    }
                    RawEvent::IbcV1(
                        IbcEvent::CreateClient(CreateClient { ref client_id, .. })
                        | IbcEvent::UpdateClient(UpdateClient { ref client_id, .. })
                        | IbcEvent::ClientMisbehaviour(ClientMisbehaviour { ref client_id, .. })
                        | IbcEvent::ConnectionOpenInit(ConnectionOpenInit { ref client_id, .. })
                        | IbcEvent::ConnectionOpenTry(ConnectionOpenTry { ref client_id, .. })
                        | IbcEvent::ConnectionOpenAck(ConnectionOpenAck { ref client_id, .. })
                        | IbcEvent::ConnectionOpenConfirm(ConnectionOpenConfirm {
                            ref client_id,
                            ..
                        }),
                    ) => {
                        let client_info = voyager_client
                            .client_info(
                                self.chain_id.clone(),
                                IbcV1::ID,
                                RawClientId::new(client_id.clone()),
                            )
                            .await
                            .map_err(json_rpc_error_to_error_object)?;

                        let client_meta = voyager_client
                            .client_meta(
                                self.chain_id.clone(),
                                IbcV1::ID,
                                height.into(),
                                RawClientId::new(client_id.clone()),
                            )
                            .await
                            .map_err(json_rpc_error_to_error_object)?;

                        Ok(data(ChainEvent {
                            chain_id: self.chain_id.clone(),
                            client_info,
                            counterparty_chain_id: client_meta.chain_id,
                            tx_hash,
                            provable_height,
                            ibc_version_id: IbcV1::ID,
                            event: into_value::<ibc_v1::FullIbcEvent>(match event {
                                RawEvent::IbcV1(IbcEvent::CreateClient(event)) => {
                                    ibc_v1::CreateClient {
                                        client_id: event.client_id,
                                        client_type: ClientType::new(event.client_type),
                                        consensus_height: event.consensus_height,
                                    }
                                    .into()
                                }
                                RawEvent::IbcV1(IbcEvent::UpdateClient(event)) => {
                                    ibc_v1::UpdateClient {
                                        client_id: event.client_id,
                                        client_type: ClientType::new(event.client_type),
                                        consensus_heights: event.consensus_heights,
                                    }
                                    .into()
                                }
                                RawEvent::IbcV1(IbcEvent::ConnectionOpenInit(event)) => {
                                    ibc_v1::ConnectionOpenInit {
                                        client_id: event.client_id,
                                        connection_id: event.connection_id,
                                        counterparty_client_id: event.counterparty_client_id,
                                    }
                                }
                                .into(),
                                RawEvent::IbcV1(IbcEvent::ConnectionOpenTry(event)) => {
                                    ibc_v1::ConnectionOpenTry {
                                        client_id: event.client_id,
                                        connection_id: event.connection_id,
                                        counterparty_client_id: event.counterparty_client_id,
                                        counterparty_connection_id: event
                                            .counterparty_connection_id,
                                    }
                                }
                                .into(),
                                RawEvent::IbcV1(IbcEvent::ConnectionOpenAck(event)) => {
                                    ibc_v1::ConnectionOpenAck {
                                        client_id: event.client_id,
                                        connection_id: event.connection_id,
                                        counterparty_client_id: event.counterparty_client_id,
                                        counterparty_connection_id: event
                                            .counterparty_connection_id,
                                    }
                                }
                                .into(),
                                RawEvent::IbcV1(IbcEvent::ConnectionOpenConfirm(event)) => {
                                    ibc_v1::ConnectionOpenConfirm {
                                        client_id: event.client_id,
                                        connection_id: event.connection_id,
                                        counterparty_client_id: event.counterparty_client_id,
                                        counterparty_connection_id: event
                                            .counterparty_connection_id,
                                    }
                                }
                                .into(),
                                _ => unreachable!("who needs flow typing"),
                            }),
                        }))
                    }
                    RawEvent::IbcV1(
                        IbcEvent::ChannelOpenInit(ChannelOpenInit {
                            ref connection_id, ..
                        })
                        | IbcEvent::ChannelOpenTry(ChannelOpenTry {
                            ref connection_id, ..
                        }),
                    ) => {
                        let connection = voyager_client
                            .query_spec_ibc_state(
                                self.chain_id.clone(),
                                height.into(),
                                ConnectionPath {
                                    connection_id: connection_id.clone(),
                                },
                            )
                            .await?
                            .state
                            .ok_or_else(missing_state("connection must exist", None))?;

                        let client_info = voyager_client
                            .client_info(
                                self.chain_id.clone(),
                                IbcV1::ID,
                                RawClientId::new(connection.client_id.clone()),
                            )
                            .await
                            .map_err(json_rpc_error_to_error_object)?;

                        let client_meta = voyager_client
                            .client_meta(
                                self.chain_id.clone(),
                                IbcV1::ID,
                                height.into(),
                                RawClientId::new(connection.client_id.clone()),
                            )
                            .await
                            .map_err(json_rpc_error_to_error_object)?;

                        Ok(data(ChainEvent {
                            chain_id: self.chain_id.clone(),
                            client_info,
                            counterparty_chain_id: client_meta.chain_id,
                            tx_hash,
                            provable_height,
                            ibc_version_id: IbcV1::ID,
                            event: into_value::<ibc_v1::FullIbcEvent>(match event {
                                RawEvent::IbcV1(IbcEvent::ChannelOpenInit(event)) => {
                                    ibc_v1::ChannelOpenInit {
                                        port_id: event.port_id,
                                        channel_id: event.channel_id,
                                        counterparty_port_id: event.counterparty_port_id,
                                        connection,
                                        version: event.version,
                                    }
                                }
                                .into(),
                                RawEvent::IbcV1(IbcEvent::ChannelOpenTry(event)) => {
                                    ibc_v1::ChannelOpenTry {
                                        port_id: event.port_id,
                                        channel_id: event.channel_id,
                                        counterparty_port_id: event.counterparty_port_id,
                                        counterparty_channel_id: event.counterparty_channel_id,
                                        connection,
                                        version: event.version,
                                    }
                                    .into()
                                }
                                _ => unreachable!("who needs flow typing"),
                            }),
                        }))
                    }
                    RawEvent::IbcV1(
                        IbcEvent::ChannelOpenAck(ChannelOpenAck {
                            ref connection_id,
                            ref port_id,
                            ref channel_id,
                            ..
                        })
                        | IbcEvent::ChannelOpenConfirm(ChannelOpenConfirm {
                            ref connection_id,
                            ref port_id,
                            ref channel_id,
                            ..
                        }),
                    ) => {
                        let connection = voyager_client
                            .query_spec_ibc_state(
                                self.chain_id.clone(),
                                height.into(),
                                ConnectionPath {
                                    connection_id: connection_id.clone(),
                                },
                            )
                            .await?
                            .state
                            .ok_or_else(missing_state("connection must exist", None))?;

                        let client_info = voyager_client
                            .client_info(
                                self.chain_id.clone(),
                                IbcV1::ID,
                                RawClientId::new(connection.client_id.clone()),
                            )
                            .await
                            .map_err(json_rpc_error_to_error_object)?;

                        let client_meta = voyager_client
                            .client_meta(
                                self.chain_id.clone(),
                                IbcV1::ID,
                                height.into(),
                                RawClientId::new(connection.client_id.clone()),
                            )
                            .await
                            .map_err(json_rpc_error_to_error_object)?;

                        let channel = voyager_client
                            .query_spec_ibc_state(
                                self.chain_id.clone(),
                                height.into(),
                                ChannelEndPath {
                                    port_id: port_id.to_owned(),
                                    channel_id: channel_id.to_owned(),
                                },
                            )
                            .await?
                            .state
                            .ok_or_else(missing_state("channel must exist", None))?;

                        Ok(data(ChainEvent {
                            chain_id: self.chain_id.clone(),
                            client_info,
                            counterparty_chain_id: client_meta.chain_id,
                            tx_hash,
                            provable_height,
                            ibc_version_id: IbcV1::ID,
                            event: into_value::<ibc_v1::FullIbcEvent>(match event {
                                RawEvent::IbcV1(IbcEvent::ChannelOpenAck(event)) => {
                                    ibc_v1::ChannelOpenAck {
                                        port_id: event.port_id,
                                        channel_id: event.channel_id,
                                        counterparty_port_id: event.counterparty_port_id,
                                        counterparty_channel_id: event.counterparty_channel_id,
                                        connection,
                                        version: channel.version,
                                    }
                                }
                                .into(),
                                RawEvent::IbcV1(IbcEvent::ChannelOpenConfirm(event)) => {
                                    ibc_v1::ChannelOpenConfirm {
                                        port_id: event.port_id,
                                        channel_id: event.channel_id,
                                        counterparty_port_id: event.counterparty_port_id,
                                        counterparty_channel_id: event.counterparty_channel_id,
                                        connection,
                                        version: channel.version,
                                    }
                                    .into()
                                }
                                _ => unreachable!("who needs flow typing"),
                            }),
                        }))
                    }
                    // packet origin is this chain
                    RawEvent::IbcV1(IbcEvent::SendPacket(event)) => {
                        let (
                            counterparty_chain_id,
                            client_info,
                            source_channel,
                            destination_channel,
                            channel_ordering,
                        ) = self
                            .make_packet_metadata(
                                height,
                                event.connection_id.to_owned(),
                                event.packet_src_port.to_owned(),
                                event.packet_src_channel.to_owned(),
                                event.packet_dst_port.to_owned(),
                                event.packet_dst_channel.to_owned(),
                                voyager_client,
                            )
                            .await?;

                        Ok(data(ChainEvent {
                            chain_id: self.chain_id.clone(),
                            client_info,
                            counterparty_chain_id,
                            tx_hash,
                            provable_height,
                            ibc_version_id: IbcV1::ID,
                            event: into_value::<ibc_v1::FullIbcEvent>(
                                ibc_v1::SendPacket {
                                    packet_data: event.packet_data_hex,
                                    packet: ibc_v1::PacketMetadata {
                                        sequence: event.packet_sequence,
                                        source_channel,
                                        destination_channel,
                                        channel_ordering,
                                        timeout_height: event.packet_timeout_height,
                                        timeout_timestamp: event.packet_timeout_timestamp,
                                    },
                                }
                                .into(),
                            ),
                        }))
                    }
                    RawEvent::IbcV1(IbcEvent::TimeoutPacket(event)) => {
                        let (
                            counterparty_chain_id,
                            client_info,
                            source_channel,
                            destination_channel,
                            channel_ordering,
                        ) = self
                            .make_packet_metadata(
                                height,
                                event.connection_id.to_owned(),
                                event.packet_src_port.to_owned(),
                                event.packet_src_channel.to_owned(),
                                event.packet_dst_port.to_owned(),
                                event.packet_dst_channel.to_owned(),
                                voyager_client,
                            )
                            .await?;

                        Ok(data(ChainEvent {
                            chain_id: self.chain_id.clone(),
                            client_info,
                            counterparty_chain_id,
                            tx_hash,
                            provable_height,
                            ibc_version_id: IbcV1::ID,
                            event: into_value::<ibc_v1::FullIbcEvent>(
                                ibc_v1::TimeoutPacket {
                                    packet: ibc_v1::PacketMetadata {
                                        sequence: event.packet_sequence,
                                        source_channel,
                                        destination_channel,
                                        channel_ordering,
                                        timeout_height: event.packet_timeout_height,
                                        timeout_timestamp: event.packet_timeout_timestamp,
                                    },
                                }
                                .into(),
                            ),
                        }))
                    }
                    RawEvent::IbcV1(IbcEvent::AcknowledgePacket(event)) => {
                        let (
                            counterparty_chain_id,
                            client_info,
                            source_channel,
                            destination_channel,
                            channel_ordering,
                        ) = self
                            .make_packet_metadata(
                                height,
                                event.connection_id.to_owned(),
                                event.packet_src_port.to_owned(),
                                event.packet_src_channel.to_owned(),
                                event.packet_dst_port.to_owned(),
                                event.packet_dst_channel.to_owned(),
                                voyager_client,
                            )
                            .await?;

                        Ok(data(ChainEvent {
                            chain_id: self.chain_id.clone(),
                            client_info,
                            counterparty_chain_id,
                            tx_hash,
                            provable_height,
                            ibc_version_id: IbcV1::ID,
                            event: into_value::<ibc_v1::FullIbcEvent>(
                                ibc_v1::AcknowledgePacket {
                                    packet: ibc_v1::PacketMetadata {
                                        sequence: event.packet_sequence,
                                        source_channel,
                                        destination_channel,
                                        channel_ordering,
                                        timeout_height: event.packet_timeout_height,
                                        timeout_timestamp: event.packet_timeout_timestamp,
                                    },
                                }
                                .into(),
                            ),
                        }))
                    }
                    // packet origin is the counterparty chain (if i put this comment above this pattern rustfmt explodes)
                    RawEvent::IbcV1(IbcEvent::WriteAcknowledgement(event)) => {
                        let (
                            counterparty_chain_id,
                            client_info,
                            destination_channel,
                            source_channel,
                            channel_ordering,
                        ) = self
                            .make_packet_metadata(
                                height,
                                event.connection_id.to_owned(),
                                event.packet_dst_port.to_owned(),
                                event.packet_dst_channel.to_owned(),
                                event.packet_src_port.to_owned(),
                                event.packet_src_channel.to_owned(),
                                voyager_client,
                            )
                            .await?;

                        Ok(data(ChainEvent {
                            chain_id: self.chain_id.clone(),
                            client_info,
                            counterparty_chain_id,
                            tx_hash,
                            provable_height,
                            ibc_version_id: IbcV1::ID,
                            event: into_value::<ibc_v1::FullIbcEvent>(
                                ibc_v1::WriteAcknowledgement {
                                    packet_data: event.packet_data_hex,
                                    packet_ack: event.packet_ack_hex,
                                    packet: ibc_v1::PacketMetadata {
                                        sequence: event.packet_sequence,
                                        source_channel,
                                        destination_channel,
                                        channel_ordering,
                                        timeout_height: event.packet_timeout_height,
                                        timeout_timestamp: event.packet_timeout_timestamp,
                                    },
                                }
                                .into(),
                            ),
                        }))
                    }
                    RawEvent::IbcV1(IbcEvent::RecvPacket(event)) => {
                        let (
                            counterparty_chain_id,
                            client_info,
                            destination_channel,
                            source_channel,
                            channel_ordering,
                        ) = self
                            .make_packet_metadata(
                                height,
                                event.connection_id.to_owned(),
                                event.packet_dst_port.to_owned(),
                                event.packet_dst_channel.to_owned(),
                                event.packet_src_port.to_owned(),
                                event.packet_src_channel.to_owned(),
                                voyager_client,
                            )
                            .await?;

                        Ok(data(ChainEvent {
                            chain_id: self.chain_id.clone(),
                            client_info,
                            counterparty_chain_id,
                            tx_hash,
                            provable_height,
                            ibc_version_id: IbcV1::ID,
                            event: into_value::<ibc_v1::FullIbcEvent>(
                                ibc_v1::RecvPacket {
                                    packet_data: event.packet_data_hex,
                                    packet: ibc_v1::PacketMetadata {
                                        sequence: event.packet_sequence,
                                        source_channel,
                                        destination_channel,
                                        channel_ordering,
                                        timeout_height: event.packet_timeout_height,
                                        timeout_timestamp: event.packet_timeout_timestamp,
                                    },
                                }
                                .into(),
                            ),
                        }))
                    }
                    RawEvent::IbcUnion(ibc_events::union_ibc::IbcEvent::CreateClient(
                        create_client,
                    )) => {
                        dbg!(&create_client);

                        let client_info = voyager_client
                            .client_info(
                                self.chain_id.clone(),
                                IbcUnion::ID,
                                RawClientId::new(create_client.client_id),
                            )
                            .await
                            .map_err(json_rpc_error_to_error_object)?;

                        let client_meta = voyager_client
                            .client_meta(
                                self.chain_id.clone(),
                                IbcUnion::ID,
                                height.into(),
                                RawClientId::new(create_client.client_id),
                            )
                            .await
                            .map_err(json_rpc_error_to_error_object)?;

                        Ok(data(ChainEvent {
                            chain_id: self.chain_id.clone(),
                            client_info,
                            counterparty_chain_id: client_meta.chain_id,
                            tx_hash,
                            provable_height,
                            ibc_version_id: IbcV1::ID,
                            event: into_value::<ibc_union::FullIbcEvent>(
                                ibc_union::ClientCreated {
                                    client_id: create_client.client_id,
                                    client_type: ClientType::new(create_client.client_type),
                                }
                                .into(),
                            ),
                        }))
                    }
                    RawEvent::IbcUnion(ibc_events::union_ibc::IbcEvent::ConnectionOpenInit(
                        connection_open_init,
                    )) => {
                        dbg!(&connection_open_init);

                        let client_info = voyager_client
                            .client_info(
                                self.chain_id.clone(),
                                IbcUnion::ID,
                                RawClientId::new(connection_open_init.client_id),
                            )
                            .await
                            .map_err(json_rpc_error_to_error_object)?;

                        let client_meta = voyager_client
                            .client_meta(
                                self.chain_id.clone(),
                                IbcUnion::ID,
                                height.into(),
                                RawClientId::new(connection_open_init.client_id),
                            )
                            .await
                            .map_err(json_rpc_error_to_error_object)?;

                        Ok(data(ChainEvent {
                            chain_id: self.chain_id.clone(),
                            client_info,
                            counterparty_chain_id: client_meta.chain_id,
                            tx_hash,
                            provable_height,
                            ibc_version_id: IbcUnion::ID,
                            event: into_value::<ibc_union::FullIbcEvent>(
                                ibc_union::ConnectionOpenInit {
                                    client_id: connection_open_init.client_id,
                                    connection_id: connection_open_init.connection_id,
                                    counterparty_client_id: connection_open_init
                                        .counterparty_client_id,
                                }
                                .into(),
                            ),
                        }))
                    }
                }
            }
        }
    }
}

// NOTE: For both of the below functions, `message` as a field will override any actual message put in (i.e. `error!("foo", message = "bar")` will print as "bar", not "foo" with an extra field `message = "bar"`.

fn rpc_error<E: Error>(
    message: impl Display,
    data: Option<Value>,
) -> impl FnOnce(E) -> ErrorObjectOwned {
    move |e| {
        let message = format!("{message}: {}", ErrorReporter(e));
        error!(%message, data = %data.as_ref().unwrap_or(&serde_json::Value::Null));
        ErrorObject::owned(-1, message, data)
    }
}
