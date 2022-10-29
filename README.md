# <img src="src/resources/logo_small.png" width="4%" height="4%"> Redgold


![Dev](https://github.com/redgold-labs/redgold/actions/workflows/ci.yml/badge.svg?branch=dev) | 
[Website](https://redgold.io) |
[Contributing](docs/CONTRIBUTING.md) | [Dev Setup](docs/dev_setup.md) | 
[Whitepaper](docs/whitepaper.md) | [Run A Node](docs/run_a_node.md) | 
[Security Procedures](docs/security_procedures.md)


Redgold is a peer-to-peer database and SQL compute engine bringing Spark and pandas like data transformation & 
conventional database usage patterns & functionality to crypto. WASM executors are used to chain together transforms
operating on SQL-like data loading functions. Protobuf is used for relational 
algebra descriptors and for raw signature operations, and Arrow is used as a cross-memory format for WASM 
invocations, with sqlite tables for frequent access and parquet tables for long-lived data indexes. All operations 
are translated to work with Kademlia 
distances. [ACCEPT](https://arxiv.org/pdf/2108.05236.pdf) consensus protocol is the most similar to the demonstrated 
primary optimization technique. For a full technical description and motivation of this project please refer 
above to the [whitepaper](docs/whitepaper.md). The main product focus is on ETFs & portfolio target models, with 
redeemable assets from other cryptocurrencies connected tightly with financial (oracle) data stores.



