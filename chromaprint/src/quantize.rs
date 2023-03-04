#[derive(Clone, Copy)]
pub struct Quantizer {
    t0: f64,
    t1: f64,
    t2: f64,
}

impl Quantizer {
    pub const fn new(t0: f64, t1: f64, t2: f64) -> Self {
        // assert!(t0 <= t1 && t1 <= t2);
        Self { t0, t1, t2 }
    }

    pub fn quantize(&self, val: f64) -> u32 {
        if val < self.t1 {
            if val < self.t0 {
                0
            } else {
                1
            }
        } else {
            if val < self.t2 {
                2
            } else {
                3
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::quantize::Quantizer;

    #[test]
    fn quantization() {
        let q = Quantizer::new(0.0, 0.1, 0.3);
        assert_eq!(0, q.quantize(-0.1));
        assert_eq!(1, q.quantize(0.0));
        assert_eq!(1, q.quantize(0.03));
        assert_eq!(2, q.quantize(0.1));
        assert_eq!(2, q.quantize(0.13));
        assert_eq!(3, q.quantize(0.3));
        assert_eq!(3, q.quantize(0.33));
        assert_eq!(3, q.quantize(1000.0));
    }
}
