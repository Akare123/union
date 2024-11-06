use core::str::FromStr;

use frame_support_procedural::PartialEqNoBound;
use ibc_events::IbcEvent;
use serde::{Deserialize, Serialize};
use states::{
    channel_handshake::{ChannelOpenAck, ChannelOpenConfirm, ChannelOpenInit, ChannelOpenTry},
    client_state::UpdateClient,
    connection_handshake::{
        ConnectionOpenAck, ConnectionOpenConfirm, ConnectionOpenInit, ConnectionOpenTry,
    },
    packet::{Acknowledgement, RecvPacket, SendPacket},
    CreateClient,
};
use types::{
    channel::{ChannelOrder, ChannelState},
    connection::ConnectionState,
};
use unionlabs::{
    encoding::{Decode, DecodeErrorOf, Encode, Encoding, Proto},
    ibc::core::{
        channel::{self, order::Order, packet::Packet},
        client::height::Height,
        commitment::{merkle_path::MerklePath, merkle_prefix::MerklePrefix},
        connection::{self, version::Version},
    },
    ics24::Path,
    id::{ChannelId, ClientId, ConnectionId, PortId},
};

pub mod states;
pub mod types;

lazy_static::lazy_static! {
    pub static ref DEFAULT_IBC_VERSION: Vec<Version> = vec![Version { identifier: String::from("1"), features: vec![Order::Unordered] }];

    // TODO(aeryz): idk if this is enforced by ibc-go or by the spec. Because we don't have merkle prefix in ethereum or near.
    pub static ref DEFAULT_MERKLE_PREFIX: MerklePrefix = MerklePrefix { key_prefix: b"ibc".into() };
}

#[derive(thiserror::Error, PartialEqNoBound, Debug)]
pub enum IbcError {
    #[error("client {0} is not active ({1})")]
    NotActive(u32, Status),

    // TODO(aeryz): this needs context
    #[error("unexpected action is provided to the state machine")]
    UnexpectedAction,

    // TODO(aeryz): this needs context
    #[error("client message verification failed")]
    ClientMessageVerificationFailed,

    #[error("connection ({0}) not found")]
    ConnectionNotFound(ConnectionId),

    // TODO(aeryz): make this a struct
    #[error("connection state is {0:?} while {1:?} is expected")]
    IncorrectConnectionState(ConnectionState, ConnectionState),

    // TODO(aeryz): this should have the error
    #[error("ibc app callback failed ({0})")]
    IbcAppCallbackFailed(String),

    #[error("acknowledgement with the sequence {0} already exists")]
    AcknowledgementExists(u64),

    #[error("empty acknowledgement")]
    EmptyAcknowledgement,

    // TODO(aeryz): this should have the error
    #[error("membership verification failed")]
    MembershipVerificationFailure,

    #[error("no supported version is found")]
    NoSupportedVersionFound,

    #[error("empty version features")]
    EmptyVersionFeatures,

    #[error("version identifier ({0}) does not match the proposed version ({1})")]
    VersionIdentifiedMismatch(String, String),

    #[error("the proposed version contains an unsupported feature ({0})")]
    UnsupportedFeatureInVersion(Order),

    #[error("the client state is not found for client {0}")]
    ClientStateNotFound(u32),

    #[error("channel ({0}) is not found")]
    ChannelNotFound(ChannelId),

    #[error("channel state is {0:?} while {1:?} is expected")]
    IncorrectChannelState(ChannelState, ChannelState),

    #[error("source port ({0}) does not match the received packet's counterparty port ({1})")]
    SourcePortMismatch(PortId, PortId),

    #[error("destination port ({0}) does not match the received packet's counterparty port ({1})")]
    DestinationPortMismatch(PortId, PortId),

