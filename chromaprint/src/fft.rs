use std::collections::VecDeque;
use std::sync::Arc;

use rustfft::num_complex::{Complex, Complex64};
use rustfft::num_traits::Zero;

use crate::stages::{AudioConsumer, FeatureVectorConsumer, Stage};

pub struct Fft<C: FeatureVectorConsumer> {
    consumer: C,
    frame_size: usize,
    frame_overlap: usize,

    fft_plan: Arc<dyn rustfft::Fft<f64>>,
    fft_buffer_complex: Box<[Complex64]>,
    fft_frame: Box<[f64]>,
    fft_scratch: Box<[Complex64]>,

    window: Box<[f64]>,
    ring_buf: VecDeque<f64>,
}

impl<C: FeatureVectorConsumer> Fft<C> {
    pub(crate) fn new(frame_size: usize, frame_overlap: usize, consumer: C) -> Self {
        let fft_plan = rustfft::FftPlanner::new().plan_fft_forward(frame_size);

        Self {
            consumer,
            frame_size,
            frame_overlap,
            fft_buffer_complex: vec![Complex64::zero(); frame_size].into_boxed_slice(),
            fft_scratch: vec![Complex::zero(); fft_plan.get_inplace_scratch_len()].into_boxed_slice(),
            fft_frame: vec![0.0; 1 + frame_size / 2].into_boxed_slice(),
            fft_plan,
            window: make_hamming_window(frame_size, 1.0),
            ring_buf: VecDeque::new(),
        }
    }
}

impl<C: FeatureVectorConsumer> Stage for Fft<C> {
    type Output = C::Output;

    fn output(&self) -> &Self::Output {
        self.consumer.output()
    }
}

impl<C: FeatureVectorConsumer> AudioConsumer<f64> for Fft<C> {
    fn reset(&mut self) {
        self.consumer.reset();
    }

    fn consume(&mut self, data: &[f64]) {
        self.ring_buf.extend(data.iter().copied());

        while self.ring_buf.len() >= self.frame_size {
            let window = self.ring_buf.iter().copied().take(self.frame_size);

            assert_eq!(self.fft_buffer_complex.len(), self.frame_size);
            assert_eq!(self.window.len(), self.frame_size);

            for (i, (output, input)) in self.fft_buffer_complex.iter_mut().zip(window).enumerate() {
                output.re = input * self.window[i];
                output.im = 0.0;
            }

            self.fft_plan.process_with_scratch(&mut self.fft_buffer_complex, &mut self.fft_scratch);

            for i in 0..self.frame_size / 2 {
                self.fft_frame[i] = self.fft_buffer_complex[i].norm_sqr();
            }

            self.consumer.consume(&self.fft_frame);
            self.ring_buf.drain(..self.frame_size - self.frame_overlap);
        }
    }

    fn flush(&mut self) {
        if self.ring_buf.is_empty() {
            return;
        }

        // It makes sense to pad the remaining samples with zeros and process the last frame,
        // but the reference implementation doesn't do it.
        // if self.ring_buf.len() < self.frame_size {
        //     self.ring_buf.resize(self.frame_size, 0.0);
        //     self.consume(&[]);
        // }
    }
}

fn make_hamming_window(size: usize, scale: f64) -> Box<[f64]> {
    let mut window = Vec::with_capacity(size);
    for i in 0..size {
        window.push(scale * (0.54 - 0.46 * f64::cos(2.0 * std::f64::consts::PI * (i as f64) / (size as f64 - 1.0))));
    }
    window.into_boxed_slice()
}

#[cfg(test)]
mod tests {
    use crate::fft::Fft;
    use crate::stages::{AudioConsumer, FeatureVectorConsumer, Stage};

    struct Collector {
        frames: Vec<Vec<f64>>,
    }

    impl Collector {
        fn new() -> Self {
            Self { frames: vec![] }
        }
    }

    impl Stage for Collector {
        type Output = [Vec<f64>];

        fn output(&self) -> &Self::Output {
            &self.frames
        }
    }

    impl FeatureVectorConsumer for Collector {
        fn consume(&mut self, features: &[f64]) {
            self.frames.push(features.to_vec());
        }

        fn reset(&mut self) {
            self.frames.clear();
        }
    }

    #[test]
    fn sine() {
        let nframes = 3;
        let frame_size = 32;
        let overlap = 8;

        let sample_rate = 1000;
        let freq = 7 * (sample_rate / 2) / (frame_size / 2);

        let mut input = vec![0.0; frame_size + (nframes - 1) * (frame_size - overlap)];
        for i in 0..input.len() {
            input[i] = f64::sin(i as f64 * freq as f64 * 2.0 * std::f64::consts::PI / sample_rate as f64);
        }

        let collector = Collector::new();
        let mut fft = Fft::new(frame_size, overlap, collector);

        assert_eq!(frame_size, fft.frame_size);
        assert_eq!(overlap, fft.frame_overlap);

        let chunk_size = 100;
        for chunk in input.chunks(chunk_size) {
            fft.consume(chunk);
        }

        assert_eq!(nframes, fft.output().len());

        let expected_spectrum = [
            2.87005e-05,
            0.00011901,
            0.00029869,
            0.000667172,
            0.00166813,
            0.00605612,
            0.228737,
            0.494486,
            0.210444,
            0.00385322,
            0.00194379,
            0.00124616,
            0.000903851,
            0.000715237,
            0.000605707,
            0.000551375,
            0.000534304,
        ];

        for (frame_idx, frame) in fft.output().iter().enumerate() {
            for i in 0..frame.len() {
                let magnitude = f64::sqrt(frame[i]) / frame.len() as f64;
                let expected_mag = expected_spectrum[i];
                if (expected_mag - magnitude).abs() > 0.001 {
                    panic!("different magnitude for frame {frame_idx} at offset {i}: s[{i}]={magnitude} (!= {expected_mag})");
                }
            }
        }
    }

    #[test]
    fn dc() {
        let nframes = 3;
        let frame_size = 32;
        let overlap = 8;

        let input = vec![0.5; frame_size + (nframes - 1) * (frame_size - overlap)];

        let collector = Collector::new();
        let mut fft = Fft::new(frame_size, overlap, collector);

        assert_eq!(frame_size, fft.frame_size);
        assert_eq!(overlap, fft.frame_overlap);

        let chunk_size = 100;
        for chunk in input.chunks(chunk_size) {
            fft.consume(chunk);
        }

        assert_eq!(nframes, fft.output().len());

        let expected_spectrum = [
            0.494691,
            0.219547,
            0.00488079,
            0.00178991,
            0.000939219,
            0.000576082,
            0.000385808,
            0.000272904,
            0.000199905,
            0.000149572,
            0.000112947,
            8.5041e-05,
            6.28312e-05,
            4.4391e-05,
            2.83757e-05,
            1.38507e-05,
            0.0,
        ];

        for (frame_idx, frame) in fft.output().iter().enumerate() {
            for i in 0..frame.len() {
                let magnitude = f64::sqrt(frame[i]) / frame.len() as f64;
                let expected_mag = expected_spectrum[i];
                if (expected_mag - magnitude).abs() > 0.001 {
                    panic!("different magnitude for frame {frame_idx} at offset {i}: s[{i}]={magnitude} (!= {expected_mag})");
                }
            }
        }
    }
}
