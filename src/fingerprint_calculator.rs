use crate::classifier::Classifier;
use crate::stages::{FeatureVectorConsumer, Stage};
use crate::rolling_image::RollingIntegralImage;

pub struct FingerprintCalculator {
    classifiers: Vec<Classifier>,
    max_filter_width: usize,
    image: RollingIntegralImage,
    fingerprint: Vec<u32>,
}

impl FingerprintCalculator {
    pub(crate) fn new(classifiers: Vec<Classifier>) -> Self {
        let max_width = classifiers.iter().map(|c| c.filter().width()).max().unwrap();
        assert!(max_width > 0);
        assert!(max_width <= 256);

        Self {
            max_filter_width: max_width,
            classifiers,
            image: RollingIntegralImage::new(255),
            fingerprint: vec![],
        }
    }

    fn calculate_subfingerprint(&self, offset: usize) -> u32 {
        let mut bits = 0u32;
        for classifier in &self.classifiers {
            bits = (bits << 2) | gray_code(classifier.classify(&self.image, offset));
        }
        return bits;
    }

    pub(crate) fn fingerprint(&self) -> &[u32] {
        &self.fingerprint
    }

    fn clear_fingerprint(&mut self) {
        self.fingerprint.clear()
    }
}

impl Stage for FingerprintCalculator {
    type Output = [u32];

    fn output(&self) -> &Self::Output {
        self.fingerprint.as_slice()
    }
}

impl FeatureVectorConsumer for FingerprintCalculator {
    fn consume(&mut self, features: &[f64]) {
        self.image.add_row(features);
        if self.image.rows() >= self.max_filter_width {
            self.fingerprint.push(self.calculate_subfingerprint(self.image.rows() - self.max_filter_width));
        }
    }

    fn reset(&mut self) {
        self.image.reset();
        self.fingerprint.clear();
    }
}

fn gray_code(i: u32) -> u32 {
    [0, 1, 3, 2][i as usize]
}

