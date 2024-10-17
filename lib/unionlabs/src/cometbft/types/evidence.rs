use macros::model;

use crate::{
    cometbft::types::{
        duplicate_vote_evidence::{DuplicateVoteEvidence, TryFromDuplicateVoteEvidenceError},
        light_client_attack_evidence::{
            LightClientAttackEvidence, TryFromLightClientAttackEvidenceError,
        },
    },
    errors::{required, MissingField},
};

#[model(proto(raw(protos::cometbft::types::v1::Evidence), into, from))]
#[allow(clippy::large_enum_variant)]
pub enum Evidence {
    DuplicateVote(DuplicateVoteEvidence),
    LightClientAttack(LightClientAttackEvidence),
}

impl From<Evidence> for protos::cometbft::types::v1::Evidence {
    fn from(value: Evidence) -> Self {
        Self {
            sum: Some(match value {
                Evidence::DuplicateVote(e) => {
                    protos::cometbft::types::v1::evidence::Sum::DuplicateVoteEvidence(e.into())
                }
                Evidence::LightClientAttack(e) => {
                    protos::cometbft::types::v1::evidence::Sum::LightClientAttackEvidence(e.into())
                }
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum TryFromEvidenceError {
    #[error(transparent)]
    MissingField(#[from] MissingField),
    #[error("invalid duplicate vote evidence")]
    DuplicateVote(#[from] TryFromDuplicateVoteEvidenceError),
    #[error("invalid light client attack evidence")]
    LightClientAttack(#[from] TryFromLightClientAttackEvidenceError),
}

impl TryFrom<protos::cometbft::types::v1::Evidence> for Evidence {
    type Error = TryFromEvidenceError;

    fn try_from(value: protos::cometbft::types::v1::Evidence) -> Result<Self, Self::Error> {
        Ok(match required!(value.sum)? {
            protos::cometbft::types::v1::evidence::Sum::DuplicateVoteEvidence(e) => {
                Self::DuplicateVote(e.try_into()?)
            }
            protos::cometbft::types::v1::evidence::Sum::LightClientAttackEvidence(e) => {
                Self::LightClientAttack(e.try_into()?)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn json() {
        let json = r#"
{
  "type": "tendermint/DuplicateVoteEvidence",
  "value": {
    "vote_a": {
      "type": 2,
      "height": "1376375",
      "round": 0,
      "block_id": {
        "hash": "",
        "parts": {
          "total": 0,
          "hash": ""
        }
      },
      "timestamp": "2024-07-10T19:08:48.638106489Z",
      "validator_address": "D9ED770DE0106B3F2BDFD0D74DB8923C1A5A2ECA",
      "validator_index": 102,
      "signature": "qAlcTiG2aHT0+LbDThS9Q1Z3EDKrJgr7iUX5hyBUx0HQRPp5kXz83wL33IIaxV+BAhckoqfw8Iuef3SpOerI3mz9s3fr8trxewTk1cnFeBc2EzBGegLAztY4plFcl6cl",
      "extension": null,
      "extension_signature": null
    },
    "vote_b": {
      "type": 2,
      "height": "1376375",
      "round": 0,
      "block_id": {
        "hash": "3FA185C5CABCF3932144BAAB0B23CC70A2A8A58DE085854FD17B18E0CC0546B5",
        "parts": {
          "total": 1,
          "hash": "50FD744CA1FE21094B4C4509A885D82143661B7EC2E895E4758AFE755C0FABE7"
        }
      },
      "timestamp": "2024-07-10T19:08:48.193419475Z",
      "validator_address": "D9ED770DE0106B3F2BDFD0D74DB8923C1A5A2ECA",
      "validator_index": 102,
      "signature": "puUC4TuJtj1Wb3zM0DPWL/cK12babXitsLV7w3sxRshXOC9DmRTHMBk2fwu32g8NCU1Q2Z+hCJZWi1LtcxeVY05sSVenjnV99v45R2K0+xcdoZsqrKyT65J7x/F6S4Fv",
      "extension": null,
      "extension_signature": null
    },
    "TotalVotingPower": "3936000000000",
    "ValidatorPower": "32000000000",
    "Timestamp": "2024-07-10T19:08:46.622139607Z"
  }
}
"#;

        let evidence = serde_json::from_str::<protos::cometbft::types::v1::Evidence>(json).unwrap();

        dbg!(evidence);
    }
}
