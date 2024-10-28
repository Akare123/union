// @generated
/// Value returned by eth_getProof
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StorageProof {
    /// NOTE: U256
    #[prost(bytes = "vec", tag = "1")]
    pub key: ::prost::alloc::vec::Vec<u8>,
    /// NOTE: U256
    #[prost(bytes = "vec", tag = "2")]
    pub value: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", repeated, tag = "3")]
    pub proof: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
}
impl ::prost::Name for StorageProof {
    const NAME: &'static str = "StorageProof";
    const PACKAGE: &'static str = "union.ibc.lightclients.ethereum.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("union.ibc.lightclients.ethereum.v1.{}", Self::NAME)
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AccountProof {
    /// NOTE: H256
    /// NOTE: eth_getProof.storageHash
    #[prost(bytes = "vec", tag = "1")]
    pub storage_root: ::prost::alloc::vec::Vec<u8>,
    /// NOTE: eth_getProof.accountProof
    #[prost(bytes = "vec", repeated, tag = "2")]
    pub proof: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
}
impl ::prost::Name for AccountProof {
    const NAME: &'static str = "AccountProof";
    const PACKAGE: &'static str = "union.ibc.lightclients.ethereum.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("union.ibc.lightclients.ethereum.v1.{}", Self::NAME)
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ClientState {
    #[prost(string, tag = "1")]
    pub chain_id: ::prost::alloc::string::String,
    #[prost(bytes = "vec", tag = "2")]
    pub genesis_validators_root: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint64, tag = "3")]
    pub min_sync_committee_participants: u64,
    #[prost(uint64, tag = "4")]
    pub genesis_time: u64,
    #[prost(message, optional, tag = "5")]
    pub fork_parameters: ::core::option::Option<ForkParameters>,
    #[prost(uint64, tag = "6")]
    pub seconds_per_slot: u64,
    #[prost(uint64, tag = "7")]
    pub slots_per_epoch: u64,
    #[prost(uint64, tag = "8")]
    pub epochs_per_sync_committee_period: u64,
    #[prost(uint64, tag = "9")]
    pub latest_slot: u64,
    #[prost(message, optional, tag = "10")]
    pub frozen_height:
        ::core::option::Option<super::super::super::super::super::ibc::core::client::v1::Height>,
    #[prost(bytes = "vec", tag = "11")]
    pub ibc_commitment_slot: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "12")]
    pub ibc_contract_address: ::prost::alloc::vec::Vec<u8>,
}
impl ::prost::Name for ClientState {
    const NAME: &'static str = "ClientState";
    const PACKAGE: &'static str = "union.ibc.lightclients.ethereum.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("union.ibc.lightclients.ethereum.v1.{}", Self::NAME)
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ConsensusState {
    #[prost(uint64, tag = "1")]
    pub slot: u64,
    #[prost(bytes = "vec", tag = "2")]
    pub state_root: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub storage_root: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint64, tag = "4")]
    pub timestamp: u64,
    #[prost(bytes = "vec", tag = "5")]
    pub current_sync_committee: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "6")]
    pub next_sync_committee: ::prost::alloc::vec::Vec<u8>,
}
impl ::prost::Name for ConsensusState {
    const NAME: &'static str = "ConsensusState";
    const PACKAGE: &'static str = "union.ibc.lightclients.ethereum.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("union.ibc.lightclients.ethereum.v1.{}", Self::NAME)
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Header {
    #[prost(message, optional, tag = "1")]
    pub trusted_height:
        ::core::option::Option<super::super::super::super::super::ibc::core::client::v1::Height>,
    #[prost(message, optional, tag = "2")]
    pub consensus_update: ::core::option::Option<LightClientUpdate>,
    #[prost(message, optional, tag = "3")]
    pub ibc_account_proof: ::core::option::Option<AccountProof>,
}
impl ::prost::Name for Header {
    const NAME: &'static str = "Header";
    const PACKAGE: &'static str = "union.ibc.lightclients.ethereum.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("union.ibc.lightclients.ethereum.v1.{}", Self::NAME)
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Misbehaviour {
    #[prost(message, optional, tag = "1")]
    pub trusted_height:
        ::core::option::Option<super::super::super::super::super::ibc::core::client::v1::Height>,
    #[prost(message, optional, tag = "2")]
    pub current_sync_committee: ::core::option::Option<SyncCommittee>,
    #[prost(message, optional, tag = "3")]
    pub update_1: ::core::option::Option<LightClientUpdate>,
    #[prost(message, optional, tag = "4")]
    pub update_2: ::core::option::Option<LightClientUpdate>,
}
impl ::prost::Name for Misbehaviour {
    const NAME: &'static str = "Misbehaviour";
    const PACKAGE: &'static str = "union.ibc.lightclients.ethereum.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("union.ibc.lightclients.ethereum.v1.{}", Self::NAME)
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ForkParameters {
    #[prost(bytes = "vec", tag = "1")]
    pub genesis_fork_version: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint64, tag = "2")]
    pub genesis_slot: u64,
    #[prost(message, optional, tag = "3")]
    pub altair: ::core::option::Option<Fork>,
    #[prost(message, optional, tag = "4")]
    pub bellatrix: ::core::option::Option<Fork>,
    #[prost(message, optional, tag = "5")]
    pub capella: ::core::option::Option<Fork>,
    #[prost(message, optional, tag = "6")]
    pub deneb: ::core::option::Option<Fork>,
}
impl ::prost::Name for ForkParameters {
    const NAME: &'static str = "ForkParameters";
    const PACKAGE: &'static str = "union.ibc.lightclients.ethereum.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("union.ibc.lightclients.ethereum.v1.{}", Self::NAME)
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Fork {
    #[prost(bytes = "vec", tag = "1")]
    pub version: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint64, tag = "2")]
    pub epoch: u64,
}
impl ::prost::Name for Fork {
    const NAME: &'static str = "Fork";
    const PACKAGE: &'static str = "union.ibc.lightclients.ethereum.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("union.ibc.lightclients.ethereum.v1.{}", Self::NAME)
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LightClientUpdate {
    #[prost(oneof = "light_client_update::Update", tags = "1, 2")]
    pub update: ::core::option::Option<light_client_update::Update>,
}
/// Nested message and enum types in `LightClientUpdate`.
pub mod light_client_update {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Update {
        #[prost(message, tag = "1")]
        EpochChangeUpdate(super::EpochChangeUpdate),
        #[prost(message, tag = "2")]
        WithinEpochUpdate(super::WithinEpochUpdate),
    }
}
impl ::prost::Name for LightClientUpdate {
    const NAME: &'static str = "LightClientUpdate";
    const PACKAGE: &'static str = "union.ibc.lightclients.ethereum.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("union.ibc.lightclients.ethereum.v1.{}", Self::NAME)
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EpochChangeUpdate {
    #[prost(message, optional, tag = "1")]
    pub sync_committee: ::core::option::Option<SyncCommittee>,
    #[prost(message, optional, tag = "2")]
    pub next_sync_committee: ::core::option::Option<SyncCommittee>,
    #[prost(bytes = "vec", repeated, tag = "3")]
    pub next_sync_committee_branch: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    #[prost(message, optional, tag = "4")]
    pub update_data: ::core::option::Option<LightClientUpdateData>,
}
impl ::prost::Name for EpochChangeUpdate {
    const NAME: &'static str = "EpochChangeUpdate";
    const PACKAGE: &'static str = "union.ibc.lightclients.ethereum.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("union.ibc.lightclients.ethereum.v1.{}", Self::NAME)
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WithinEpochUpdate {
    #[prost(message, optional, tag = "1")]
    pub sync_committee: ::core::option::Option<SyncCommittee>,
    #[prost(message, optional, tag = "2")]
    pub update_data: ::core::option::Option<LightClientUpdateData>,
}
impl ::prost::Name for WithinEpochUpdate {
    const NAME: &'static str = "WithinEpochUpdate";
    const PACKAGE: &'static str = "union.ibc.lightclients.ethereum.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("union.ibc.lightclients.ethereum.v1.{}", Self::NAME)
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LightClientUpdateData {
    #[prost(message, optional, tag = "1")]
    pub attested_header: ::core::option::Option<LightClientHeader>,
    #[prost(message, optional, tag = "2")]
    pub finalized_header: ::core::option::Option<LightClientHeader>,
    #[prost(bytes = "vec", repeated, tag = "3")]
    pub finality_branch: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    #[prost(message, optional, tag = "4")]
    pub sync_aggregate: ::core::option::Option<SyncAggregate>,
    #[prost(uint64, tag = "5")]
    pub signature_slot: u64,
}
impl ::prost::Name for LightClientUpdateData {
    const NAME: &'static str = "LightClientUpdateData";
    const PACKAGE: &'static str = "union.ibc.lightclients.ethereum.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("union.ibc.lightclients.ethereum.v1.{}", Self::NAME)
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SyncCommittee {
    #[prost(bytes = "vec", repeated, tag = "1")]
    #[cfg_attr(feature = "serde", serde(with = "::serde_utils::inner_base64"))]
    pub pubkeys: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    #[prost(bytes = "vec", tag = "2")]
    #[cfg_attr(feature = "serde", serde(with = "::serde_utils::base64"))]
    pub aggregate_pubkey: ::prost::alloc::vec::Vec<u8>,
}
impl ::prost::Name for SyncCommittee {
    const NAME: &'static str = "SyncCommittee";
    const PACKAGE: &'static str = "union.ibc.lightclients.ethereum.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("union.ibc.lightclients.ethereum.v1.{}", Self::NAME)
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SyncAggregate {
    #[prost(bytes = "vec", tag = "1")]
    #[cfg_attr(feature = "serde", serde(with = "::serde_utils::base64"))]
    pub sync_committee_bits: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    #[cfg_attr(feature = "serde", serde(with = "::serde_utils::base64"))]
    pub sync_committee_signature: ::prost::alloc::vec::Vec<u8>,
}
impl ::prost::Name for SyncAggregate {
    const NAME: &'static str = "SyncAggregate";
    const PACKAGE: &'static str = "union.ibc.lightclients.ethereum.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("union.ibc.lightclients.ethereum.v1.{}", Self::NAME)
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LightClientHeader {
    #[prost(message, optional, tag = "1")]
    pub beacon: ::core::option::Option<BeaconBlockHeader>,
    #[prost(message, optional, tag = "2")]
    pub execution: ::core::option::Option<ExecutionPayloadHeader>,
    #[prost(bytes = "vec", repeated, tag = "3")]
    #[cfg_attr(feature = "serde", serde(with = "::serde_utils::inner_base64"))]
    pub execution_branch: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
}
impl ::prost::Name for LightClientHeader {
    const NAME: &'static str = "LightClientHeader";
    const PACKAGE: &'static str = "union.ibc.lightclients.ethereum.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("union.ibc.lightclients.ethereum.v1.{}", Self::NAME)
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ExecutionPayloadHeader {
    #[prost(bytes = "vec", tag = "1")]
    #[cfg_attr(feature = "serde", serde(with = "::serde_utils::base64"))]
    pub parent_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    #[cfg_attr(feature = "serde", serde(with = "::serde_utils::base64"))]
    pub fee_recipient: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    #[cfg_attr(feature = "serde", serde(with = "::serde_utils::base64"))]
    pub state_root: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "4")]
    #[cfg_attr(feature = "serde", serde(with = "::serde_utils::base64"))]
    pub receipts_root: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "5")]
    #[cfg_attr(feature = "serde", serde(with = "::serde_utils::base64"))]
    pub logs_bloom: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "6")]
    #[cfg_attr(feature = "serde", serde(with = "::serde_utils::base64"))]
    pub prev_randao: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint64, tag = "7")]
    pub block_number: u64,
    #[prost(uint64, tag = "8")]
    pub gas_limit: u64,
    #[prost(uint64, tag = "9")]
    pub gas_used: u64,
    #[prost(uint64, tag = "10")]
    pub timestamp: u64,
    #[prost(bytes = "vec", tag = "11")]
    #[cfg_attr(feature = "serde", serde(with = "::serde_utils::base64"))]
    pub extra_data: ::prost::alloc::vec::Vec<u8>,
    /// TODO(aeryz): U256
    #[prost(bytes = "vec", tag = "12")]
    #[cfg_attr(feature = "serde", serde(with = "::serde_utils::base64"))]
    pub base_fee_per_gas: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "13")]
    #[cfg_attr(feature = "serde", serde(with = "::serde_utils::base64"))]
    pub block_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "14")]
    #[cfg_attr(feature = "serde", serde(with = "::serde_utils::base64"))]
    pub transactions_root: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "15")]
    #[cfg_attr(feature = "serde", serde(with = "::serde_utils::base64"))]
    pub withdrawals_root: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint64, tag = "16")]
    pub blob_gas_used: u64,
    #[prost(uint64, tag = "17")]
    pub excess_blob_gas: u64,
}
impl ::prost::Name for ExecutionPayloadHeader {
    const NAME: &'static str = "ExecutionPayloadHeader";
    const PACKAGE: &'static str = "union.ibc.lightclients.ethereum.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("union.ibc.lightclients.ethereum.v1.{}", Self::NAME)
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BeaconBlockHeader {
    #[prost(uint64, tag = "1")]
    pub slot: u64,
    #[prost(uint64, tag = "2")]
    pub proposer_index: u64,
    #[prost(bytes = "vec", tag = "3")]
    #[cfg_attr(feature = "serde", serde(with = "::serde_utils::base64"))]
    pub parent_root: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "4")]
    #[cfg_attr(feature = "serde", serde(with = "::serde_utils::base64"))]
    pub state_root: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "5")]
    #[cfg_attr(feature = "serde", serde(with = "::serde_utils::base64"))]
    pub body_root: ::prost::alloc::vec::Vec<u8>,
}
impl ::prost::Name for BeaconBlockHeader {
    const NAME: &'static str = "BeaconBlockHeader";
    const PACKAGE: &'static str = "union.ibc.lightclients.ethereum.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("union.ibc.lightclients.ethereum.v1.{}", Self::NAME)
    }
}
// @@protoc_insertion_point(module)
