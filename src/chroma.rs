use std::cell::RefCell;
use std::rc::Rc;

pub(crate) struct Chroma<C: FeatureVectorConsumer> {
    interpolate: bool,
    notes: Vec<u8>,
    notes_frac: Vec<f64>,
    min_index: u32,
    max_index: u32,
    features: Vec<f64>,
    consumer: C,
}

const NUM_BANDS: usize = 12;

impl<C: FeatureVectorConsumer> Chroma<C> {
    pub(crate) fn new(min_freq: u32, max_freq: u32, frame_size: usize, sample_rate: usize, consumer: C) -> Self {
        let mut chroma = Self {
            interpolate: false,
            notes: vec![0; frame_size],
            notes_frac: vec![0.0; frame_size],
            min_index: 0,
            max_index: 0,
            features: vec![0.0; NUM_BANDS],
            consumer,
        };
        chroma.prepare_notes(min_freq, max_freq, frame_size, sample_rate);
        chroma
    }

    fn prepare_notes(&mut self, min_freq: u32, max_freq: u32, frame_size: usize, sample_rate: usize) {
        self.min_index = freq_to_index(min_freq, frame_size, sample_rate).max(1);
        self.max_index = freq_to_index(max_freq, frame_size, sample_rate).min(frame_size as u32 / 2);

        for i in self.min_index..self.max_index {
            let freq = index_to_freq(i, frame_size, sample_rate);
            let octave = freq_to_octave(freq);

            let note = NUM_BANDS as f64 * (octave - octave.floor());
            self.notes[i as usize] = note.floor() as u8;
            self.notes_frac[i as usize] = note - note.floor();
        }
    }

    fn into_consumer(self) -> C {
        self.consumer
    }
}

impl<C: FeatureVectorConsumer> FeatureVectorConsumer for Chroma<C> {
    fn consume(&mut self, frame: &[f64]) {
        self.features.fill(0.0);
        for i in self.min_index..self.max_index {
            let note = self.notes[i as usize] as usize;
            let energy = frame[i as usize];
            if self.interpolate {
                let mut note2 = note;
                let mut a = 1.0;
                if self.notes_frac[i as usize] < 0.5 {
                    note2 = (note + NUM_BANDS - 1) % NUM_BANDS;
                    a = 0.5 + self.notes_frac[i as usize];
                }
                if self.notes_frac[i as usize] > 0.5 {
                    note2 = (note + 1) % NUM_BANDS;
                    a = 1.5 - self.notes_frac[i as usize];
                }
                self.features[note] += energy * a;
                self.features[note2] += energy * (1.0 - a);
            } else {
                self.features[note as usize] += energy;
            }
        }

        self.consumer.consume(&self.features);
    }
}

fn freq_to_index(freq: u32, frame_size: usize, sample_rate: usize) -> u32 {
    (freq as f64 * frame_size as f64 / sample_rate as f64).floor() as u32
}

fn index_to_freq(i: u32, frame_size: usize, sample_rate: usize) -> f64 {
    return (i as f64) * sample_rate as f64 / frame_size as f64;
}

fn freq_to_octave(freq: f64) -> f64 {
    let base = 440.0 / 16.0;
    return f64::log2(freq / base);
}

pub trait FeatureVectorConsumer {
    fn consume(&mut self, features: &[f64]);
    fn reset(&mut self) {}
}

impl<C: FeatureVectorConsumer + ?Sized> FeatureVectorConsumer for Rc<RefCell<C>> {
    fn consume(&mut self, features: &[f64]) {
        (**self).borrow_mut().consume(features);
    }
    fn reset(&mut self) {
        (**self).borrow_mut().reset();
    }
}

impl<C: FeatureVectorConsumer + ?Sized> FeatureVectorConsumer for &mut C {
    fn consume(&mut self, features: &[f64]) {
        (**self).consume(features);
    }
    fn reset(&mut self) {
        (**self).reset();
    }
}

