export const ucs02NftAbi = <const>[
  {
    type: "constructor",
    inputs: [],
    stateMutability: "nonpayable"
  },
  {
    type: "function",
    name: "UPGRADE_INTERFACE_VERSION",
    inputs: [],
    outputs: [
      {
        name: "",
        type: "string",
        internalType: "string"
      }
    ],
    stateMutability: "view"
  },
  {
    type: "function",
    name: "getOutstanding",
    inputs: [
      {
        name: "sourcePort",
        type: "string",
        internalType: "string"
      },
      {
        name: "sourceChannel",
        type: "string",
        internalType: "string"
      },
      {
        name: "token",
        type: "address",
        internalType: "address"
      }
    ],
    outputs: [
      {
        name: "",
        type: "uint256",
        internalType: "uint256"
      }
    ],
    stateMutability: "view"
  },
  {
    type: "function",
    name: "ibcAddress",
    inputs: [],
    outputs: [
      {
        name: "",
        type: "address",
        internalType: "address"
      }
    ],
    stateMutability: "view"
  },
  {
    type: "function",
    name: "initialize",
    inputs: [
      {
        name: "_ibcHandler",
        type: "address",
        internalType: "contract IIBCPacket"
      },
      {
        name: "admin",
        type: "address",
        internalType: "address"
      }
    ],
    outputs: [],
    stateMutability: "nonpayable"
  },
  {
    type: "function",
    name: "onAcknowledgementPacket",
    inputs: [
      {
        name: "ibcPacket",
        type: "tuple",
        internalType: "struct IbcCoreChannelV1Packet.Data",
        components: [
          {
            name: "sequence",
            type: "uint64",
            internalType: "uint64"
          },
          {
            name: "source_port",
            type: "string",
            internalType: "string"
          },
          {
            name: "source_channel",
            type: "string",
            internalType: "string"
          },
          {
            name: "destination_port",
            type: "string",
            internalType: "string"
          },
          {
            name: "destination_channel",
            type: "string",
            internalType: "string"
          },
          {
            name: "data",
            type: "bytes",
            internalType: "bytes"
          },
          {
            name: "timeout_height",
            type: "tuple",
            internalType: "struct IbcCoreClientV1Height.Data",
            components: [
              {
                name: "revision_number",
                type: "uint64",
                internalType: "uint64"
              },
              {
                name: "revision_height",
                type: "uint64",
                internalType: "uint64"
              }
            ]
          },
          {
            name: "timeout_timestamp",
            type: "uint64",
            internalType: "uint64"
          }
        ]
      },
      {
        name: "acknowledgement",
        type: "bytes",
        internalType: "bytes"
      },
      {
        name: "_relayer",
        type: "address",
        internalType: "address"
      }
    ],
    outputs: [],
    stateMutability: "nonpayable"
  },
  {
    type: "function",
    name: "onChanCloseConfirm",
    inputs: [
      {
        name: "_portId",
        type: "string",
        internalType: "string"
      },
      {
        name: "_channelId",
        type: "string",
        internalType: "string"
      }
    ],
    outputs: [],
    stateMutability: "nonpayable"
  },
  {
    type: "function",
    name: "onChanCloseInit",
    inputs: [
      {
        name: "_portId",
        type: "string",
        internalType: "string"
      },
      {
        name: "_channelId",
        type: "string",
        internalType: "string"
      }
    ],
    outputs: [],
    stateMutability: "nonpayable"
  },
  {
    type: "function",
    name: "onChanOpenAck",
    inputs: [
      {
        name: "portId",
        type: "string",
        internalType: "string"
      },
      {
        name: "channelId",
        type: "string",
        internalType: "string"
      },
      {
        name: "counterpartyChannelId",
        type: "string",
        internalType: "string"
      },
      {
        name: "counterpartyVersion",
        type: "string",
        internalType: "string"
      }
    ],
    outputs: [],
    stateMutability: "nonpayable"
  },
  {
    type: "function",
    name: "onChanOpenConfirm",
    inputs: [
      {
        name: "_portId",
        type: "string",
        internalType: "string"
      },
      {
        name: "_channelId",
        type: "string",
        internalType: "string"
      }
    ],
    outputs: [],
    stateMutability: "nonpayable"
  },
  {
    type: "function",
    name: "onChanOpenInit",
    inputs: [
      {
        name: "order",
        type: "uint8",
        internalType: "enum IbcCoreChannelV1GlobalEnums.Order"
      },
      {
        name: "_connectionHops",
        type: "string[]",
        internalType: "string[]"
      },
      {
        name: "portId",
        type: "string",
        internalType: "string"
      },
      {
        name: "channelId",
        type: "string",
        internalType: "string"
      },
      {
        name: "counterpartyEndpoint",
        type: "tuple",
        internalType: "struct IbcCoreChannelV1Counterparty.Data",
        components: [
          {
            name: "port_id",
            type: "string",
            internalType: "string"
          },
          {
            name: "channel_id",
            type: "string",
            internalType: "string"
          }
        ]
      },
      {
        name: "version",
        type: "string",
        internalType: "string"
      }
    ],
    outputs: [],
    stateMutability: "nonpayable"
  },
  {
    type: "function",
    name: "onChanOpenTry",
    inputs: [
      {
        name: "order",
        type: "uint8",
        internalType: "enum IbcCoreChannelV1GlobalEnums.Order"
      },
      {
        name: "_connectionHops",
        type: "string[]",
        internalType: "string[]"
      },
      {
        name: "portId",
        type: "string",
        internalType: "string"
      },
      {
        name: "channelId",
        type: "string",
        internalType: "string"
      },
      {
        name: "counterpartyEndpoint",
        type: "tuple",
        internalType: "struct IbcCoreChannelV1Counterparty.Data",
        components: [
          {
            name: "port_id",
            type: "string",
            internalType: "string"
          },
          {
            name: "channel_id",
            type: "string",
            internalType: "string"
          }
        ]
      },
      {
        name: "version",
        type: "string",
        internalType: "string"
      },
      {
        name: "counterpartyVersion",
        type: "string",
        internalType: "string"
      }
    ],
    outputs: [],
    stateMutability: "nonpayable"
  },
  {
    type: "function",
    name: "onERC721Received",
    inputs: [
      {
        name: "",
        type: "address",
        internalType: "address"
      },
      {
        name: "",
        type: "address",
        internalType: "address"
      },
      {
        name: "",
        type: "uint256",
        internalType: "uint256"
      },
      {
        name: "",
        type: "bytes",
        internalType: "bytes"
      }
    ],
    outputs: [
      {
        name: "",
        type: "bytes4",
        internalType: "bytes4"
      }
    ],
    stateMutability: "pure"
  },
  {
    type: "function",
    name: "onRecvPacket",
    inputs: [
      {
        name: "ibcPacket",
        type: "tuple",
        internalType: "struct IbcCoreChannelV1Packet.Data",
        components: [
          {
            name: "sequence",
            type: "uint64",
            internalType: "uint64"
          },
          {
            name: "source_port",
            type: "string",
            internalType: "string"
          },
          {
            name: "source_channel",
            type: "string",
            internalType: "string"
          },
          {
            name: "destination_port",
            type: "string",
            internalType: "string"
          },
          {
            name: "destination_channel",
            type: "string",
            internalType: "string"
          },
          {
            name: "data",
            type: "bytes",
            internalType: "bytes"
          },
          {
            name: "timeout_height",
            type: "tuple",
            internalType: "struct IbcCoreClientV1Height.Data",
            components: [
              {
                name: "revision_number",
                type: "uint64",
                internalType: "uint64"
              },
              {
                name: "revision_height",
                type: "uint64",
                internalType: "uint64"
              }
            ]
          },
          {
            name: "timeout_timestamp",
            type: "uint64",
            internalType: "uint64"
          }
        ]
      },
      {
        name: "relayer",
        type: "address",
        internalType: "address"
      }
    ],
    outputs: [
      {
        name: "",
        type: "bytes",
        internalType: "bytes"
      }
    ],
    stateMutability: "nonpayable"
  },
  {
    type: "function",
    name: "onRecvPacketProcessing",
    inputs: [
      {
        name: "ibcPacket",
        type: "tuple",
        internalType: "struct IbcCoreChannelV1Packet.Data",
        components: [
          {
            name: "sequence",
            type: "uint64",
            internalType: "uint64"
          },
          {
            name: "source_port",
            type: "string",
            internalType: "string"
          },
          {
            name: "source_channel",
            type: "string",
            internalType: "string"
          },
          {
            name: "destination_port",
            type: "string",
            internalType: "string"
          },
          {
            name: "destination_channel",
            type: "string",
            internalType: "string"
          },
          {
            name: "data",
            type: "bytes",
            internalType: "bytes"
          },
          {
            name: "timeout_height",
            type: "tuple",
            internalType: "struct IbcCoreClientV1Height.Data",
            components: [
              {
                name: "revision_number",
                type: "uint64",
                internalType: "uint64"
              },
              {
                name: "revision_height",
                type: "uint64",
                internalType: "uint64"
              }
            ]
          },
          {
            name: "timeout_timestamp",
            type: "uint64",
            internalType: "uint64"
          }
        ]
      },
      {
        name: "relayer",
        type: "address",
        internalType: "address"
      }
    ],
    outputs: [],
    stateMutability: "nonpayable"
  },
  {
    type: "function",
    name: "onTimeoutPacket",
    inputs: [
      {
        name: "ibcPacket",
        type: "tuple",
        internalType: "struct IbcCoreChannelV1Packet.Data",
        components: [
          {
            name: "sequence",
            type: "uint64",
            internalType: "uint64"
          },
          {
            name: "source_port",
            type: "string",
            internalType: "string"
          },
          {
            name: "source_channel",
            type: "string",
            internalType: "string"
          },
          {
            name: "destination_port",
            type: "string",
            internalType: "string"
          },
          {
            name: "destination_channel",
            type: "string",
            internalType: "string"
          },
          {
            name: "data",
            type: "bytes",
            internalType: "bytes"
          },
          {
            name: "timeout_height",
            type: "tuple",
            internalType: "struct IbcCoreClientV1Height.Data",
            components: [
              {
                name: "revision_number",
                type: "uint64",
                internalType: "uint64"
              },
              {
                name: "revision_height",
                type: "uint64",
                internalType: "uint64"
              }
            ]
          },
          {
            name: "timeout_timestamp",
            type: "uint64",
            internalType: "uint64"
          }
        ]
      },
      {
        name: "_relayer",
        type: "address",
        internalType: "address"
      }
    ],
    outputs: [],
    stateMutability: "nonpayable"
  },
  {
    type: "function",
    name: "owner",
    inputs: [],
    outputs: [
      {
        name: "",
        type: "address",
        internalType: "address"
      }
    ],
    stateMutability: "view"
  },
  {
    type: "function",
    name: "paused",
    inputs: [],
    outputs: [
      {
        name: "",
        type: "bool",
        internalType: "bool"
      }
    ],
    stateMutability: "view"
  },
  {
    type: "function",
    name: "proxiableUUID",
    inputs: [],
    outputs: [
      {
        name: "",
        type: "bytes32",
        internalType: "bytes32"
      }
    ],
    stateMutability: "view"
  },
  {
    type: "function",
    name: "renounceOwnership",
    inputs: [],
    outputs: [],
    stateMutability: "nonpayable"
  },
  {
    type: "function",
    name: "send",
    inputs: [
      {
        name: "sourcePort",
        type: "string",
        internalType: "string"
      },
      {
        name: "sourceChannel",
        type: "string",
        internalType: "string"
      },
      {
        name: "receiver",
        type: "string",
        internalType: "string"
      },
      {
        name: "nftClass",
        type: "address",
        internalType: "address"
      },
      {
        name: "tokens",
        type: "uint256[]",
        internalType: "uint256[]"
      },
      {
        name: "timeoutTimestamp",
        type: "uint64",
        internalType: "uint64"
      }
    ],
    outputs: [],
    stateMutability: "nonpayable"
  },
  {
    type: "function",
    name: "transferOwnership",
    inputs: [
      {
        name: "newOwner",
        type: "address",
        internalType: "address"
      }
    ],
    outputs: [],
    stateMutability: "nonpayable"
  },
  {
    type: "function",
    name: "upgradeToAndCall",
    inputs: [
      {
        name: "newImplementation",
        type: "address",
        internalType: "address"
      },
      {
        name: "data",
        type: "bytes",
        internalType: "bytes"
      }
    ],
    outputs: [],
    stateMutability: "payable"
  },
  {
    type: "event",
    name: "ClassCreated",
    inputs: [
      {
        name: "packetSequence",
        type: "uint64",
        indexed: false,
        internalType: "uint64"
      },
      {
        name: "channelId",
        type: "string",
        indexed: false,
        internalType: "string"
      },
      {
        name: "nftClass",
        type: "address",
        indexed: false,
        internalType: "address"
      }
    ],
    anonymous: false
  },
  {
    type: "event",
    name: "Initialized",
    inputs: [
      {
        name: "version",
        type: "uint64",
        indexed: false,
        internalType: "uint64"
      }
    ],
    anonymous: false
  },
  {
    type: "event",
    name: "OwnershipTransferred",
    inputs: [
      {
        name: "previousOwner",
        type: "address",
        indexed: true,
        internalType: "address"
      },
      {
        name: "newOwner",
        type: "address",
        indexed: true,
        internalType: "address"
      }
    ],
    anonymous: false
  },
  {
    type: "event",
    name: "Paused",
    inputs: [
      {
        name: "account",
        type: "address",
        indexed: false,
        internalType: "address"
      }
    ],
    anonymous: false
  },
  {
    type: "event",
    name: "Received",
    inputs: [
      {
        name: "packetSequence",
        type: "uint64",
        indexed: false,
        internalType: "uint64"
      },
      {
        name: "channelId",
        type: "string",
        indexed: false,
        internalType: "string"
      },
      {
        name: "sender",
        type: "string",
        indexed: false,
        internalType: "string"
      },
      {
        name: "receiver",
        type: "address",
        indexed: false,
        internalType: "address"
      },
      {
        name: "nftClass",
        type: "address",
        indexed: false,
        internalType: "address"
      },
      {
        name: "tokenIds",
        type: "uint256[]",
        indexed: false,
        internalType: "uint256[]"
      }
    ],
    anonymous: false
  },
  {
    type: "event",
    name: "Refunded",
    inputs: [
      {
        name: "packetSequence",
        type: "uint64",
        indexed: false,
        internalType: "uint64"
      },
      {
        name: "channelId",
        type: "string",
        indexed: false,
        internalType: "string"
      },
      {
        name: "sender",
        type: "address",
        indexed: false,
        internalType: "address"
      },
      {
        name: "receiver",
        type: "string",
        indexed: false,
        internalType: "string"
      },
      {
        name: "nftClass",
        type: "address",
        indexed: false,
        internalType: "address"
      },
      {
        name: "tokenIds",
        type: "uint256[]",
        indexed: false,
        internalType: "uint256[]"
      }
    ],
    anonymous: false
  },
  {
    type: "event",
    name: "Sent",
    inputs: [
      {
        name: "packetSequence",
        type: "uint64",
        indexed: false,
        internalType: "uint64"
      },
      {
        name: "channelId",
        type: "string",
        indexed: false,
        internalType: "string"
      },
      {
        name: "sender",
        type: "address",
        indexed: false,
        internalType: "address"
      },
      {
        name: "receiver",
        type: "string",
        indexed: false,
        internalType: "string"
      },
      {
        name: "nftClass",
        type: "address",
        indexed: false,
        internalType: "address"
      },
      {
        name: "tokenIds",
        type: "uint256[]",
        indexed: false,
        internalType: "uint256[]"
      }
    ],
    anonymous: false
  },
  {
    type: "event",
    name: "Unpaused",
    inputs: [
      {
        name: "account",
        type: "address",
        indexed: false,
        internalType: "address"
      }
    ],
    anonymous: false
  },
  {
    type: "event",
    name: "Upgraded",
    inputs: [
      {
        name: "implementation",
        type: "address",
        indexed: true,
        internalType: "address"
      }
    ],
    anonymous: false
  },
  {
    type: "error",
    name: "AddressEmptyCode",
    inputs: [
      {
        name: "target",
        type: "address",
        internalType: "address"
      }
    ]
  },
  {
    type: "error",
    name: "ERC1967InvalidImplementation",
    inputs: [
      {
        name: "implementation",
        type: "address",
        internalType: "address"
      }
    ]
  },
  {
    type: "error",
    name: "ERC1967NonPayable",
    inputs: []
  },
  {
    type: "error",
    name: "EnforcedPause",
    inputs: []
  },
  {
    type: "error",
    name: "ErrInvalidAcknowledgement",
    inputs: []
  },
  {
    type: "error",
    name: "ErrInvalidCounterpartyProtocolVersion",
    inputs: []
  },
  {
    type: "error",
    name: "ErrInvalidHexAddress",
    inputs: []
  },
  {
    type: "error",
    name: "ErrInvalidProtocolOrdering",
    inputs: []
  },
  {
    type: "error",
    name: "ErrInvalidProtocolVersion",
    inputs: []
  },
  {
    type: "error",
    name: "ErrNotIBC",
    inputs: []
  },
  {
    type: "error",
    name: "ErrUnauthorized",
    inputs: []
  },
  {
    type: "error",
    name: "ErrUnstoppable",
    inputs: []
  },
  {
    type: "error",
    name: "ExpectedPause",
    inputs: []
  },
  {
    type: "error",
    name: "FailedInnerCall",
    inputs: []
  },
  {
    type: "error",
    name: "InvalidInitialization",
    inputs: []
  },
  {
    type: "error",
    name: "MustTransferAtLeastOneToken",
    inputs: []
  },
  {
    type: "error",
    name: "NotInitializing",
    inputs: []
  },
  {
    type: "error",
    name: "OwnableInvalidOwner",
    inputs: [
      {
        name: "owner",
        type: "address",
        internalType: "address"
      }
    ]
  },
  {
    type: "error",
    name: "OwnableUnauthorizedAccount",
    inputs: [
      {
        name: "account",
        type: "address",
        internalType: "address"
      }
    ]
  },
  {
    type: "error",
    name: "UUPSUnauthorizedCallContext",
    inputs: []
  },
  {
    type: "error",
    name: "UUPSUnsupportedProxiableUUID",
    inputs: [
      {
        name: "slot",
        type: "bytes32",
        internalType: "bytes32"
      }
    ]
  }
]