use std::path::Path;

macro_rules! assert_eq_float {
    ($a:expr, $b:expr) => {
        assert_eq_float!($a, $b, 0.00001);
    };
    ($a:expr, $b:expr, $eps:expr) => {
        assert!(($a - $b).abs() < $eps);
    };
}


pub(crate) fn read_s16le(path: impl AsRef<Path>) -> Vec<i16> {
    std::fs::read(path)
        .unwrap()
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
        .collect::<Vec<_>>()
}