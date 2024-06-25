

# Sentinel

## Use cases

1. Send native tokens and wrapped tokens back periodically.

2. Trace the ibc events.

## Structure

### Config file

1. Chain's contain `chain_config` for `chain_utils::<chain>::Config`.

```json
{
  "chain_configs": {
    "ethereum": {
      "ethereum": {
        "enabled": true,
        "ibc_handler_address": "0xa390514f803a3b318b93bf6cd4beeb9f8299a0eb",
        "signers": [
          {
            "raw": "0xc7a7febac36c64d3cd757747494e4de734af5f05339a2fc818f589709e738fff"
          }
        ],
        "eth_rpc_api": "wss://eth-sepolia.g.alchemy.com/v2/Xn_VBUDyUtXUYb9O6b5ZmuBNDaSlH-BB",
        "transfer_module": {
          "type": "contract",
          "address": "0xd0081080ae8493cf7340458eaf4412030df5feeb"
        }
      }
    },
    "osmosis": {
      "cosmos": {
        "enabled": false,
        "chain_config": {
          "keyring": {
            "name": "osmosis-testnet",
            "keys": [
              {
                "type": "raw",
                "name": "osmosis-testnet-key0",
                "key": "0x1503e463998e28b130a2d4876632c80462bbd5e0d9eb7ce6ed5f6210f02a2913"
              }
            ]
          },
          "gas_config": {
            "gas_price": "1.0",
            "gas_denom": "uosmo",
            "gas_multiplier": "1.1",
            "max_gas": 400000
          },
          "fee_denom": "uosmo",
          "ws_url": "wss://rpc.osmo.test.yieldpay.finance/websocket",
          "grpc_url": "https://grpc.osmo.test.yieldpay.finance:443"
        },
        "transfer_module": {
          "type": "native"
        }
      }
    },
    "union": {
      "cosmos": {
        "enabled": true,
        "chain_config": {
          "keyring": {
            "name": "union-testnet",
            "keys": [
              {
                "type": "raw",
                "name": "union-testnet-key0",
                "key": "0xe6b7f3906f38ea3547c91ed2f5eab850d27dd5672424fa4759e471be45598860"
              }
            ]
          },
          "gas_config": {
            "gas_price": "1.0",
            "gas_denom": "muno",
            "gas_multiplier": "1.1",
            "max_gas": 400000
          },
          "fee_denom": "muno",
          "ws_url": "wss://rpc.testnet.bonlulu.uno/websocket",
          "prover_endpoint": "https://galois.testnet-8.union.build:443",
          "grpc_url": "https://grpc.testnet.bonlulu.uno"
        },
        "transfer_module": {
          "type": "contract",
          "address": "union177jpkxrhvzca0dhr7p05ty595ucdgdl6k4wv67500jxcu6t5hppqemdy20"
        }
      }
    }
  },
  "interactions": [
    {
      "source": {
        "chain": "ethereum",
        "channel": "channel-78"
      },
      "destination": {
        "chain": "union",
        "channel": "channel-83"
      },
      "protocol": {
        "Ucs01": {
          "receivers": ["union1qgvmcfkpd66wat6shhfas0z8z9dzp683mcj9tq"],
          "contract": "union1m37cxl0ld4uaw3r4lv9nt2uw69xxf8xfjrf7a4w9hamv6xvp6ddqqfaaaa"
        }
      },
      "memo": "{\"forward\":{\"receiver\":\"84cB5E16918547aD6181fe6513861a7eA476f2EC\",\"port\":\"wasm.union1m37cxl0ld4uaw3r4lv9nt2uw69xxf8xfjrf7a4w9hamv6xvp6ddqqfaaaa\",\"channel\":\"channel-70\"}}",
      "sending_memo_probability": 1,
      "denoms": [
        "0x1c7d4b196cb0c7b01d743fbc6116a902379c7238",
        "muno",
        "0x08210F9170F89Ab7658F0B5E3fF39b0E03C594D4"
      ],
      "send_packet_interval": 50,
      "expect_full_cycle": 35,
      "amount_min": 1,
      "amount_max": 3
    },
    {
      "source": {
        "chain": "union",
        "channel": "channel-83"
      },
      "destination": {
        "chain": "ethereum",
        "channel": "channel-78"
      },
      "protocol": {
        "Ucs01": {
          "receivers": ["0xfbf2b6f136feb11b738592c7c5cf63b83825ff46"],
          "contract": "union1m37cxl0ld4uaw3r4lv9nt2uw69xxf8xfjrf7a4w9hamv6xvp6ddqqfaaaa"
        }
      },
      "memo": "{\"forward\":{\"receiver\":\"84cB5E16918547aD6181fe6513861a7eA476f2EC\",\"port\":\"wasm.union1m37cxl0ld4uaw3r4lv9nt2uw69xxf8xfjrf7a4w9hamv6xvp6ddqqfaaaa\",\"channel\":\"channel-78\"}}",
      "sending_memo_probability": 0.0,
      "denoms": [
        "factory/union1m37cxl0ld4uaw3r4lv9nt2uw69xxf8xfjrf7a4w9hamv6xvp6ddqqfaaaa/0xe619529b4396a62ab6d88ff2bb195e83c11e909ad9",
        "factory/union1m37cxl0ld4uaw3r4lv9nt2uw69xxf8xfjrf7a4w9hamv6xvp6ddqqfaaaa/0x742f7232c73bea91be2828fa14129f68015c3f895b",
        "muno"
      ],
      "send_packet_interval": 50,
      "expect_full_cycle": 35,
      "amount_min": 1,
      "amount_max": 3
    }
  ],
  "single_interaction": {
    "source": {
      "chain": "ethereum",
      "channel": "channel-78"
    },
    "destination": {
      "chain": "union",
      "channel": "channel-83"
    },
    "protocol": {
      "Ucs01": {
        "receivers": ["union1qgvmcfkpd66wat6shhfas0z8z9dzp683mcj9tq"],
        "contract": "union1m37cxl0ld4uaw3r4lv9nt2uw69xxf8xfjrf7a4w9hamv6xvp6ddqqfaaaa"
      }
    },
    "memo": "{\"forward\":{\"receiver\":\"614E946f6D769Ad2983E4d4B81DDeBBFA51B09b5\",\"port\":\"wasm.union1m37cxl0ld4uaw3r4lv9nt2uw69xxf8xfjrf7a4w9hamv6xvp6ddqqfaaaa\",\"channel\":\"channel-79\"}}",
    "sending_memo_probability": 0,
    "denoms": [
      "factory/union1m37cxl0ld4uaw3r4lv9nt2uw69xxf8xfjrf7a4w9hamv6xvp6ddqqfaaaa/0x742f7232c73bea91be2828fa14129f68015c3f895b"
    ],
    "send_packet_interval": 1,
    "expect_full_cycle": 1,
    "amount_min": 1,
    "amount_max": 1
  }
}
```