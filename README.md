# P2P Network Handshake with Cardano Nodes

## Goals
- [X] To demonstrate how to connect to cardano nodes as a node
- [X] Configure cardano nodes in configuration file viz. App.yaml
- [X] I/O should be non-blocking
- [X] Represent messages as close to CDDL encoding specification mentioned in Ref. 3.6.5 (Node to node handshake mini-protocol) section of 'network spec' of cardano.
- [X] Handling cardano node responses viz Succuess & Errors
- [ ] Test cases to demonstrate usages of functions
- [X] Documentation 
    * [X] How to run cardano-node locally
    * [X] Documenting cardano handshake mini-protocol relevant for this application in PROTOCOL.md file
    * [X] Adding references
    * [X] In future what can be added

## High level Logic for Node connectivity:

* Read list of hosts from App.yaml config
* For each host:
    * open a TCP connection with the node
    * negotiating the protocol version with the handshake mini-protocol
    * discovering and classifying exceptions thrown by mini-protocols
    * In case of an error, print the error message received
    * Print the time to complete negotiation
    * Close the TCP connection and shutdown thread
* Exit when handshake with all nodes are attempted


## Running local cardano-node

- Download cardano-node from link mentioned in Reference [5].
- Download configs for pre-production-testnet from link mentioned in Reference [4]
And store these configs in new folder for cardano-workspace, where you will run cardano-node.
It will create db folder when below command is run. Also below command expects all configs to be in same folder.

- Run command below to run cardano-node

cardano-node run \
  --topology ./topology.json \
  --database-path ./db \
  --socket-path ./db/node.socket \
  --host-addr 0.0.0.0 \
  --port 3001 \
  --config ./config.json


## App.yaml file description

This file externalizes configurations.
It has host names & magic numbers of cardano nodes such as mainnet, testnet and also local node run by above command.
These nodes will be used by the code to perform handshake negotiation.

## Testing:

To run test cases, execute below commands.


## Error Handling:


## Future Possibilities:
[1] Provide way to connect as a Client to other cardano-nodes
[2] Support for ≥ 11 version_numbers in protocol


## Reference:

[1] https://input-output-hk.github.io/ouroboros-network/pdfs/network-spec/network-spec.pdf
[2] https://input-output-hk.github.io/ouroboros-network/ouroboros-network-framework/Ouroboros-Network-Protocol-Handshake-Type#t:Handshake
[3] https://github.com/input-output-hk/ouroboros-network/blob/d4e8622955145c97d49cbeb85d964d6b44ed87b7/ouroboros-network-framework/src/Ouroboros/Network/Protocol/Handshake.hs#L8
[4] https://book.world.dev.cardano.org/environments.html#pre-production-testnet
[5] https://github.com/input-output-hk/cardano-node/releases/tag/8.1.2

General Reading about cardano nodes:
[6] https://docs.cardano.org/explore-cardano/cardano-network/about-the-cardano-network/
[7] https://developers.cardano.org/docs/get-started/testnets-and-devnets/
[8] https://developers.cardano.org/docs/integrate-cardano/testnet-faucet
[9] https://docs.cardano.org/explore-cardano/cardano-network/p2p-networking/
[10] https://developers.cardano.org/docs/get-started/running-cardano/
