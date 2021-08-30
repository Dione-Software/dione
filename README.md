# Dione Messaging Application
## Secure and Anonymous Messaging
__WARNING:__ Currently Dione is not ready to be used nor does it fulfill its goal of being an anonymous messenger.
In order to achieve that every client's traffic and maybe every node's traffic hast to be routed through an Onion Router.

At the present moment the following anonymization networks are considered for integration.
However, first other issues have to be resolved and none of these services have currently a stable, native and usable Rust client.

| Service | Favoured Client |
|---------|-----------------|
| [Tor](https://www.torproject.org) | [Arti](https://gitlab.torproject.org/tpo/core/arti)|
| I2P     | [I2p-rs](https://github.com/i2p/i2p-rs) (could be deprecated) |
| Lokinet | (none)          |


## What is Dione?

Dione is the attempt to build a messaging application that is as censorship resistant as possible.
This is achieved by not relying on a single entity for storing and distributing messages.
Instead, every message is split up into several parts and stored on several servers (nodes). These servers are only known
to sender and receiver. This is achieved by a simplified Double Ratchet (Address Ratchet). In the background
the Dione servers are connected via [libp2p](https://libp2p.io). [Kademlia](https://en.wikipedia.org/wiki/Kademlia) is utilized to find servers for the Address Ratchets
Output and to find providers for message parts.

#### A more detailed description will follow. In the process of standardizing and improving of Dione breaking changes are very likely

## Try out Dione yourself

Currently, there is no Dione main-net that one can just join. For now, you have to set up a test-net yourself.

TODO