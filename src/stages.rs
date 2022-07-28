pub trait Stage {
    type Output: ?Sized;
    fn output(&self) -> &Self::Output;
}

impl<C: Stage> Stage for &mut C {
    type Output = C::Output;

    fn output(&self) -> &Self::Output {
        (**self).output()
    }
}

pub trait AudioConsumer: Stage {
    fn reset(&mut self);
    fn consume(&mut self, data: &[i16]);
}

impl<S: Stage + ?Sized> Stage for Box<S> {
    type Output = S::Output;

    fn output(&self) -> &Self::Output {
        (**self).output()
    }
}

impl<C: AudioConsumer + ?Sized> AudioConsumer for Box<C> {
    fn reset(&mut self) {
        (**self).reset();
    }

    fn consume(&mut self, data: &[i16]) {
        (**self).consume(data);
    }
}

pub trait FeatureVectorConsumer: Stage {
    fn consume(&mut self, features: &[f64]);
    fn reset(&mut self) {}
}

impl<C: FeatureVectorConsumer> FeatureVectorConsumer for &mut C {
    fn consume(&mut self, features: &[f64]) {
        (**self).consume(features);
    }
    fn reset(&mut self) {
        (**self).reset();
    }
}