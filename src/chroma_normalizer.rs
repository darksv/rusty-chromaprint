use crate::chroma::FeatureVectorConsumer;

pub struct ChromaNormalizer<C: FeatureVectorConsumer> {
    consumer: C,
}

impl<C: FeatureVectorConsumer> ChromaNormalizer<C> {
    pub(crate) fn new(consumer: C) -> Self {
        Self { consumer }
    }
}

impl<C: FeatureVectorConsumer> FeatureVectorConsumer for ChromaNormalizer<C> {
    fn consume(&mut self, features: &[f64]) {
        let norm = features.iter().fold(0.0, |acc, &x| acc + x.powi(2)).sqrt();

        let new = if norm < 0.01 {
            features.iter().map(|_| 0.0).collect::<Vec<f64>>()
        } else {
            features.iter().map(|&x| x / norm).collect::<Vec<f64>>()
        };

        self.consumer.consume(&new);
    }
}