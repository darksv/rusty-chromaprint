use crate::fingerprinter::Fingerprinter;
use crate::utils::read_s16le;

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


fn main() {
    let mut printer = Fingerprinter::new();
    printer.start(11025, 2).unwrap();
    printer.consume(&read_s16le("data/test_stereo_44100.raw"));
    printer.finish();

    println!("{:?}", printer.fingerprint());
}
