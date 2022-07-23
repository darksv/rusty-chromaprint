//! Pure Rust port of [chromaprint](https://acoustid.org/chromaprint)

mod quantize;
mod filter;
mod rolling_image;
mod utils;
mod classifier;
mod fingerprint_calculator;
mod fingerprinter;
mod chroma;
mod audio_processor;
mod chroma_normalizer;
mod chroma_filter;
mod fft;
mod fingerprint_matcher;
mod gaussian;
mod gradient;

pub use fingerprinter::{Fingerprinter, Configuration};
pub use fingerprint_matcher::match_fingerprints;