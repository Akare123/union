pub mod account_proof;
pub mod account_update;
pub mod beacon_block_header;
pub mod client_state;
pub mod consensus_state;
pub mod execution_payload_header;
pub mod fork;
pub mod fork_parameters;
pub mod header;
pub mod light_client_header;
pub mod light_client_update;
#[cfg(feature = "ssz")]
// TODO: Add an UnboundedMisbehaviour and remove the feature gate on this module
pub mod misbehaviour;
pub mod storage_proof;
pub mod sync_aggregate;
pub mod sync_committee;
pub mod trusted_sync_committee;
