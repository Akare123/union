use macros::model;

#[model(proto(raw(protos::tendermint::types::Data), from, into))]
pub struct Data {
    #[cfg_attr(feature = "serde", serde(with = "::serde_utils::hex_string_list"))]
    #[debug(wrap = ::serde_utils::fmt::DebugListAsHex)]
    pub txs: Vec<Vec<u8>>,
}

#[cfg(feature = "proto")]
pub mod proto {
    use crate::tendermint::types::data::Data;

    impl From<Data> for protos::tendermint::types::Data {
        fn from(value: Data) -> Self {
            Self { txs: value.txs }
        }
    }

    impl From<protos::tendermint::types::Data> for Data {
        fn from(value: protos::tendermint::types::Data) -> Self {
            Self { txs: value.txs }
        }
    }
}