struct FeatureVectorBuffer {
    features: Vec<f64>,
}

impl FeatureVectorBuffer {
    fn new() -> Self {
        Self {
            features: vec![],
        }
    }
}

impl FeatureVectorConsumer for FeatureVectorBuffer {
    fn consume(&mut self, features: &[f64]) {
        self.features.clear();
        self.features.extend_from_slice(features);
    }
}


#[cfg(test)]
mod tests {
    use crate::assert_eq_float;
    use crate::chroma::{Chroma, FeatureVectorBuffer, FeatureVectorConsumer};

    #[test]
    fn normal_a() {
        let mut chroma = Chroma::new(10, 510, 256, 1000, FeatureVectorBuffer::new());
        let mut frame = vec![0.0; 128];
        frame[113] = 1.0;
        chroma.consume(&frame);
        let buffer = chroma.into_consumer();

        assert_eq!(12, buffer.features.len());
        let expected_features = [
            1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        ];

        for i in 0..12 {
            assert_eq_float!(expected_features[i], buffer.features[i], 0.0001);
        }
    }

    #[test]
    fn normal_gsharp() {
        let mut chroma = Chroma::new(10, 510, 256, 1000, FeatureVectorBuffer::new());
        let mut frame = vec![0.0; 128];
        frame[112] = 1.0;
        chroma.consume(&frame);
        let buffer = chroma.into_consumer();

        assert_eq!(12, buffer.features.len());
        let expected_features = [
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];

        for i in 0..12 {
            assert_eq_float!(expected_features[i], buffer.features[i], 0.0001);
        }
    }

    #[test]
    fn normal_b() {
        let mut chroma = Chroma::new(10, 510, 256, 1000, FeatureVectorBuffer::new());
        let mut frame = vec![0.0; 128];
        frame[64] = 1.0;
        chroma.consume(&frame);
        let buffer = chroma.into_consumer();

        assert_eq!(12, buffer.features.len());
        let expected_features = [
            0.0, 0.0, 1.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        ];

        for i in 0..12 {
            assert_eq_float!(expected_features[i], buffer.features[i], 0.0001);
        }
    }

    #[test]
    fn interpolated_a() {
        let mut frame = vec![0.0; 128];
        frame[113] = 1.0;

        let mut chroma = Chroma::new(10, 510, 256, 1000, FeatureVectorBuffer::new());
        chroma.interpolate = true;
        chroma.consume(&frame);
        let buffer = chroma.into_consumer();

        assert_eq!(12, buffer.features.len());
        let expected_features = [
            0.555242, 0.0, 0.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 0.0, 0.0, 0.444758,
        ];

        for i in 0..12 {
            assert_eq_float!(expected_features[i], buffer.features[i], 0.0001);
        }
    }

    #[test]
    fn interpolated_gsharp() {
        let mut frame = vec![0.0; 128];
        frame[112] = 1.0;
        let mut chroma = Chroma::new(10, 510, 256, 1000, FeatureVectorBuffer::new());
        chroma.interpolate = true;
        chroma.consume(&frame);
        let buffer = chroma.into_consumer();

        assert_eq!(12, buffer.features.len());
        let expected_features = [
            0.401354, 0.0, 0.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 0.0, 0.0, 0.598646,
        ];

        for i in 0..12 {
            assert_eq_float!(expected_features[i], buffer.features[i], 0.0001);
        }
    }

    #[test]
    fn interpolated_b() {
        let mut frame = vec![0.0; 128];
        frame[64] = 1.0;
        let mut chroma = Chroma::new(10, 510, 256, 1000, FeatureVectorBuffer::new());
        chroma.interpolate = true;
        chroma.consume(&frame);
        let buffer = chroma.into_consumer();

        assert_eq!(12, buffer.features.len());
        let expected_features = [
            0.0, 0.286905, 0.713095, 0.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        ];

        for i in 0..12 {
            assert_eq_float!(expected_features[i], buffer.features[i], 0.0001);
        }
    }
}