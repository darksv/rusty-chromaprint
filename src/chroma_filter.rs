use crate::stages::{FeatureVectorConsumer, Stage};

pub struct ChromaFilter<C: FeatureVectorConsumer> {
    coefficients: Box<[f64]>,
    consumer: C,
    buffer: [[f64; 12]; 8],
    result: [f64; 12],
    buffer_offset: usize,
    buffer_size: usize,
}

impl<C: FeatureVectorConsumer> ChromaFilter<C> {
    pub(crate) fn new(coefficients: Box<[f64]>, consumer: C) -> Self {
        Self {
            coefficients,
            consumer,
            buffer: std::array::from_fn(|_| [0.0; 12]),
            result: [0.0; 12],
            buffer_offset: 0,
            buffer_size: 1,
        }
    }
}

impl<C: FeatureVectorConsumer> Stage for ChromaFilter<C> {
    type Output = C::Output;

    fn output(&self) -> &Self::Output {
        self.consumer.output()
    }
}

impl<C: FeatureVectorConsumer> FeatureVectorConsumer for ChromaFilter<C> {
    fn consume(&mut self, features: &[f64]) {
        self.buffer[self.buffer_offset].copy_from_slice(features);
        self.buffer_offset = (self.buffer_offset + 1) % self.buffer.len();
        if self.buffer_size >= self.coefficients.len() {
            let offset = (self.buffer_offset + self.buffer.len() - self.coefficients.len()) % self.buffer.len();
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

#[cfg(test)]
mod tests {
    use crate::assert_eq_float;
    use crate::chroma_filter::ChromaFilter;
    use crate::stages::{FeatureVectorConsumer, Stage};

    #[test]
    fn blur2() {
        let coefficients = [0.5, 0.5];
        let mut image = Image::new(12);
        let mut filter = ChromaFilter::new(coefficients.into(), &mut image);
        let d1 = [0.0, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let d2 = [1.0, 6.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let d3 = [2.0, 7.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        filter.consume(&d1);
        filter.consume(&d2);
        filter.consume(&d3);
        assert_eq!(2, image.rows());
        assert_eq!(0.5, image.get(0, 0));
        assert_eq!(1.5, image.get(1, 0));
        assert_eq!(5.5, image.get(0, 1));
        assert_eq!(6.5, image.get(1, 1));
    }

    #[test]
    fn blur3() {
        let coefficients = [0.5, 0.7, 0.5];
        let mut image = Image::new(12);
        let mut filter = ChromaFilter::new(coefficients.into(), &mut image);
        let d1 = [0.0, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let d2 = [1.0, 6.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let d3 = [2.0, 7.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let d4 = [3.0, 8.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        filter.consume(&d1);
        filter.consume(&d2);
        filter.consume(&d3);
        filter.consume(&d4);
        assert_eq!(2, image.rows());
        assert_eq_float!(1.7, image.get(0,0));
        assert_eq_float!(3.399999999999999,  image.get(1, 0));
        assert_eq_float!(10.199999999999999, image.get(0, 1));
        assert_eq_float!(11.899999999999999, image.get(1, 1));
    }

    #[test]
    fn diff() {
        let coefficients = [1.0, -1.0];
        let mut image = Image::new(12);
        let mut filter = ChromaFilter::new(coefficients.into(), &mut image);
        let d1 = [0.0, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let d2 = [1.0, 6.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let d3 = [2.0, 7.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        filter.consume(&d1);
        filter.consume(&d2);
        filter.consume(&d3);
        assert_eq!(2, image.rows());
        assert_eq!(-1.0, image.get(0, 0));
        assert_eq!(-1.0, image.get(1, 0));
        assert_eq!(-1.0, image.get(0, 1));
        assert_eq!(-1.0, image.get(1, 1));
    }

    struct Image {
        columns: usize,
        data: Vec<f64>,
    }

    impl Image {
        fn new(columns: usize) -> Self {
            Self {
                columns,
                data: vec![],
            }
        }

        fn rows(&self) -> usize {
            self.data.len() / self.columns
        }

        fn get(&self, row: usize, col: usize) -> f64 {
            self.data[row * self.columns + col]
        }
    }

    impl Stage for Image {
        type Output = [f64];

        fn output(&self) -> &Self::Output {
            self.data.as_slice()
        }
    }

    impl FeatureVectorConsumer for Image {
        fn consume(&mut self, features: &[f64]) {
            self.data.extend_from_slice(features);
        }

        fn reset(&mut self) {
            self.data.clear();
        }
    }
}