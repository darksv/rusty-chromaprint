use std::collections::VecDeque;
use std::sync::Arc;

use rustfft::num_complex::{Complex, Complex64};
use rustfft::num_traits::Zero;

use crate::audio_processor::AudioConsumer;
use crate::chroma::FeatureVectorConsumer;

pub struct Fft<C: FeatureVectorConsumer> {
    consumer: C,
    frame_size: usize,
    frame_overlap: usize,

    fft_plan: Arc<dyn rustfft::Fft<f64>>,
    fft_buffer_complex: Box<[Complex64]>,
    fft_frame: Box<[f64]>,
    fft_scratch: Box<[Complex64]>,

    window: Box<[f64]>,
    ring_buf: VecDeque<i16>,
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
            window: make_hamming_window(frame_size, 1.0 / f64::from(i16::MAX)),
            ring_buf: VecDeque::new(),
        }
    }
}

impl<C: FeatureVectorConsumer> AudioConsumer for Fft<C> {
    fn reset(&mut self) {
        self.consumer.reset();
    }

    fn consume(&mut self, data: &[i16]) {
        self.ring_buf.extend(data.iter().copied());

        while self.ring_buf.len() >= self.frame_size {
            let window = self.ring_buf.iter().copied().take(self.frame_size);

            assert_eq!(self.fft_buffer_complex.len(), self.frame_size);
            assert_eq!(self.window.len(), self.frame_size);

            for (i, (output, input))
            in self.fft_buffer_complex.iter_mut().zip(window).enumerate() {
                output.re = f64::from(input) * self.window[i];
                output.im = 0.0;
            }

            self.fft_plan.process_with_scratch(&mut self.fft_buffer_complex, &mut self.fft_scratch);

            self.fft_frame[0] = self.fft_buffer_complex[0].re.powi(2);
            self.fft_frame[self.frame_size / 2] = self.fft_buffer_complex[0].im.powi(2);
            for i in 1..self.frame_size / 2 {
                self.fft_frame[i] = self.fft_buffer_complex[i].norm_sqr();
            }

            self.consumer.consume(&self.fft_frame);
            self.ring_buf.drain(..self.frame_size - self.frame_overlap);
        }
    }
}

fn make_hamming_window(size: usize, scale: f64) -> Box<[f64]> {
    let mut window = Vec::with_capacity(size);
    for i in 0..size {
        window.push(scale * (0.54 - 0.46 * f64::cos(2.0 * std::f64::consts::PI * (i as f64) / (size as f64 - 1.0))));
    }
    window.into_boxed_slice()
}