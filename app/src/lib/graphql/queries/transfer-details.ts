import { graphql } from "../index.ts"

export const transfersBySourceHashBaseQueryDocument = graphql(/* GraphQL */ `
    query TransfersBySourceHashBase($source_transaction_hash: String!)
    @cached(ttl: 1) {
        v1_transfers(
            where: { source_transaction_hash: { _eq: $source_transaction_hash } }
        ) {
            sender
            normalized_sender
            source_chain_id
            source_connection_id
            source_channel_id
            source_sequence
            source_transaction_hash
            receiver
            normalized_receiver
            destination_chain_id
            destination_connection_id
            destination_channel_id
            destination_sequence
            tokens {
                denom
                amount
                asset {
                    denom
                    decimals
                    display_name
                    display_symbol
                }
            }
            source_timestamp
            destination_timestamp
            forwards {
                source_connection_id
                source_channel_id
                destination_connection_id
                destination_channel_id
                destination_chain_id
                source_channel_id
                receiver
            }
        }
    }
`)

export const transfersBySourceHashTracesAndHopsQueryDocument = graphql(/* GraphQL */ `
    query TransfersBySourceHashTracesAndHops($source_transaction_hash: String!)
    @cached(ttl: 1) {
      v1_transfers(
        where: { source_transaction_hash: { _eq: $source_transaction_hash } }
      ) {
        traces(order_by: { timestamp: asc }) {
          timestamp
          chain {
            chain_id
          }
          type
          transaction_hash
          height
        }
        hop {
          traces(order_by: { timestamp: asc }) {
            timestamp
            chain {
              chain_id
            }
            type
            transaction_hash
            height
          }
        }
      }
    }
  `)
