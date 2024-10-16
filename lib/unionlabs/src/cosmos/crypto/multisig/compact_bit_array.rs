use macros::model;

#[model(proto(
    raw(protos::cosmos::crypto::multisig::v1beta1::CompactBitArray),
    from,
    into
))]
pub struct CompactBitArray {
    pub extra_bits_stored: u32,
    #[cfg_attr(feature = "serde", serde(with = "::serde_utils::hex_string"))]
    #[debug(wrap = ::serde_utils::fmt::DebugAsHex)]
    pub elems: Vec<u8>,
}

#[cfg(feature = "proto")]
pub mod proto {
    use crate::cosmos::crypto::multisig::compact_bit_array::CompactBitArray;

    impl From<CompactBitArray> for protos::cosmos::crypto::multisig::v1beta1::CompactBitArray {
        fn from(value: CompactBitArray) -> Self {
            Self {
                extra_bits_stored: value.extra_bits_stored,
                elems: value.elems,
            }
        }
    }

    impl From<protos::cosmos::crypto::multisig::v1beta1::CompactBitArray> for CompactBitArray {
        fn from(value: protos::cosmos::crypto::multisig::v1beta1::CompactBitArray) -> Self {
            Self {
                extra_bits_stored: value.extra_bits_stored,
                elems: value.elems,
            }
        }
    }
}
