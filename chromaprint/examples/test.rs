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
    let mut printer = Fingerprinter::new(&Configuration::preset_test1());
    printer.start(11025, 2).unwrap();
    printer.consume(&read_s16le("data/test_stereo_44100.raw"));
    printer.finish();

    assert_eq!(printer.fingerprint(), &[
        3086176501, 3077772469, 3077638581, 3052408789, 3048228821, 3046201301, 3042148311,
        3037102035, 2969993073, 3041294129, 3045483313, 3046514967, 3050712326, 3040164098,
        3040163847, 3073719559, 3073733965, 3212169693, 3212169693, 3220542455, 3220542399,
        3212152503, 3077933717, 3086327509, 3080034295, 4120237047, 4119197543, 4119295527,
        4123424293, 1975934501, 2110152245, 2111233559, 2144501255, 1005778439, 1001636359,
        1005683463, 1005682948, 1005686104, 991003132, 991031785, 995223531, 995190635,
        1003562858
    ]);
}
