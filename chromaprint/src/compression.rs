use crate::Configuration;

/// Number of "normal" bits.
const NORMAL_BITS: u8 = 3;
/// Maximum "normal" value above which a value becomes "exceptional".
const MAX_NORMAL_VALUE: u8 = (1 << NORMAL_BITS) - 1;

/// Turns an object (e.g. an `u32`) over an iterator of bits.
trait IntoBitIterator {
    /// Converts the item into an an iterator over its bits.
    fn into_bit_iter(self) -> impl Iterator<Item = bool>;
}

impl IntoBitIterator for u32 {
    fn into_bit_iter(self) -> impl Iterator<Item = bool> {
        (0..Self::BITS).map(move |index| ((self >> index) & 1) == 1)
    }
}

pub struct FingerprintCompressor<'a>(&'a Configuration);

impl<'a> FingerprintCompressor<'a> {
    /// Compress a sub-fingerprint.
    fn compress_subfingerprint(subfingerprint: u32) -> impl Iterator<Item = (u8, Option<u8>)> {
        subfingerprint
            .into_bit_iter()
            .enumerate()
            .filter_map(|(bit_index, is_bit_set)| {
                is_bit_set.then_some(u8::try_from(bit_index + 1).unwrap())
            })
            .scan(0, |last_bit_index, bit_index| {
                let value = bit_index - *last_bit_index;
                let result = if value >= MAX_NORMAL_VALUE {
                    (MAX_NORMAL_VALUE, Some(value - MAX_NORMAL_VALUE))
                } else {
                    (value, None)
                };

                *last_bit_index = bit_index;
                Some(result)
            })
            .chain(std::iter::once((0, None)))
    }

    /// Compress the fingerprint.
    pub fn compress(&self, fingerprint: &[u32]) -> Vec<u8> {
        let size = fingerprint.len();
        let (normal_bits, exceptional_bits) = fingerprint
            .iter()
            .scan(0, |last_subfp, current_subfp| {
                let value = current_subfp ^ *last_subfp;
                *last_subfp = *current_subfp;
                Some(value)
            })
            .flat_map(Self::compress_subfingerprint)
            .fold(
                (
                    Vec::<u8>::with_capacity(size),
                    Vec::<u8>::with_capacity(size),
                ),
                |(mut normal_bits, mut exceptional_bits), (normal_value, exceptional_value)| {
                    normal_bits.push(normal_value);
                    if let Some(exceptional_value) = exceptional_value {
                        exceptional_bits.push(exceptional_value);
                    }
                    (normal_bits, exceptional_bits)
                },
            );

        let header_size = 4;
        let normal_size = packed_intn_array_len(normal_bits.len(), 3);
        let exceptional_size = packed_intn_array_len(exceptional_bits.len(), 5);
        let expected_size = header_size + normal_size + exceptional_size;

        #[allow(clippy::cast_possible_truncation)]
        let output = [
            self.0.id(),
            ((size >> 16) & 0xFF) as u8,
            ((size >> 8) & 0xFF) as u8,
            (size & 0xFF) as u8,
        ];

        let output = output
            .into_iter()
            .chain(iter_packed_intn_array::<3>(&normal_bits))
            .chain(iter_packed_intn_array::<5>(&exceptional_bits))
            .collect::<Vec<u8>>();
        debug_assert_eq!(output.len(), expected_size);
        output
    }
}

impl<'a> From<&'a Configuration> for FingerprintCompressor<'a> {
    fn from(value: &'a Configuration) -> Self {
        Self(value)
    }
}

/// Calculate the size of a packed Int<N> array.
const fn packed_intn_array_len(array_len: usize, n: usize) -> usize {
    (array_len * n + 7) / 8
}

