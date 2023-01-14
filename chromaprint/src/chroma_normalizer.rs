use crate::stages::{FeatureVectorConsumer, Stage};

pub struct ChromaNormalizer<C: FeatureVectorConsumer> {
    consumer: C,
}

impl<C: FeatureVectorConsumer> ChromaNormalizer<C> {
    pub(crate) fn new(consumer: C) -> Self {
        Self { consumer }
    }
}

impl<C: FeatureVectorConsumer> Stage for ChromaNormalizer<C> {
    type Output = C::Output;

    fn output(&self) -> &Self::Output {
        self.consumer.output()
    }
}

impl<C: FeatureVectorConsumer> FeatureVectorConsumer for ChromaNormalizer<C> {
    fn consume(&mut self, features: &[f64]) {
        let mut features = features.to_vec();
        normalize(&mut features, 0.01);
        self.consumer.consume(&features);
    }

    fn reset(&mut self) {
        self.consumer.reset();
    }
}

fn normalize(values: &mut [f64], eps: f64) {
    let norm = values.iter().fold(0.0, |acc, &x| acc + x.powi(2)).sqrt();
    if norm < eps {
        values.fill(0.0);
    } else {
        for x in values {
            *x /= norm;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::assert_eq_float;
    use crate::chroma_normalizer::normalize;

    #[test]
    fn normalize_vector() {
        let data = [0.1, 0.2, 0.4, 1.0];
        let normalized = [0.090909, 0.181818, 0.363636, 0.909091];
        let mut normalized_data = data;
        normalize(&mut normalized_data, 0.01);

        for i in 0..4 {
            assert_eq_float!(normalized_data[i], normalized[i], 1e-5);
        }
    }

    #[test]
    fn normalize_vector_near_zero() {
        let data = [0.0, 0.001, 0.002, 0.003];
        let mut normalized_data = data;
        normalize(&mut normalized_data, 0.01);

        for i in 0..4 {
            assert_eq_float!(normalized_data[i], 0.0, 1e-5);
        }
    }

    #[test]
    fn normalize_vector_zero() {
        let data = [0.0, 0.0, 0.0, 0.0];
        let mut normalized_data = data;
        normalize(&mut normalized_data, 0.01);

        for i in 0..4 {
            assert_eq_float!(normalized_data[i], 0.0, 1e-5);
        }
    }
}