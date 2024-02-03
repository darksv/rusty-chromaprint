/// Pack N least significant bits from each one value into a bitstream.
pub fn pack<const N: usize>(values: &[u32]) -> Vec<u8> {
    let mut buf = vec![];
    let mut writer = BitWriter::new(&mut buf);
    writer.buffer.reserve((values.len() * N + 7) / 8);
    for val in values {
        writer.write_bits::<N>(*val as u8);
    }
    writer.flush();
    buf
}

/// Unpack bitstream of N bit numbers into an array.
pub fn unpack<const N: usize>(bytes: &[u8]) -> Vec<u32> {
    let mut buf = vec![];
    let mut reader = BitReader::new(bytes);
    while let Some(bits) = reader.read_bits::<N>() {
        buf.push(bits as u32);
    }
    buf
}

/// Create a bitmask with `n` least significant bits set to `1`.
const fn mask_n_bits(n: usize) -> usize {
    (1 << n) - 1
}

struct BitWriter<'b> {
    buffer: &'b mut Vec<u8>,
    current_byte: u8,
    /// Number of bits written into `current_byte`.
    written_bits: usize,
}

impl<'b> BitWriter<'b> {
    fn new(buffer: &'b mut Vec<u8>) -> Self {
        Self {
            buffer,
            current_byte: 0,
            written_bits: 0,
        }
    }

    #[inline]
    fn write_bits<const BITS: usize>(&mut self, val: u8) {
        assert!(BITS <= 8);
        // Mask out bits we don't need.
        let val = val & mask_n_bits(BITS) as u8;
        if self.written_bits + BITS < 8 {
            // We have space for new bits in the current byte so just add them to it.
            self.current_byte <<= BITS;
            self.current_byte |= val;
            self.written_bits += BITS;
        } else if self.written_bits + BITS == 8 {
            // We have just enough space for new bits to make a single byte.
            self.current_byte <<= BITS;
            self.current_byte |= val;
            self.buffer.push(self.current_byte);
            self.current_byte = 0;
            self.written_bits = 0;
        } else {
            // We will overflow some bits...
            let overflowing_bits = (self.written_bits + BITS) - 8;
            // ... and create a new whole byte from previously saved bits and some of new bits.
            let fitting_bits = BITS - overflowing_bits;
            self.current_byte <<= fitting_bits;
            self.current_byte |= val >> overflowing_bits;
            self.buffer.push(self.current_byte);
            // Now we just save the remaining bits.
            self.current_byte = val & mask_n_bits(overflowing_bits) as u8;
            self.written_bits = overflowing_bits;
        }
    }

    fn flush(&mut self) {
        if self.written_bits != 0 {
            // Finish the current byte by adding some padding.
            self.buffer.push(self.current_byte << (8 - self.written_bits as u32));
            self.written_bits = 0;
            self.current_byte = 0;
        }
    }
}

struct BitReader<'b> {
    bytes: &'b [u8],
    current_byte: u8,
    remaining_bits: usize,
}

impl<'b> BitReader<'b> {
    fn new(bytes: &'b [u8]) -> Self {
        Self {
            bytes,
            current_byte: 0,
            remaining_bits: 0,
        }
    }

    #[inline]
    fn read_bits<const BITS: usize>(&mut self) -> Option<u8> {
        assert!(BITS > 0 && BITS <= 8);

        if self.remaining_bits >= BITS {
            // Just read bits from the current byte.
            let bits = (self.current_byte >> (8 - BITS)) & (mask_n_bits(BITS) as u8);
            self.current_byte <<= BITS;
            self.remaining_bits -= BITS;
            Some(bits)
        } else {
            // Try read next byte.
            let [next_byte, rest @ ..] = self.bytes else {
                return None;
            };
            self.bytes = rest;

            let bits_from_next_byte = BITS - self.remaining_bits;
            let remaining_bits_from_next_byte = 8 - bits_from_next_byte;
            let bits = (self.current_byte >> (8 - BITS)) | (next_byte >> remaining_bits_from_next_byte);
            self.current_byte = next_byte << bits_from_next_byte;
            self.remaining_bits = remaining_bits_from_next_byte;
            Some(bits)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{mask_n_bits, pack, unpack};

    fn packing_n<const N: usize>() {
        let values: Vec<_> = (0..1024 * 1024).collect();

        let packed = pack::<N>(&values);
        let unpacked = unpack::<N>(&packed);

        for (a, b) in values.iter().copied().zip(unpacked.iter().copied()) {
            assert_eq!(a & mask_n_bits(N) as u32, b);
        }
    }

    #[test]
    fn packing_3() {
        packing_n::<3>();
    }

    #[test]
    fn packing_5() {
        packing_n::<5>();
    }

    #[test]
    fn padding() {
        let vals = vec![0b11100000u8];
        let unpacked = unpack::<3>(&vals);
        assert_eq!(unpacked, &[7, 0]);
    }
}