use crate::filter::{Filter, Image};
use crate::quantize::Quantizer;

pub struct Classifier {
    filter: Filter,
    quantizer: Quantizer,
}

impl Classifier {
    pub const fn new(filter: Filter, quantizer: Quantizer) -> Self {
        Self { filter, quantizer }
    }

    pub(crate) fn classify(&self, image: &impl Image, offset: usize) -> u32 {
        let value = self.filter.apply(image, offset);
        self.quantizer.quantize(value)
    }

    pub(crate) fn filter(&self) -> &Filter {
        &self.filter
    }
}
