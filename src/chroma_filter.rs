use crate::chroma::FeatureVectorConsumer;

pub struct ChromaFilter<C: FeatureVectorConsumer> {
    coefficients: Vec<f64>,
    consumer: C,
    buffer: [[f64; 12]; 8],
    result: [f64; 12],
    buffer_offset: usize,
    buffer_size: usize,
}

impl<C: FeatureVectorConsumer> ChromaFilter<C> {
    pub(crate) fn new(coefficients: Vec<f64>, consumer: C) -> Self {
        Self {
            coefficients: coefficients.to_vec(),
            consumer,
            buffer: std::array::from_fn(|_| [0.0; 12]),
            result: [0.0; 12],
            buffer_offset: 0,
            buffer_size: 1,
        }
    }
}

impl<C: FeatureVectorConsumer> FeatureVectorConsumer for ChromaFilter<C> {
    fn consume(&mut self, features: &[f64]) {
        self.buffer[self.buffer_offset].copy_from_slice(features);
        self.buffer_offset = (self.buffer_offset + 1) % 8;
        if self.buffer_size >= self.coefficients.len() {
            let offset = (self.buffer_offset + 8 - self.coefficients.len()) % 8;
            self.result.fill(0.0);
            for i in 0..self.result.len() {
                for j in 0..self.coefficients.len() {
                    self.result[i] += self.buffer[(offset + j) % self.buffer.len()][i] * self.coefficients[j];
                }
            }

            self.consumer.consume(&self.result);
        } else {
            self.buffer_size += 1;
        }
    }

    fn reset(&mut self) {
        self.buffer_size = 1;
        self.buffer_offset = 0;
    }
}