use std::num::NonZeroU64;

use enumorph::Enumorph;
use macros::apply;
use queue_msg::{data, queue_msg, HandleData, Op, QueueError, SubsetOf};
use serde_json::Value;
use unionlabs::{
    hash::H256,
    ibc::core::{
        channel::{
            msg_acknowledgement::MsgAcknowledgement, msg_channel_open_ack::MsgChannelOpenAck,
            msg_channel_open_confirm::MsgChannelOpenConfirm,
            msg_channel_open_try::MsgChannelOpenTry, msg_recv_packet::MsgRecvPacket,
            msg_timeout::MsgTimeout, order::Order,
        },
        client::{
            height::Height, msg_create_client::MsgCreateClient, msg_update_client::MsgUpdateClient,
        },
        connection::{
            connection_end::ConnectionEnd, msg_connection_open_ack::MsgConnectionOpenAck,
            msg_connection_open_confirm::MsgConnectionOpenConfirm,
            msg_connection_open_try::MsgConnectionOpenTry,
        },
    },
    ics24::{
        AcknowledgementPath, ChannelEndPath, ClientConsensusStatePath, ClientStatePath,
        CommitmentPath, ConnectionPath, IbcPath, NextClientSequencePath,
        NextConnectionSequencePath, NextSequenceAckPath, NextSequenceRecvPath,
        NextSequenceSendPath, Path, ReceiptPath,
    },
    id::{ChannelId, ClientId, ConnectionId, PortId},
    traits::Member,
};

use crate::{
    plugin::{ClientStateMeta, ConsensusStateMeta},
    top_level_identifiable_enum, ClientType, Context, IbcInterface, PluginMessage, VoyagerMessage,
};

#[apply(top_level_identifiable_enum)]
#[queue_msg]
#[derive(Enumorph, SubsetOf)]
pub enum Data<D = serde_json::Value> {
    // originally block
    IbcEvent(ChainEvent),
    IbcMessage(IbcMessage),
    LatestHeight(LatestHeight),

    ClientInfo(ClientInfo),

    // originally relay
    SelfClientState(SelfClientState),
    SelfConsensusState(SelfConsensusState),

    UnfinalizedClientState(UnfinalizedTrustedClientState),

    // state
    ClientState(IbcState<ClientStatePath>),
    ClientConsensusState(IbcState<ClientConsensusStatePath>),
    Connection(IbcState<ConnectionPath>),
    ChannelEnd(IbcState<ChannelEndPath>),
    Commitment(IbcState<CommitmentPath>),
    Acknowledgement(IbcState<AcknowledgementPath>),
    Receipt(IbcState<ReceiptPath>),
    NextSequenceSend(IbcState<NextSequenceSendPath>),
    NextSequenceRecv(IbcState<NextSequenceRecvPath>),
    NextSequenceAck(IbcState<NextSequenceAckPath>),
    NextConnectionSequence(IbcState<NextConnectionSequencePath>),
    NextClientSequence(IbcState<NextClientSequencePath>),

    // proof
    ClientStateProof(IbcProof<ClientStatePath>),
    ClientConsensusStateProof(IbcProof<ClientConsensusStatePath>),
    ConnectionProof(IbcProof<ConnectionPath>),
    ChannelEndProof(IbcProof<ChannelEndPath>),
    CommitmentProof(IbcProof<CommitmentPath>),
    AcknowledgementProof(IbcProof<AcknowledgementPath>),
    ReceiptProof(IbcProof<ReceiptPath>),
    NextSequenceSendProof(IbcProof<NextSequenceSendPath>),
    NextSequenceRecvProof(IbcProof<NextSequenceRecvPath>),
    NextSequenceAckProof(IbcProof<NextSequenceAckPath>),
    NextConnectionSequenceProof(IbcProof<NextConnectionSequencePath>),
    NextClientSequenceProof(IbcProof<NextClientSequencePath>),

    RawIbcProof(RawIbcProof),

    DecodedClientStateMeta(DecodedClientStateMeta),
    DecodedClientConsensusStateMeta(DecodedConsensusStateMeta),

    OrderedHeaders(OrderedHeaders),
    OrderedMsgUpdateClients(OrderedMsgUpdateClients),

    EncodedClientState(EncodedClientState),
    EncodedConsensusState(EncodedConsensusState),
    EncodedHeader(EncodedHeader),

    #[subset_of(ignore)]
    Plugin(PluginMessage<D>),
}

// Passthrough since we don't want to handle any top-level data, just bubble it
// up to the top level.
impl<D: Member, F: Member, A: Member> HandleData<VoyagerMessage<D, F, A>> for Data<D> {
    // #[instrument(skip_all, fields(chain_id = %self.chain_id))]
    fn handle(self, _store: &Context) -> Result<Op<VoyagerMessage<D, F, A>>, QueueError> {
        Ok(data(self))
    }
}

