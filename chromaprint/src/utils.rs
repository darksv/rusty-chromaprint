use std::path::Path;

#[doc(hidden)]
#[macro_export]
macro_rules! assert_eq_float {
    ($a:expr, $b:expr) => {
        assert_eq_float!($a, $b, 0.00001);
    };
    ($a:expr, $b:expr, $eps:expr) => {
        assert!(($a - $b).abs() < $eps);
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! assert_eq_float_slice {
    ($a:expr, $b:expr) => {
        $crate::assert_eq_float_slice!($a, $b, 0.00001);
    };
    ($a:expr, $b:expr, $eps:expr) => {
        let a = $a;
        let b = $b;

        assert_eq!(a.len(), b.len());
        for (a, b) in std::iter::zip(a, b) {
            $crate::assert_eq_float!(a, b, $eps);
        }
    };
}

#[allow(unused)]
pub(crate) fn read_s16le(path: impl AsRef<Path>) -> Vec<i16> {
    std::fs::read(path)
        .unwrap()
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
        .collect::<Vec<_>>()
}
