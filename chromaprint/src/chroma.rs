use crate::stages::{FeatureVectorConsumer, Stage};

pub(crate) struct Chroma<C: FeatureVectorConsumer> {
    interpolate: bool,
    notes: Box<[u8]>,
    notes_frac: Box<[f64]>,
    min_index: usize,
    max_index: usize,
    features: [f64; NUM_BANDS],
    consumer: C,
}

const NUM_BANDS: usize = 12;

impl<C: FeatureVectorConsumer> Chroma<C> {
    pub(crate) fn new(min_freq: u32, max_freq: u32, frame_size: usize, sample_rate: u32, consumer: C) -> Self {
        let mut chroma = Self {
            interpolate: false,
            notes: vec![0; frame_size].into_boxed_slice(),
            notes_frac: vec![0.0; frame_size].into_boxed_slice(),
            min_index: 0,
            max_index: 0,
            features: [0.0; NUM_BANDS],
            consumer,
        };
        chroma.prepare_notes(min_freq, max_freq, frame_size, sample_rate);
        chroma
    }

    fn prepare_notes(&mut self, min_freq: u32, max_freq: u32, frame_size: usize, sample_rate: u32) {
        self.min_index = freq_to_index(min_freq, frame_size, sample_rate).max(1);
        self.max_index = freq_to_index(max_freq, frame_size, sample_rate).min(frame_size / 2);
        for i in self.min_index..self.max_index {
            let freq = index_to_freq(i, frame_size, sample_rate);
            let octave = freq_to_octave(freq);
            let note = NUM_BANDS as f64 * (octave - octave.floor());
            self.notes[i] = note.floor() as u8;
            self.notes_frac[i] = note - note.floor();
        }
    }
}

impl<C: FeatureVectorConsumer> Stage for Chroma<C> {
    type Output = C::Output;

    fn output(&self) -> &Self::Output {
        self.consumer.output()
    }
}

impl<C: FeatureVectorConsumer> FeatureVectorConsumer for Chroma<C> {
    fn consume(&mut self, frame: &[f64]) {
        self.features.fill(0.0);
        for (i, energy) in frame.iter().enumerate().take(self.max_index).skip(self.min_index) {
            let note = self.notes[i] as usize;
            if self.interpolate {
                let mut note2 = note;
                let mut a = 1.0;
                if self.notes_frac[i] < 0.5 {
                    note2 = (note + NUM_BANDS - 1) % NUM_BANDS;
                    a = 0.5 + self.notes_frac[i];
                }
                if self.notes_frac[i] > 0.5 {
                    note2 = (note + 1) % NUM_BANDS;
                    a = 1.5 - self.notes_frac[i];
                }
                self.features[note] += energy * a;
                self.features[note2] += energy * (1.0 - a);
            } else {
                self.features[note] += energy;
            }
        }

        self.consumer.consume(&self.features);
    }

    fn reset(&mut self) {
        self.consumer.reset();
    }
}

fn freq_to_index(freq: u32, frame_size: usize, sample_rate: u32) -> usize {
    (frame_size as f64 * freq as f64 / sample_rate as f64).round() as usize
}

fn index_to_freq(i: usize, frame_size: usize, sample_rate: u32) -> f64 {
    (i as f64) * sample_rate as f64 / frame_size as f64
}

fn freq_to_octave(freq: f64) -> f64 {
    let base = 440.0 / 16.0;
    f64::log2(freq / base)
}

#[cfg(test)]
mod tests {
    use crate::assert_eq_float;
    use crate::chroma::{Chroma, FeatureVectorConsumer};
    use crate::stages::Stage;

    #[test]
    fn normal_a() {
        let mut chroma = Chroma::new(10, 510, 256, 1000, FeatureVectorBuffer::new());
        let mut frame = vec![0.0; 128];
        frame[113] = 1.0;
        chroma.consume(&frame);
        let features = chroma.output();

        assert_eq!(12, features.len());
        let expected_features = [
            1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        ];

        for i in 0..12 {
            assert_eq_float!(expected_features[i], features[i], 0.0001);
        }
    }

    #[test]
    fn normal_gsharp() {
        let mut chroma = Chroma::new(10, 510, 256, 1000, FeatureVectorBuffer::new());
        let mut frame = vec![0.0; 128];
        frame[112] = 1.0;
        chroma.consume(&frame);
        let features = chroma.output();

        assert_eq!(12, features.len());
        let expected_features = [
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];

        for i in 0..12 {
            assert_eq_float!(expected_features[i], features[i], 0.0001);
        }
    }

    #[test]
    fn normal_b() {
        let mut chroma = Chroma::new(10, 510, 256, 1000, FeatureVectorBuffer::new());
        let mut frame = vec![0.0; 128];
        frame[64] = 1.0;
        chroma.consume(&frame);
        let features = chroma.output();

        assert_eq!(12, features.len());
        let expected_features = [
            0.0, 0.0, 1.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        ];

        for i in 0..12 {
            assert_eq_float!(expected_features[i], features[i], 0.0001);
        }
    }

    #[test]
    fn interpolated_a() {
        let mut frame = vec![0.0; 128];
        frame[113] = 1.0;

        let mut chroma = Chroma::new(10, 510, 256, 1000, FeatureVectorBuffer::new());
        chroma.interpolate = true;
        chroma.consume(&frame);
        let features = chroma.output();

        assert_eq!(12, features.len());
        let expected_features = [
            0.555242, 0.0, 0.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 0.0, 0.0, 0.444758,
        ];

        for i in 0..12 {
            assert_eq_float!(expected_features[i], features[i], 0.0001);
        }
    }

    #[test]
    fn interpolated_gsharp() {
        let mut frame = vec![0.0; 128];
        frame[112] = 1.0;
        let mut chroma = Chroma::new(10, 510, 256, 1000, FeatureVectorBuffer::new());
        chroma.interpolate = true;
        chroma.consume(&frame);
        let features = chroma.output();

        assert_eq!(12, features.len());
        let expected_features = [
            0.401354, 0.0, 0.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 0.0, 0.0, 0.598646,
        ];

        for i in 0..12 {
            assert_eq_float!(expected_features[i], features[i], 0.0001);
        }
    }

    #[test]
    fn interpolated_b() {
        let mut frame = vec![0.0; 128];
        frame[64] = 1.0;
        let mut chroma = Chroma::new(10, 510, 256, 1000, FeatureVectorBuffer::new());
        chroma.interpolate = true;
        chroma.consume(&frame);
        let features = chroma.output();

        assert_eq!(12, features.len());
        let expected_features = [
            0.0, 0.286905, 0.713095, 0.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        ];

        for i in 0..12 {
            assert_eq_float!(expected_features[i], features[i], 0.0001);
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

    impl Stage for FeatureVectorBuffer {
        type Output = [f64];

        fn output(&self) -> &Self::Output {
            self.features.as_slice()
        }
    }

    impl FeatureVectorConsumer for FeatureVectorBuffer {
        fn consume(&mut self, features: &[f64]) {
            self.features.clear();
            self.features.extend_from_slice(features);
        }

        fn reset(&mut self) {
            self.features.clear();
        }
    }
}