#[queue_msg]
pub struct ChainEvent {
    /// The chain where this event was emitted.
    pub chain_id: String,
    /// The underlying client of this event, on [`Self::chain_id`].
    pub client_info: ClientInfo,
    /// The chain on the other end of this IBC event.
    pub counterparty_chain_id: String,
    pub tx_hash: H256,
    /// The 'provable height' of the event. This is the minimum height at which the effect of the IBC action that caused this event is provable in the state root of the chain identified by [`Self::chain_id`].
    pub provable_height: Height,
    pub event: FullIbcEvent,
}

impl ChainEvent {
    pub fn client_id(&self) -> &ClientId {
        match self.event {
            FullIbcEvent::CreateClient(ref event) => &event.client_id,
            FullIbcEvent::UpdateClient(ref event) => &event.client_id,
            FullIbcEvent::ConnectionOpenInit(ref event) => &event.client_id,
            FullIbcEvent::ConnectionOpenTry(ref event) => &event.client_id,
            FullIbcEvent::ConnectionOpenAck(ref event) => &event.client_id,
            FullIbcEvent::ConnectionOpenConfirm(ref event) => &event.client_id,
            FullIbcEvent::ChannelOpenInit(ref event) => &event.connection.client_id,
            FullIbcEvent::ChannelOpenTry(ref event) => &event.connection.client_id,
            FullIbcEvent::ChannelOpenAck(ref event) => &event.connection.client_id,
            FullIbcEvent::ChannelOpenConfirm(ref event) => &event.connection.client_id,
            FullIbcEvent::SendPacket(ref event) => {
                &event.packet.source_channel.connection.client_id
            }
            FullIbcEvent::RecvPacket(ref event) => {
                &event.packet.source_channel.connection.client_id
            }
            FullIbcEvent::WriteAcknowledgement(ref event) => {
                &event.packet.source_channel.connection.client_id
            }
            FullIbcEvent::AcknowledgePacket(ref event) => {
                &event.packet.source_channel.connection.client_id
            }
            FullIbcEvent::TimeoutPacket(ref event) => {
                &event.packet.source_channel.connection.client_id
            }
        }
    }

    /// Returns the counterparty client id of this ibc event, if there is a
    /// counterparty. This will return `None` for `UpdateClient` and
    /// `CreateClient`.
    pub fn counterparty_client_id(&self) -> Option<&ClientId> {
        match self.event {
            FullIbcEvent::ConnectionOpenInit(ref event) => Some(&event.counterparty_client_id),
            FullIbcEvent::ConnectionOpenTry(ref event) => Some(&event.counterparty_client_id),
            FullIbcEvent::ConnectionOpenAck(ref event) => Some(&event.counterparty_client_id),
            FullIbcEvent::ConnectionOpenConfirm(ref event) => Some(&event.counterparty_client_id),
            FullIbcEvent::ChannelOpenInit(ref event) => {
                Some(&event.connection.counterparty.client_id)
            }
            FullIbcEvent::ChannelOpenTry(ref event) => {
                Some(&event.connection.counterparty.client_id)
            }
            FullIbcEvent::ChannelOpenAck(ref event) => {
                Some(&event.connection.counterparty.client_id)
            }
            FullIbcEvent::ChannelOpenConfirm(ref event) => {
                Some(&event.connection.counterparty.client_id)
            }
            FullIbcEvent::SendPacket(ref event) => {
                Some(&event.packet.destination_channel.connection.client_id)
            }
            FullIbcEvent::RecvPacket(ref event) => {
                Some(&event.packet.source_channel.connection.client_id)
            }
            FullIbcEvent::WriteAcknowledgement(ref event) => {
                Some(&event.packet.source_channel.connection.client_id)
            }
            FullIbcEvent::AcknowledgePacket(ref event) => {
                Some(&event.packet.destination_channel.connection.client_id)
            }
            FullIbcEvent::TimeoutPacket(ref event) => {
                Some(&event.packet.destination_channel.connection.client_id)
            }
            _ => None,
        }
    }
}

#[queue_msg]
#[derive(Enumorph, Eq)]
pub enum IbcMessage {
    CreateClient(MsgCreateClient),

    // UpdateClient(MsgUpdateClient),
    ConnectionOpenTry(MsgConnectionOpenTry),
    ConnectionOpenAck(MsgConnectionOpenAck),
    ConnectionOpenConfirm(MsgConnectionOpenConfirm),

    ChannelOpenTry(MsgChannelOpenTry),
    ChannelOpenAck(MsgChannelOpenAck),
    ChannelOpenConfirm(MsgChannelOpenConfirm),

    RecvPacket(MsgRecvPacket),
    AcknowledgePacket(MsgAcknowledgement),
    TimeoutPacket(MsgTimeout),
}