    #[error(
        "source channel ({0}) does not match the received packet's counterparty channel ({1})"
    )]
    SourceChannelMismatch(ChannelId, ChannelId),

    #[error(
        "source channel ({0}) does not match the received packet's counterparty channel ({1})"
    )]
    DestinationChannelMismatch(ChannelId, ChannelId),

    #[error("packet is already timed out")]
    TimedOutPacket,

    #[error("zero timeout is not allowed")]
    ZeroTimeout,

    #[error("committed packet ({comm}) does not match the calculated one ({exp_comm})", comm = serde_utils::to_hex(.0), exp_comm= serde_utils::to_hex(.1))]
    PacketCommitmentMismatch(Vec<u8>, Vec<u8>),

    #[error("empty packets received")]
    EmptyPacketsReceived,

    #[error("intents don't work with ordered IBC")]
    IntentOrderedPacket,
}

pub enum IbcVersion {
    V1,
    Union,
}

impl FromStr for IbcVersion {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "1" => Ok(IbcVersion::V1),
            "union" => Ok(IbcVersion::Union),
            _ => Err(()),
        }
    }
}

pub trait IbcHost: Sized {
    type Error: core::fmt::Display + core::fmt::Debug + PartialEq + From<IbcError>;

    fn version(&self, connection_id: ConnectionId) -> IbcVersion;

    fn caller(&self) -> Vec<u8>;

    fn next_client_identifier(&mut self, client_type: &str) -> Result<u32, Self::Error>;

    fn next_connection_identifier(&mut self) -> Result<ConnectionId, Self::Error>;

    fn next_channel_identifier(&mut self) -> Result<ChannelId, Self::Error>;

    fn client_state(&self, client_id: &u32) -> Option<Vec<u8>>;

    fn read(&self, key: &[u8]) -> Option<Vec<u8>>;

    fn read_decode<T: Decode<E>, E: Encoding>(&self, key: &[u8]) -> Result<Option<T>, Self::Error>
    where
        Self::Error: From<DecodeErrorOf<E, T>>,
    {
        self.read(key)
            .map(|value| Ok(T::decode(&value)?))
            .transpose()
    }

    fn commit(&mut self, key: &[u8], value: Vec<u8>) -> Result<(), Self::Error>;

    fn commit_encode<T: Encode<E>, E: Encoding>(
        &mut self,
        key: &[u8],
        value: T,
    ) -> Result<(), Self::Error> {
        self.commit(key, value.encode())
    }

    fn delete(&mut self, key: &Path) -> Result<(), Self::Error>;

    fn current_height(&self) -> Height;

    fn current_timestamp(&self) -> u64;

    fn sha256(&self, data: Vec<u8>) -> Vec<u8>;
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Copy, Clone)]
#[cfg_attr(feature = "schemars", derive(::schemars::JsonSchema))]
pub enum Status {
    Active,
    Frozen,
    Expired,
}

impl core::fmt::Display for Status {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "schemars", derive(::schemars::JsonSchema))]
pub enum IbcVmResponse {
    SendPacket { sequence: u64 },
    Empty,
}

pub type CallbackError = Option<String>;

// TODO(aeryz): rename this
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "schemars", derive(::schemars::JsonSchema))]
pub enum IbcResponse {
    Empty,
    Initialize,
    Status {
        status: Status,
    },
    LatestHeight {
        height: Height,
    },
    TimestampAtHeight {
        timestamp: u64,
    },
    VerifyMembership {
        valid: bool,
    },
    VerifyClientMessage {
        valid: bool,
    },
    CheckForMisbehaviour {
        misbehaviour_found: bool,
    },
    UpdateStateOnMisbehaviour,
    UpdateState {
        consensus_states: Vec<(Height, Vec<u8>)>,
        client_state: Vec<u8>,
    },
    OnChannelOpenInit {
        err: CallbackError,
    },
    OnChannelOpenTry {
        err: CallbackError,
    },
    OnChannelOpenAck {
        err: CallbackError,
    },
    OnChannelOpenConfirm {
        err: CallbackError,
    },
    OnRecvPacket {
        acks: Vec<Vec<u8>>,
    },
    OnAcknowledgePacket {
        err: CallbackError,
    },
}

#[derive(enumorph::Enumorph, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(::schemars::JsonSchema))]
pub enum IbcState {
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
    AcknowledgePacket(Acknowledgement),
}

