# Handshake Mini-Protocol details

Below details are found in document from link mentioned in Reference [1].

## Handshake mini protocol actions for node to node communication

Send initial message MsgProposeVersions
Receive one of [MsgAcceptVersion, MsgRefuse]

## Header message format Ref. 2.1.1 section

Transmission Time                           - 16 bits
Mode                                        - 1 bit
Mini Protocol ID as in tables 2.2 and 2.3.  - 15 bits
Payload Length  - Max payload size 2^16 − 1 - 16 bits

### Header structure Ref. Table 2.1

 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+---------------------------------------------------------------+
|              transmission time                                |
+-------------------------------+-------------------------------+
|M|    conversation id          |              length           |
+-------------------------------+-------------------------------+

M -> Mode [0 => 'Request', 1 => 'Reply']
conversation id -> Mini Protocol ID
length -> length of the payload

## Transition Table from Ref. 3.6.2 State Machine

                            Transition Table
+----------+--------------------+-----------------------------------+-----------+
|from      | message/event      | parameters                        | to        |
+==========+====================+===================================+===========+
|StPropose | MsgProposeVersions | VersionTable                      | StConfirm |
|StConfirm | MsgReplyVersion    | VersionTable                      | StDone    |
|StConfirm | MsgAcceptVersion   | (VersionNumber, ExtraParameters)  | StDone    |
|StConfirm | MsgRefuse          | reason                            | StDone    |
+----------+--------------------+-----------------------------------+-----------+