#[queue_msg]
pub struct CreateClient {
    pub client_id: ClientId,
    // TODO: Figure out if there's a better type we can use than string
    pub client_type: String,
    pub consensus_height: Height,
}

#[queue_msg]
pub struct UpdateClient {
    pub client_id: ClientId,
    pub client_type: String,
    pub consensus_heights: Vec<Height>,
}

#[queue_msg]
pub struct ConnectionOpenInit {
    pub connection_id: ConnectionId,
    pub client_id: ClientId,
    pub counterparty_client_id: ClientId,
}

#[queue_msg]
pub struct ConnectionOpenTry {
    pub connection_id: ConnectionId,
    pub client_id: ClientId,
    pub counterparty_client_id: ClientId,
    pub counterparty_connection_id: ConnectionId,
}

#[queue_msg]
pub struct ConnectionOpenAck {
    pub connection_id: ConnectionId,
    pub client_id: ClientId,
    pub counterparty_client_id: ClientId,
    pub counterparty_connection_id: ConnectionId,
}

#[queue_msg]
pub struct ConnectionOpenConfirm {
    pub connection_id: ConnectionId,
    pub client_id: ClientId,
    pub counterparty_client_id: ClientId,
    pub counterparty_connection_id: ConnectionId,
}

#[queue_msg]
pub struct ChannelOpenInit {
    pub port_id: PortId,
    pub channel_id: ChannelId,

    pub counterparty_port_id: PortId,

    pub connection: ConnectionEnd,

    pub version: String,
}

#[queue_msg]
pub struct ChannelOpenTry {
    pub port_id: PortId,
    pub channel_id: ChannelId,

    pub counterparty_port_id: PortId,
    pub counterparty_channel_id: ChannelId,

    pub connection: ConnectionEnd,

    pub version: String,
}

#[queue_msg]
pub struct ChannelOpenAck {
    pub port_id: PortId,
    pub channel_id: ChannelId,

    pub counterparty_port_id: PortId,
    pub counterparty_channel_id: ChannelId,

    pub connection: ConnectionEnd,

    pub version: String,
}

#[queue_msg]
pub struct ChannelOpenConfirm {
    pub port_id: PortId,
    pub channel_id: ChannelId,

    pub counterparty_port_id: PortId,
    pub counterparty_channel_id: ChannelId,

    pub connection: ConnectionEnd,

    pub version: String,
}

#[queue_msg]
pub struct WriteAcknowledgement {
    #[serde(with = "::serde_utils::hex_string")]
    #[debug(wrap = ::serde_utils::fmt::DebugAsHex)]
    pub packet_data: Vec<u8>,

    #[serde(with = "::serde_utils::hex_string")]
    #[debug(wrap = ::serde_utils::fmt::DebugAsHex)]
    pub packet_ack: Vec<u8>,

    pub packet: PacketMetadata,
}

#[queue_msg]
pub struct RecvPacket {
    #[serde(with = "::serde_utils::hex_string")]
    #[debug(wrap = ::serde_utils::fmt::DebugAsHex)]
    pub packet_data: Vec<u8>,

    pub packet: PacketMetadata,
}

#[queue_msg]
pub struct SendPacket {
    #[serde(with = "::serde_utils::hex_string")]
    #[debug(wrap = ::serde_utils::fmt::DebugAsHex)]
    pub packet_data: Vec<u8>,

    pub packet: PacketMetadata,
}

#[queue_msg]
pub struct AcknowledgePacket {
    pub packet: PacketMetadata,
}

#[queue_msg]
pub struct TimeoutPacket {
    pub packet: PacketMetadata,
}

#[queue_msg]
pub struct PacketMetadata {
    pub sequence: NonZeroU64,

    pub source_channel: ChannelMetadata,
    pub destination_channel: ChannelMetadata,

    pub timeout_height: Height,
    pub timeout_timestamp: u64,
}

#[queue_msg]
pub struct ChannelMetadata {
    pub port_id: PortId,
    pub channel_id: ChannelId,
    pub ordering: Order,
    pub version: String,
    pub connection: ConnectionMetadata,
}

#[queue_msg]
pub struct ConnectionMetadata {
    pub client_id: ClientId,
    // this is really `Either<ConnectionId, EmptyString>`
    // REVIEW: Is it?
    pub connection_id: ConnectionId,
}

/// Similar to [`IbcEvent`], but contains more information (counterparty
/// clients, channel version, etc)
#[queue_msg]
#[derive(Enumorph, SubsetOf)]
pub enum FullIbcEvent {
    CreateClient(CreateClient),

    UpdateClient(UpdateClient),

    ConnectionOpenInit(ConnectionOpenInit),
    ConnectionOpenTry(ConnectionOpenTry),
    ConnectionOpenAck(ConnectionOpenAck),
    ConnectionOpenConfirm(ConnectionOpenConfirm),

