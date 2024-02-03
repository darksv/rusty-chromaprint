//! Pure Rust port of [chromaprint](https://acoustid.org/chromaprint)

pub use fingerprint_matcher::{match_fingerprints, Segment};
pub use fingerprinter::{Configuration, Fingerprinter};

mod audio_processor;
mod chroma;
mod chroma_filter;
mod chroma_normalizer;
mod classifier;
mod compression;
mod fft;
mod filter;
mod fingerprint_calculator;
mod fingerprint_matcher;
mod fingerprinter;
mod gaussian;
mod gradient;
mod quantize;
mod rolling_image;
mod stages;
mod utils;