/// Iterate bytes as packed Int<N> array.
fn iter_packed_intn_array<const N: usize>(array: &[u8]) -> impl Iterator<Item = u8> + '_ {
    let mask = (0xFF << (8 - N)) >> (8 - N);
    array.chunks(8).flat_map(move |slice| {
        let (size, result) = slice.iter().map(|s| s & mask).enumerate().fold(
            (0, [0u8; N]),
            |(_, mut result), (i, bits)| {
                let rightmost_bit_index = i * N;
                let leftmost_bit_index = rightmost_bit_index + N - 1;

                let right_byte = rightmost_bit_index / 8;
                let left_byte = leftmost_bit_index / 8;

                result[right_byte] |= bits << (rightmost_bit_index % 8);
                if left_byte != right_byte {
                    result[left_byte] |= bits >> ((8 - (rightmost_bit_index % 8)) % 8);
                }

                (left_byte + 1, result)
            },
        );
        result.into_iter().take(size)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE_BYTE: [u8; 1] = [0b1011_1010];
    const NINE_BYTES: [u8; 9] = [
        0b1010_1010,
        0b0011_0011,
        0b1100_1100,
        0b1100_0111,
        0b0101_0101,
        0b1100_1100,
        0b1010_1010,
        0b0000_0000,
        0b1111_1111,
    ];
    const SIXTYFOUR_BYTES: [u8; 64] = [
        0xA2, 0x87, 0xE3, 0xED, 0xAA, 0xD7, 0xE8, 0x94, 0x53, 0x4E, 0x9B, 0xD5, 0x83, 0x12, 0x05,
        0x43, 0x67, 0x7E, 0x0A, 0xAF, 0x2D, 0x85, 0xB4, 0x03, 0xEB, 0x13, 0x8E, 0x47, 0x07, 0xA6,
        0x76, 0x5D, 0x43, 0x67, 0x8D, 0x9F, 0xEA, 0xAD, 0x3F, 0x34, 0x86, 0xF4, 0x25, 0xC8, 0xA2,
        0xBF, 0xF1, 0x22, 0xB5, 0xA6, 0xB8, 0x4A, 0xED, 0xA2, 0xF5, 0x25, 0xDB, 0x62, 0x70, 0xC2,
        0xB7, 0x9C, 0xB1, 0x3C,
    ];

    #[test]
    fn test_iter_packed_int3_array_single_byte() {
        const N: usize = 3;
        let packed = iter_packed_intn_array::<N>(&ONE_BYTE).collect::<Vec<u8>>();
        assert_eq!(packed.len(), packed_intn_array_len(ONE_BYTE.len(), N));
        assert_eq!(&packed, &[0b0000_0010]);
    }

    #[test]
    fn test_iter_packed_int3_array_some_bytes() {
        const N: usize = 3;
        let packed = iter_packed_intn_array::<N>(&NINE_BYTES).collect::<Vec<u8>>();
        assert_eq!(packed.len(), packed_intn_array_len(NINE_BYTES.len(), N));
        assert_eq!(
            &packed,
            &[0b0001_1010, 0b0101_1111, 0b0000_1010, 0b0000_0111]
        );
    }

    #[test]
    fn test_iter_packed_int3_array_many_bytes() {
        const N: usize = 3;
        let packed = iter_packed_intn_array::<N>(&SIXTYFOUR_BYTES).collect::<Vec<u8>>();
        assert_eq!(
            packed.len(),
            packed_intn_array_len(SIXTYFOUR_BYTES.len(), N)
        );
        assert_eq!(
            &packed,
            &[
                0xFA, 0xAA, 0x83, 0xF3, 0x3A, 0x75, 0xB7, 0xDE, 0x72, 0x9B, 0x7F, 0xBB, 0x7B, 0xAF,
                0x9E, 0x66, 0xA1, 0x47, 0x35, 0x54, 0xB5, 0x13, 0x74, 0x86
            ],
        );
    }

    #[test]
    fn test_iter_packed_int5_array_many_bytes() {
        const N: usize = 5;
        let packed = iter_packed_intn_array::<N>(&SIXTYFOUR_BYTES).collect::<Vec<u8>>();
        assert_eq!(
            packed.len(),
            packed_intn_array_len(SIXTYFOUR_BYTES.len(), N)
        );
        assert_eq!(
            &packed,
            &[
                0xE2, 0x8C, 0xA6, 0x2E, 0xA2, 0xD3, 0xED, 0x3A, 0x64, 0x19, 0xC7, 0xAB, 0xD7, 0x0A,
                0x1D, 0x6B, 0xBA, 0x73, 0x8C, 0xED, 0xE3, 0xB4, 0xAF, 0xDA, 0xA7, 0x86, 0x16, 0x24,
                0x7E, 0x14, 0xD5, 0x60, 0xD5, 0x44, 0x2D, 0x5B, 0x40, 0x71, 0x79, 0xE4,
            ],
        );
    }

    #[test]
    fn test_iter_packed_int5_array_single_byte() {
        const N: usize = 5;
        let packed = iter_packed_intn_array::<N>(&ONE_BYTE).collect::<Vec<u8>>();
        assert_eq!(packed.len(), packed_intn_array_len(ONE_BYTE.len(), N));
        assert_eq!(&packed, &[0b0001_1010]);
    }

    #[test]
    fn test_iter_packed_int5_array_some_bytes() {
        const N: usize = 5;
        let packed = iter_packed_intn_array::<N>(&NINE_BYTES).collect::<Vec<u8>>();
        assert_eq!(packed.len(), packed_intn_array_len(NINE_BYTES.len(), N));
        assert_eq!(
            &packed,
            &[
                0b0110_1010,
                0b1011_0010,
                0b0101_0011,
                0b1001_1001,
                0b0000_0010,
                0b0001_1111
            ]
        );
    }

    #[test]
    fn test_compression() {
        const INPUT: [u32; 32] = [
            0x0FCAF446, 0xE3519E89, 0xD3494DD6, 0x8F219806, 0x9200D530, 0x06B1D52F, 0xB48CC681,
            0x428991C3, 0x59AFBD6B, 0x6ECFB2E5, 0xE8EB7BC3, 0x99A44270, 0x31FFEC13, 0x4A4D81DA,
            0x53887C82, 0x2BB7BEC2, 0xAB895A65, 0x9D7C0AE4, 0xDA356857, 0xE030F7D8, 0x4D428EEE,
            0x0558E019, 0xC3278998, 0xA1D035E4, 0x582E98E5, 0x44C8B708, 0x2E8BA9E2, 0xCB13BC48,
            0xB169A3D8, 0x861274AF, 0x1213EF1C, 0x1F9F06B8,
        ];

        const OUTPUT: [u8; 220] = [
            0x01, 0x00, 0x00, 0x20, 0x0A, 0xA9, 0x24, 0xD2, 0x92, 0x24, 0x48, 0x92, 0x45, 0x52,
            0x14, 0x65, 0x8B, 0x12, 0x24, 0x49, 0xA4, 0x4C, 0x61, 0x1E, 0x54, 0x89, 0xA4, 0x50,
            0x61, 0x22, 0x28, 0xCA, 0x94, 0xA9, 0x53, 0x82, 0x24, 0xC9, 0x19, 0x4D, 0x83, 0x12,
            0x29, 0x19, 0x95, 0x84, 0x8B, 0xA0, 0x2A, 0x91, 0xA4, 0x47, 0x49, 0x40, 0x69, 0x11,
            0xB3, 0x45, 0x81, 0x12, 0x26, 0xC9, 0xA3, 0x44, 0x81, 0xB2, 0x6D, 0xD9, 0x98, 0x22,
            0x59, 0x94, 0x25, 0x4B, 0x32, 0x31, 0x41, 0xC2, 0x2C, 0x91, 0x12, 0x45, 0x95, 0x90,
            0x2D, 0x51, 0x94, 0x2D, 0x4A, 0x94, 0x04, 0x8C, 0xA4, 0x24, 0x49, 0xC4, 0x64, 0xC1,
            0xD7, 0x24, 0x49, 0xE2, 0x24, 0x48, 0x32, 0x6D, 0x89, 0x92, 0xE4, 0xC8, 0x2B, 0x49,
            0x49, 0x14, 0x05, 0xC9, 0x22, 0x31, 0xDA, 0x94, 0x10, 0x49, 0xC2, 0x24, 0xC9, 0xA2,
            0x2B, 0x81, 0xA2, 0x6C, 0x49, 0xB6, 0x44, 0x8A, 0x84, 0x24, 0x4A, 0xA2, 0x44, 0x99,
            0xF2, 0x21, 0xCF, 0x14, 0x25, 0x49, 0xB2, 0x30, 0x58, 0x92, 0x30, 0x89, 0x92, 0x28,
            0x89, 0x18, 0xE4, 0x8A, 0xA4, 0x24, 0x49, 0xB2, 0x24, 0x41, 0x14, 0x25, 0x49, 0x22,
            0x66, 0xC9, 0x12, 0x48, 0x4A, 0x94, 0x84, 0xE9, 0xA4, 0x40, 0x92, 0x22, 0x3D, 0x8B,
            0x96, 0xA0, 0x4B, 0x92, 0x54, 0x49, 0xA6, 0x24, 0x48, 0xA2, 0x44, 0x89, 0x94, 0x44,
            0x49, 0x94, 0x28, 0x48, 0x16, 0x25, 0xCA, 0x72, 0x0D, 0x9B, 0x32, 0x25, 0x0B, 0xA3,
            0x00, 0xA1, 0x80, 0x01, 0x06, 0x00, 0x00, 0x04, 0x30, 0x00,
        ];

        let config = Configuration::default();
        let compressor = FingerprintCompressor::from(&config);
        let output = compressor.compress(&INPUT);
        assert_eq!(output, OUTPUT);
    }
}