    ChannelOpenInit(ChannelOpenInit),
    ChannelOpenTry(ChannelOpenTry),
    ChannelOpenAck(ChannelOpenAck),
    ChannelOpenConfirm(ChannelOpenConfirm),

    SendPacket(SendPacket),
    RecvPacket(RecvPacket),
    WriteAcknowledgement(WriteAcknowledgement),
    AcknowledgePacket(AcknowledgePacket),
    TimeoutPacket(TimeoutPacket),
}

#[queue_msg]
pub struct LatestHeight {
    pub chain_id: String,
    pub height: Height,
}

/// The type of a light client on a chain, along with the IBC interface it's on
/// (and any associated metadata).
///
/// # Examples
///
/// - 08-wasm client on union, tracking ethereum mainnet: `(ibc-go-v8/08-wasm,
///   ethereum_mainnet, {"checksum": "0x..."})`
/// - 07-tendermint client on stargaze, tracking osmosis: `(ibc-go-v8/native,
///   tendermint)`
/// - 08-wasm client on babylon, tracking union: `(ibc-go-v8/08-wasm, cometbls, {"checksum": "0x..."}))`
/// - cometbls client on scroll, tracking union: `(ibc-solidity, cometbls)`
#[queue_msg]
pub struct ClientInfo {
    pub client_type: ClientType<'static>,
    pub ibc_interface: IbcInterface<'static>,
    /// Additional metadata about this client.
    ///
    /// This is currently only used for threading the checksum for ibc-go
    /// 08-wasm clients, and can likely be removed when support for that IBC
    /// interface is dropped.
    #[serde(default)]
    pub metadata: Value,
}

#[queue_msg]
pub struct SelfClientState {
    #[serde(with = "::serde_utils::hex_string")]
    #[debug(wrap = ::serde_utils::fmt::DebugAsHex)]
    pub self_client_state: Vec<u8>,
}

#[queue_msg]
pub struct SelfConsensusState {
    #[serde(with = "::serde_utils::hex_string")]
    #[debug(wrap = ::serde_utils::fmt::DebugAsHex)]
    pub self_consensus_state: Vec<u8>,
}

#[queue_msg]
pub struct UnfinalizedTrustedClientState {
    pub height: Height,
    // pub client_state: Hc::StoredClientState<Tr>,
    pub client_state: ClientStateMeta,
}

#[queue_msg]
#[serde(bound(serialize = "", deserialize = ""))]
pub struct IbcState<P: IbcPath> {
    pub chain_id: String,
    pub path: P,
    /// The height that the state was read at.
    pub height: Height,
    pub state: P::Value,
}

#[queue_msg]
#[serde(bound(serialize = "", deserialize = ""))]
pub struct IbcProof<P: IbcPath> {
    pub path: P,
    pub height: Height,
    #[serde(with = "::serde_utils::hex_string")]
    #[debug(wrap = ::serde_utils::fmt::DebugAsHex)]
    pub proof: Vec<u8>,
}

#[queue_msg]
pub struct RawIbcProof {
    pub path: Path,
    pub height: Height,
    /// The raw proof, encoded as JSON, which will be encoded by the relevant
    /// client module.
    pub proof: Value,
}

#[queue_msg]
pub struct DecodedClientStateMeta {
    pub path: ClientStatePath,
    /// The height that the state was read at. Same as [`IbcState::height`].
    pub height: Height,
    pub state: ClientStateMeta,
}

#[queue_msg]
pub struct DecodedConsensusStateMeta {
    pub path: ClientConsensusStatePath,
    pub height: Height,
    pub state: ConsensusStateMeta,
}

#[queue_msg]
pub struct DecodedHeaderMeta {
    /// The new trusted height that the header provides a consensus update to.
    pub height: Height,
}

#[queue_msg]
pub struct OrderedHeaders {
    pub headers: Vec<(DecodedHeaderMeta, Value)>,
}

#[queue_msg]
pub struct OrderedMsgUpdateClients {
    pub updates: Vec<(DecodedHeaderMeta, MsgUpdateClient)>,
}

#[queue_msg]
pub struct EncodedClientState {
    #[serde(with = "::serde_utils::hex_string")]
    #[debug(wrap = ::serde_utils::fmt::DebugAsHex)]
    pub encoded_client_state: Vec<u8>,
}

#[queue_msg]
pub struct EncodedConsensusState {
    #[serde(with = "::serde_utils::hex_string")]
    #[debug(wrap = ::serde_utils::fmt::DebugAsHex)]
    pub encoded_consensus_state: Vec<u8>,
}

#[queue_msg]
pub struct EncodedHeader {
    #[serde(with = "::serde_utils::hex_string")]
    #[debug(wrap = ::serde_utils::fmt::DebugAsHex)]
    pub encoded_header: Vec<u8>,
}