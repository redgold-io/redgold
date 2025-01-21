
(SVG Logo) (Redgold) ---- Download - About - Explorer - Docs - Examples - Search bar 


Unified P2P Database & Compute Engine

Express application logic as flows & transforms

Redgold is a high performance horizontal permissionless database & smart contract platform. 
Build decentralized applications with SQL & executor backed UDFs. 

Applications are meant to be stateless and isolated from data stores. Conventional contracts persist internal variable state through serialization between successive invocations, a strategy which would be considered an anti-pattern for a conventional application.
Instead, SQL & database like queries should be supported as a first class citizen, with support for state isolation & application data model development. Blockchains are supposed to be a world-database, and yet none of that functionality is exposed to the user.

MARKETPLACE FOR NOTARIZATION

Blockchains should not be companies, but instead reflect an openly audited market for validation

Fees should not reflect ownership, but rather user preferences

RELATIVE MODELS

Centralized data structures can be avoided. Conflicts alone can be propagated and resolved independently, without enforcing a global state. Total hashes necessitate total agreement, leading to numerous problems. Local relative ordering leads to horizontal scalability

PORTFOLIO TARGET MODELS

Finance still remains the main use case of crypto. Support for ETFs, customizable trading models, strategies and derivative products is lacking. Bridge risk models, custodial redemption, and oracle pricing are relative operations, and need a new model for representation & safety.

DATA PIPELINES

Using the eUTXO model, data is naturally expressed as a series of transforms. Oracle sourced pricing information or other data is treated equivalently to modern database operations.

Transform APIs allow connecting SQL or other readers to UDFs to more complex mapping transforms in a seamless unified experience. Generic executors allow multiple languages & re-use of existing code

WHY REDGOLD

A different approach, using relative peer to peer scores

Social proof is the underlying phenomena used to prevent exceptions in PoW and PoS systems, and yet this process is obfuscated, manual, and error prone. A proper model should account for relative peer decisions in order to avoid centralization and conflicts

Secure Scaling


Local decisions based on node id determine data persistence for fuzzy sharding and redundancy
HASH DISTANCE

Peers flexibly attempt to
prioritize any transaction they have an incentive to verify in an open market
DYNAMIC FEES

End result is a collection of Merkle proofs proving acceptance of a given transaction from peers
MERKLE PROOF

Workers don't need to be supercomputers to participate and produce proofs
SMALL NODES

Anyone is welcome to contribute or collaborate on GitHub

Redgold
info@redgold.io



Welcome to the Redgold core docs page, here you'll find information about what Redgold is, how to use it, and how to participate in the network.

What is Redgold?

​

Redgold is a peer to peer crypto-currency / blockchain with a focus on exposing database functionality to application developers.


What inspired this?


The first motivating problem is that of the social proof issue with conventional networks. Blockchains are meant to provide security in an open context, but that security primarily derives from the socially accepted active software fork and chain data hash. PoW and PoS blockchains frequently discard the chain data to switch software versions, make manual revisions, or otherwise determine what is the "correct" network.


This process is extremely arbitrary, error-prone, subject to manipulation and attacks, and otherwise manual. Local peer decisions based on scores (even if manually calculated) are the ultimate arbiter of security. This process can benefit from inference and automation.


Additionally, as the end goal per transaction is simply a merkle proof associated with acceptance from the prior mentioned peers, there is no need for a sophisticated chain data structure. It can be eliminated & optimized away entirely, and only the final stage produced directly, yielding a "local" or relative model of security & validation


The second motivating problem is oriented around scaling issues associated with smart contracts & support for database-like interactions. Blockchain is conventionally treated as an open world database, but operations are expressed as state transitions associated with a single contract. This would be the equivalent of serializing a class as a blob in a database, reloading it and executing functions against it for a conventional application -- something that ignores proper data store design considerations entirely.


The third motivating problem is issues associated with DeFi & bridge design. Most "trustless" bridges obfuscate a serious security issue across networks, which is the inability to re-validate the state associated with a given blockchain. This is a fundamental limitation with being unable to run an entire node within the contract of another platform or network, and is unlikely to soon change. That limit creates a situation where the subset of peers which provide state information to the given contract, are themselves providing the security.


This ends up degenerating down to a similar risk profile as a multi-signature custodial group. There are some variations, but it fundamentally shares a great degree of commonality. The problem of course with these types of models, is that there are numerous potential groupings of peers, as seen by wrapper coins backed by central companies, competing wrapper coins on the same platform, and different contract implementations. All of this is a byproduct of attempting to mitigate risk with different designs, but it hints at a potential design constraint.


The correct model to represent this is again, a local or relative one. As security is determined by peers, and each peer may have a different risk profile rating of other peers, the ideal wrapper is actually a custom weighted portfolio of different bridge providers. This property extends as well to more complicated cases, such as ETFs and portfolio target models / contracts.


Quickstart

Check out the GitHub releases page to download the most recent binaries, or install with cargo directly against the git repo.