//! `base64` 0.22.1
//!
//! Not on the deliverable's list. The oracle resolved four `Engine::encode` and
//! two `Engine::decode` calls to the trait's own declaration, so the trait, not
//! an impl, is what the engine must find. `general_purpose::URL_SAFE_NO_PAD` is
//! the only engine ankurah names, and `proto`'s id encoding depends on it being
//! exactly that alphabet.

pub trait Config {}
pub trait DecodeEstimate {}

pub trait Engine: Send + Sync {
    type Config: Config;
    type DecodeEstimate: DecodeEstimate;

    fn config(&self) -> &Self::Config;
    fn internal_decoded_len_estimate(&self, input_len: usize) -> Self::DecodeEstimate;
    fn encode<T: AsRef<[u8]>>(&self, input: T) -> String;
    fn decode<T: AsRef<[u8]>>(&self, input: T) -> Result<Vec<u8>, base64::DecodeError>;
    fn encode_string<T: AsRef<[u8]>>(&self, input: T, output_buf: &mut String);
    fn decode_slice<T: AsRef<[u8]>>(&self, input: T, output_buf: &mut [u8]) -> Result<usize, DecodeSliceError>;
}

pub mod engine {
    pub struct GeneralPurpose;
    pub struct GeneralPurposeConfig;
    pub struct GeneralPurposeEstimate;

    impl Config for GeneralPurposeConfig {}
    impl DecodeEstimate for GeneralPurposeEstimate {}

    impl Engine for GeneralPurpose {
        type Config = GeneralPurposeConfig;
        type DecodeEstimate = GeneralPurposeEstimate;

        fn config(&self) -> &GeneralPurposeConfig { todo!() }
        fn internal_decoded_len_estimate(&self, input_len: usize) -> GeneralPurposeEstimate { todo!() }
        fn encode<T: AsRef<[u8]>>(&self, input: T) -> String { todo!() }
        fn decode<T: AsRef<[u8]>>(&self, input: T) -> Result<Vec<u8>, base64::DecodeError> { todo!() }
        fn encode_string<T: AsRef<[u8]>>(&self, input: T, output_buf: &mut String) { todo!() }
        fn decode_slice<T: AsRef<[u8]>>(&self, input: T, output_buf: &mut [u8]) -> Result<usize, DecodeSliceError> { todo!() }
    }

    pub mod general_purpose {
        pub const STANDARD: GeneralPurpose = GeneralPurpose;
        pub const STANDARD_NO_PAD: GeneralPurpose = GeneralPurpose;
        pub const URL_SAFE: GeneralPurpose = GeneralPurpose;
        pub const URL_SAFE_NO_PAD: GeneralPurpose = GeneralPurpose;
    }
}

pub enum DecodeError {
    InvalidByte(usize, u8),
    InvalidLength(usize),
    InvalidLastSymbol(usize, u8),
    InvalidPadding,
}

impl Debug for DecodeError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Display for DecodeError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Clone for DecodeError { fn clone(&self) -> base64::DecodeError { todo!() } }
impl PartialEq for DecodeError { fn eq(&self, other: &base64::DecodeError) -> bool { todo!() } }
impl std::error::Error for DecodeError {}

pub enum DecodeSliceError {
    DecodeError(base64::DecodeError),
    OutputSliceTooSmall,
}

impl Debug for DecodeSliceError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Display for DecodeSliceError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::error::Error for DecodeSliceError {}
