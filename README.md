# P2P Network Handshake with Cardano Nodes

## 1. Goals
- [X] To demonstrate how to connect to cardano nodes as a node
- [X] Configure cardano nodes in configuration file viz. App.yaml
- [X] I/O should be non-blocking
- [X] Represent messages as close to CDDL encoding specification mentioned in Ref. 3.6.5 (Node to node handshake mini-protocol) section of 'network spec' of cardano.
- [X] Handling cardano node responses viz Succuess & Errors
- [X] Test cases to demonstrate usages of functions
- [X] Documentation 
    1. [X] How to run cardano-node locally
    2. [X] Documenting cardano handshake mini-protocol relevant for this application in PROTOCOL.md file
    3. [X] Adding references
    4. [X] In future what can be added

## 2. High level Logic for Node connectivity [Handshake mini-protocol]:

* Read list of hosts from App.yaml config
* For each host:
    1. open a TCP connection with the node
    2. negotiating the protocol version with the handshake mini-protocol
    3. discovering and classifying exceptions thrown by mini-protocols
    4. In case of an error, print the error message received
    5. Print the time to complete negotiation
    6. Close the TCP connection and shutdown thread
* Exit when handshake with all nodes are attempted


## 3. Running local cardano-node

- Open link given in Reference [5]. Check at the bottom of the page, section 'Assets'.
- Download cardano-node-8.1.2-linux.tar.gz or your platform specific file from Assets section.
- Open link given in Reference [4]. Download files named below, and store it under './cardano-rust-node/cardano-local folder'

    - Node Config
    - DB Sync Config
    - Submit API Config
    - Node Topology
    - Byron Genesis
    - Shelley Genesis
    - Alonzo Genesis
    - Conway Genesis

- Open command line and cd to ./cardano-rust-node/cardano-local folder.
- Run command below to run cardano-node

        cardano-node run \
          --topology ./topology.json \
          --database-path ./db \
          --socket-path ./db/node.socket \
          --host-addr 0.0.0.0 \
          --port 3001 \
          --config ./config.json

- It will create db folder when above command is run. Also above command expects all configs to be in same folder.

## 4. App.yaml file description

This file externalizes configurations.
It has host names & magic numbers of cardano nodes such as mainnet, testnet and also local node run by above command.
These nodes will be used by the code to perform handshake negotiation.

## 5. Testing:

1. Success Scenario: with local cardano-node
    1. Run local cardano-node with the help of section 3. above.
    2. No change in App.yaml file. It uses local node configuration.
    3. Run command:
        * RUST_LOG=info cargo run --release

2. Success Scenario: with "Main Net"
    1. Enable "Main Net" section of hosts and disable "Local Dev Net" section of hosts parameter.
        Config will look like this:
            
            hosts:
                # Main Net
                - network_id: "Main_Net"
                  host: "relays-new.cardano-mainnet.iohk.io:3001"
                  network_magic: 764824073
                .
                .
                # Local Dev Net
                #- network_id: "Local_Dev_Net"
                #host: "0.0.0.0:3001"
                #network_magic: 1

        Note: In Above config sample . in between vertical values represents other existing config should be kept as it is.
    2. Run command:
        * RUST_LOG=info cargo run --release

3. RefuseReasonVersionMismatch Scenario: Version 3 not supported
    1. Run local cardano-node with the help of section 3. above. 
    2. Change App.yaml file. In supported_versions section, enable 3 version and disable rest of the versions.
    Keep "hosts" section as per 1st or 2nd scenario. "supported_versions" section will be as below:

            supported_versions:
                # RefuseReasonVersionMismatch scenario
                - 3
                # Success scenario
                #- 7
                #- 8
                #- 9
                #- 10
                # RefuseReasonHandshakeDecodeError scenario
                #- 11

    3. Run command:
        * RUST_LOG=info cargo run --release

4. RefuseReasonHandshakeDecodeError Scenario: Version 11 needs extra parameters which is yet to be implemented, thus decode error.
    1. Run local cardano-node with the help of section 3. above. 
    2. Change App.yaml file. In supported_versions section, enable 3 version and disable rest of the versions.
    Keep "hosts" section as per 1st or 2nd scenario. "supported_versions" section will be as below:

            supported_versions:
                # RefuseReasonVersionMismatch scenario
                #- 3
                # Success scenario
                #- 7
                #- 8
                #- 9
                #- 10
                # RefuseReasonHandshakeDecodeError scenario
                - 11

    3. Run command:
        * RUST_LOG=info cargo run --release

5. Test cases execution:

There are two ways to run test cases.
First way: 

        cargo test -- --nocapture

Second way: 
    Install nextest using command

        cargo install nextest

    Then run below command:

        cargo nextest run


## 6. Error Handling:

1. Server failure response with RefuseReason is handled
2. If there is wrong message passed to server, it sends error code 104 (DecodeFailure). And shuts down the connection. Handled gracefully.


## 7. Future Possibilities:
1. Provide way to connect as a Client to other cardano-nodes
2. Support for ≥ 11 version_numbers in protocol


## 8. Reference:

1. https://input-output-hk.github.io/ouroboros-network/pdfs/network-spec/network-spec.pdf
2. https://input-output-hk.github.io/ouroboros-network/ouroboros-network-framework/Ouroboros-Network-Protocol-Handshake-Type#t:Handshake
3. https://github.com/input-output-hk/ouroboros-network/blob/d4e8622955145c97d49cbeb85d964d6b44ed87b7/ouroboros-network-framework/src/Ouroboros/Network/Protocol/Handshake.hs#L8
4. https://book.world.dev.cardano.org/environments.html#pre-production-testnet
5. https://github.com/input-output-hk/cardano-node/releases/tag/8.1.2

General Reading about cardano nodes:

6. https://docs.cardano.org/explore-cardano/cardano-network/about-the-cardano-network/
7. https://developers.cardano.org/docs/get-started/testnets-and-devnets/
8. https://developers.cardano.org/docs/integrate-cardano/testnet-faucet
9. https://docs.cardano.org/explore-cardano/cardano-network/p2p-networking/
10. https://developers.cardano.org/docs/get-started/running-cardano/
