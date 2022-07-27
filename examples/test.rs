use std::path::Path;

use rusty_chromaprint::{Configuration, Fingerprinter};

fn read_s16le(path: impl AsRef<Path>) -> Vec<i16> {
    std::fs::read(path)
        .unwrap()
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
        .collect::<Vec<_>>()
}

fn main() {
    let mut printer = Fingerprinter::new(Configuration::preset_test2());
    printer.start(11025, 2).unwrap();
    printer.consume(&read_s16le("data/test_stereo_44100.raw"));
    printer.finish();

    println!("{:04X?}", printer.fingerprint());
}
