uniffi::setup_scaffolding!();

pub mod error;
pub mod keys;
pub mod ecdh;
pub mod cipher;
pub mod signing;
pub mod record;
pub mod ffi;
pub mod dht;
pub mod mnemonic;

pub use error::PktapError;
