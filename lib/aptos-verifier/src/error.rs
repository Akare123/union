use unionlabs::aptos::hash_value::HashValue;

use crate::MAX_ACCUMULATOR_PROOF_DEPTH;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum Error {
    #[error("root hash mismatch, expected ({expected}) given ({given})")]
    RootHashMismatch {
        expected: HashValue,
        given: HashValue,
    },
    #[error("accumulator proof hash has more than maximum ({MAX_ACCUMULATOR_PROOF_DEPTH}) siblings ({0})")]
    MaxSiblingsExceeded(usize),
    #[error("storage verification error")]
    StorageVerification(#[from] StorageVerificationError),
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum StorageVerificationError {
    #[error("accumulator proof hash has more than maximum ({0}) siblings ({1})")]
    MaxSiblingsExceeded(usize, usize),
    #[error("leaf key mismatch (({0}), ({1}))")]
    LeafKeyMismatch(HashValue, HashValue),
    #[error("leaf value mismatch (({0}), ({1}))")]
    LeafValueMismatch(HashValue, HashValue),
    #[error("expected membership verification")]
    ExpectedMembershipVerification,
    #[error("expected non-membership verification")]
    ExpectedNonMembershipVerification,
    #[error("root hash mismatch (({0}, {1}))")]
    RootHashMismatch(HashValue, HashValue),
}