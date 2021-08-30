//! # Dione-Lib
//! This is the `no_std` library of Dione implementing all needed cryptographic primitives and most
//! importantly the [Magic Ratchet].
//!
#![no_std]

extern crate alloc;

pub mod compression;
pub mod cryptography;
pub mod hashing;

pub use cryptography::ratchet::MagicRatchet;