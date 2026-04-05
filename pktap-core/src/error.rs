/// PktapError is the unified error type for all pktap-core operations.
///
/// Per D-07: typed error enum via UniFFI maps to a Kotlin sealed class.
/// Per D-08: crypto failures coalesce to RecordInvalid across FFI to prevent oracle attacks.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum PktapError {
    #[error("Invalid key bytes")]
    InvalidKey,

    #[error("Record invalid or decryption failed")]
    RecordInvalid,

    #[error("Record payload too large")]
    RecordTooLarge,

    #[error("Serialization failed")]
    SerializationFailed,
}