macro_rules! cast_either {
    ($this:ident, $host:ident, $resp:ident, [ $($arm:ident), *]) => {
        match $this {
            $(IbcState::$arm(s) => match s.process($host, $resp)? {
                Either::Left((substate, msg)) => Either::Left((IbcState::$arm(substate), msg)),
                Either::Right(right) => Either::Right(right),
            },)*
        }
    };
}

impl<T: IbcHost> Runnable<T> for IbcState {
    fn process(
        self,
        host: &mut T,
        resp: &[IbcResponse],
    ) -> Result<Either<(Self, IbcAction), (Vec<IbcEvent>, IbcVmResponse)>, <T as IbcHost>::Error>
    {
        let res = cast_either!(
            self,
            host,
            resp,
            [
                CreateClient,
                UpdateClient,
                ConnectionOpenInit,
                ConnectionOpenTry,
                ConnectionOpenAck,
                ConnectionOpenConfirm,
                ChannelOpenInit,
                ChannelOpenTry,
                ChannelOpenAck,
                ChannelOpenConfirm,
                SendPacket,
                RecvPacket,
                AcknowledgePacket
            ]
        );
        Ok(res)
    }
}

impl From<(u32, Vec<IbcQuery>)> for IbcAction {
    fn from(value: (u32, Vec<IbcQuery>)) -> Self {
        IbcAction::Query(value)
    }
}

impl From<Vec<IbcMsg>> for IbcAction {
    fn from(value: Vec<IbcMsg>) -> Self {
        IbcAction::Write(value)
    }
}

impl From<IbcMsg> for IbcAction {
    fn from(value: IbcMsg) -> Self {
        IbcAction::Write(vec![value])
    }
}

#[derive(Deserialize)]
pub enum IbcAction {
    Query((u32, Vec<IbcQuery>)),
    Write(Vec<IbcMsg>),
}

#[derive(Serialize, Deserialize)]
pub enum IbcQuery {
    Status,
    LatestHeight,
    VerifyMembership {
        height: Height,
        delay_time_period: u64,
        delay_block_period: u64,
        proof: Vec<u8>,
        path: Vec<u8>,
        value: Vec<u8>,
    },

    VerifyClientMessage(Vec<u8>),

    CheckForMisbehaviour(Vec<u8>),

    TimestampAtHeight(Height),
}

#[derive(Deserialize)]
pub enum IbcMsg {
    Initialize {
        client_id: u32,
        client_type: String,
        client_state: Vec<u8>,
        consensus_state: Vec<u8>,
    },
    UpdateStateOnMisbehaviour {
        client_id: u32,
        client_msg: Vec<u8>,
    },

    UpdateState {
        client_id: u32,
        client_msg: Vec<u8>,
    },

    OnChannelOpenInit {
        order: ChannelOrder,
        connection_id: ConnectionId,
        channel_id: ChannelId,
        version: String,
    },

    OnChannelOpenTry {
        order: ChannelOrder,
        connection_id: ConnectionId,
        channel_id: ChannelId,
        counterparty_channel_id: ChannelId,
        version: String,
        counterparty_version: String,
    },

    OnChannelOpenAck {
        channel_id: ChannelId,
        counterparty_channel_id: String,
        counterparty_version: String,
    },

    OnChannelOpenConfirm {
        channel_id: ChannelId,
    },

    OnRecvPacket {
        packet: Packet,
        maker: Vec<u8>,
        maker_msg: Vec<u8>,
        // TODO(aeryz): relayer address
    },

    OnRecvIntentPacket {
        packet: Packet,
        maker: Vec<u8>,
        maker_msg: Vec<u8>,
    },

    OnAcknowledgePacket {
        packet: Packet,
        ack: Vec<u8>,
        relayer: Vec<u8>,
    },
}

pub trait Runnable<T: IbcHost>: Serialize + Sized {
    #[allow(clippy::type_complexity)]
    fn process(
        self,
        host: &mut T,
        resp: &[IbcResponse],
    ) -> Result<Either<(Self, IbcAction), (Vec<IbcEvent>, IbcVmResponse)>, <T as IbcHost>::Error>;
}

pub enum Either<L, R> {
    Left(L),
    Right(R),
}
