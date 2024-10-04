use rustfft::num_complex::Complex;
use crate::audio_processor::{AudioProcessor, ResetError};
use crate::chroma::Chroma;
use crate::chroma_filter::ChromaFilter;
use crate::chroma_normalizer::ChromaNormalizer;
use crate::classifier::Classifier;
use crate::stages::{AudioConsumer, Stage};
use crate::fft::Fft;
use crate::filter::{Filter, FilterKind};
use crate::fingerprint_calculator::FingerprintCalculator;
use crate::quantize::Quantizer;

/// Structure containing configuration for a [Fingerprinter].
#[derive(Debug, Clone)]
pub struct Configuration {
    classifiers: Vec<Classifier>,
    remove_silence: bool,
    silence_threshold: u32,
    frame_size: usize,
    frame_overlap: usize,
    filter_coefficients: Vec<f64>,
    max_filter_width: usize,
    interpolate: bool,
}

impl Configuration {
    /// Creates a new default configuration.
    fn new() -> Self {
        Self {
            classifiers: Vec::new(),
            remove_silence: false,
            silence_threshold: 0,
            frame_size: 0,
            frame_overlap: 0,
            filter_coefficients: Vec::new(),
            max_filter_width: 0,
            interpolate: false,
        }
    }

    /// Adds classifiers to the configuration.
    pub fn with_classifiers(mut self, classifiers: Vec<Classifier>) -> Self {
        self.max_filter_width = classifiers.iter()
            .map(|c| c.filter().width())
            .max()
            .unwrap_or(0);
        self.classifiers = classifiers;
        self
    }

    /// Updates coefficients for internal chroma filter.
    pub fn with_coefficients(mut self, coefficients: Vec<f64>) -> Self {
        self.filter_coefficients = coefficients;
        self
    }

    /// Enables or disables interpolation.
    pub fn with_interpolation(mut self, interpolate: bool) -> Self {
        self.interpolate = interpolate;
        self
    }

    /// Sets number of samples in a single frame for FFT.
    pub fn with_frame_size(mut self, frame_size: usize) -> Self {
        self.frame_size = frame_size;
        self
    }

    /// Sets number of samples overlapping between two consecutive frames for FFT.
    pub fn with_frame_overlap(mut self, frame_overlap: usize) -> Self {
        self.frame_overlap = frame_overlap;
        self
    }

    /// Enables removal of silence with a specified threshold.
    pub fn with_removed_silence(mut self, silence_threshold: u32) -> Self {
        self.remove_silence = true;
        self.silence_threshold = silence_threshold;
        self
    }

    /// Target sample rate for fingerprint calculation.
    pub fn sample_rate(&self) -> u32 {
        DEFAULT_SAMPLE_RATE
    }

    pub fn preset_test1() -> Self {
        Self::new()
            .with_classifiers(CLASSIFIER_TEST1.into())
            .with_coefficients(CHROMA_FILTER_COEFFICIENTS.into())
            .with_interpolation(false)
            .with_frame_size(DEFAULT_FRAME_SIZE)
            .with_frame_overlap(DEFAULT_FRAME_OVERLAP)
    }

    pub fn preset_test2() -> Self {
        Self::new()
            .with_classifiers(CLASSIFIER_TEST2.into())
            .with_coefficients(CHROMA_FILTER_COEFFICIENTS.into())
            .with_interpolation(false)
            .with_frame_size(DEFAULT_FRAME_SIZE)
            .with_frame_overlap(DEFAULT_FRAME_OVERLAP)
    }

    pub fn preset_test3() -> Self {
        Self::new()
            .with_classifiers(CLASSIFIER_TEST3.into())
            .with_coefficients(CHROMA_FILTER_COEFFICIENTS.into())
            .with_interpolation(true)
            .with_frame_size(DEFAULT_FRAME_SIZE)
            .with_frame_overlap(DEFAULT_FRAME_OVERLAP)
    }

    pub fn preset_test4() -> Self {
        Self::new()
            .with_removed_silence(50)
    }

    pub fn preset_test5() -> Self {
        Self::new()
            .with_frame_size(DEFAULT_FRAME_SIZE / 2)
            .with_frame_overlap(DEFAULT_FRAME_SIZE / 2 - DEFAULT_FRAME_SIZE / 4)
    }

    fn samples_in_item(&self) -> usize {
        self.frame_size - self.frame_overlap
    }

    /// A duration of a single item from the fingerprint.
    pub fn item_duration_in_seconds(&self) -> f32 {
        self.samples_in_item() as f32 / self.sample_rate() as f32
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Self::preset_test2()
    }
}

const MIN_FREQ: u32 = 28;
const MAX_FREQ: u32 = 3520;

const DEFAULT_SAMPLE_RATE: u32 = 11025;

/// Calculates a fingerprint for a given audio samples.
pub struct Fingerprinter {
    processor: AudioProcessor<Box<dyn AudioConsumer<f64, Output=[u32]>>>,
}

impl Fingerprinter {
    /// Creates a new [Fingerprinter] with the given [Configuration].
    pub fn new(config: &Configuration) -> Self {
        let normalizer = ChromaNormalizer::new(FingerprintCalculator::new(config.classifiers.clone()));
        let filter = ChromaFilter::new(config.filter_coefficients.clone().into_boxed_slice(), normalizer);
        let chroma = Chroma::new(
            MIN_FREQ,
            MAX_FREQ,
            config.frame_size,
            DEFAULT_SAMPLE_RATE,
            filter,
        );
        let fft = Fft::new(config.frame_size, config.frame_overlap, chroma);
        let processor = AudioProcessor::new(DEFAULT_SAMPLE_RATE, Box::new(fft) as Box<dyn AudioConsumer<_, Output=_>>);
        Self { processor }
    }

    /// Resets the internal state to allow for a new fingerprint calculation.
    pub fn start(&mut self, sample_rate: u32, channels: u32) -> Result<(), ResetError> {
        self.processor.reset(sample_rate, channels)?;
        Ok(())
    }

    /// Adds a new chunk of samples to the current calculation.
    pub fn consume(&mut self, data: &[i16]) {
        self.processor.consume(data)
    }

    /// Finishes the fingerprint calculation by flushing internal buffers.
    pub fn finish(&mut self) {
        self.processor.flush();
    }

    /// Returns the fingerprint of the last consumed audio data.
    pub fn fingerprint(&self) -> &[u32] {
        self.processor.output()
    }
}

const DEFAULT_FRAME_SIZE: usize = 4096;
const DEFAULT_FRAME_OVERLAP: usize = DEFAULT_FRAME_SIZE - DEFAULT_FRAME_SIZE / 3;

const CLASSIFIER_TEST1: [Classifier; 16] = [
    Classifier::new(Filter::new(FilterKind::Filter0, 0, 3, 15), Quantizer::new(2.10543, 2.45354, 2.69414)),
    Classifier::new(Filter::new(FilterKind::Filter1, 0, 4, 14), Quantizer::new(-0.345922, 0.0463746, 0.446251)),
    Classifier::new(Filter::new(FilterKind::Filter1, 4, 4, 11), Quantizer::new(-0.392132, 0.0291077, 0.443391)),
    Classifier::new(Filter::new(FilterKind::Filter3, 0, 4, 14), Quantizer::new(-0.192851, 0.00583535, 0.204053)),
    Classifier::new(Filter::new(FilterKind::Filter2, 8, 2, 4), Quantizer::new(-0.0771619, -0.00991999, 0.0575406)),
    Classifier::new(Filter::new(FilterKind::Filter5, 6, 2, 15), Quantizer::new(-0.710437, -0.518954, -0.330402)),
    Classifier::new(Filter::new(FilterKind::Filter1, 9, 2, 16), Quantizer::new(-0.353724, -0.0189719, 0.289768)),
    Classifier::new(Filter::new(FilterKind::Filter3, 4, 2, 10), Quantizer::new(-0.128418, -0.0285697, 0.0591791)),
    Classifier::new(Filter::new(FilterKind::Filter3, 9, 2, 16), Quantizer::new(-0.139052, -0.0228468, 0.0879723)),
    Classifier::new(Filter::new(FilterKind::Filter2, 1, 3, 6), Quantizer::new(-0.133562, 0.00669205, 0.155012)),
    Classifier::new(Filter::new(FilterKind::Filter3, 3, 6, 2), Quantizer::new(-0.0267, 0.00804829, 0.0459773)),
    Classifier::new(Filter::new(FilterKind::Filter2, 8, 1, 10), Quantizer::new(-0.0972417, 0.0152227, 0.129003)),
    Classifier::new(Filter::new(FilterKind::Filter3, 4, 4, 14), Quantizer::new(-0.141434, 0.00374515, 0.149935)),
    Classifier::new(Filter::new(FilterKind::Filter5, 4, 2, 15), Quantizer::new(-0.64035, -0.466999, -0.285493)),
    Classifier::new(Filter::new(FilterKind::Filter5, 9, 2, 3), Quantizer::new(-0.322792, -0.254258, -0.174278)),
    Classifier::new(Filter::new(FilterKind::Filter2, 1, 8, 4), Quantizer::new(-0.0741375, -0.00590933, 0.0600357))
];

const CLASSIFIER_TEST2: [Classifier; 16] = [
    Classifier::new(Filter::new(FilterKind::Filter0, 4, 3, 15), Quantizer::new(1.98215, 2.35817, 2.63523)),
    Classifier::new(Filter::new(FilterKind::Filter4, 4, 6, 15), Quantizer::new(-1.03809, -0.651211, -0.282167)),
    Classifier::new(Filter::new(FilterKind::Filter1, 0, 4, 16), Quantizer::new(-0.298702, 0.119262, 0.558497)),
    Classifier::new(Filter::new(FilterKind::Filter3, 8, 2, 12), Quantizer::new(-0.105439, 0.0153946, 0.135898)),
    Classifier::new(Filter::new(FilterKind::Filter3, 4, 4, 8), Quantizer::new(-0.142891, 0.0258736, 0.200632)),
    Classifier::new(Filter::new(FilterKind::Filter4, 0, 3, 5), Quantizer::new(-0.826319, -0.590612, -0.368214)),
    Classifier::new(Filter::new(FilterKind::Filter1, 2, 2, 9), Quantizer::new(-0.557409, -0.233035, 0.0534525)),
    Classifier::new(Filter::new(FilterKind::Filter2, 7, 3, 4), Quantizer::new(-0.0646826, 0.00620476, 0.0784847)),
    Classifier::new(Filter::new(FilterKind::Filter2, 6, 2, 16), Quantizer::new(-0.192387, -0.029699, 0.215855)),
    Classifier::new(Filter::new(FilterKind::Filter2, 1, 3, 2), Quantizer::new(-0.0397818, -0.00568076, 0.0292026)),
    Classifier::new(Filter::new(FilterKind::Filter5, 10, 1, 15), Quantizer::new(-0.53823, -0.369934, -0.190235)),
    Classifier::new(Filter::new(FilterKind::Filter3, 6, 2, 10), Quantizer::new(-0.124877, 0.0296483, 0.139239)),
    Classifier::new(Filter::new(FilterKind::Filter2, 1, 1, 14), Quantizer::new(-0.101475, 0.0225617, 0.231971)),
    Classifier::new(Filter::new(FilterKind::Filter3, 5, 6, 4), Quantizer::new(-0.0799915, -0.00729616, 0.063262)),
    Classifier::new(Filter::new(FilterKind::Filter1, 9, 2, 12), Quantizer::new(-0.272556, 0.019424, 0.302559)),
    Classifier::new(Filter::new(FilterKind::Filter3, 4, 2, 14), Quantizer::new(-0.164292, -0.0321188, 0.0846339)),
];

const CLASSIFIER_TEST3: [Classifier; 16] = [
    Classifier::new(Filter::new(FilterKind::Filter0, 4, 3, 15), Quantizer::new(1.98215, 2.35817, 2.63523)),
    Classifier::new(Filter::new(FilterKind::Filter4, 4, 6, 15), Quantizer::new(-1.03809, -0.651211, -0.282167)),
    Classifier::new(Filter::new(FilterKind::Filter1, 0, 4, 16), Quantizer::new(-0.298702, 0.119262, 0.558497)),
    Classifier::new(Filter::new(FilterKind::Filter3, 8, 2, 12), Quantizer::new(-0.105439, 0.0153946, 0.135898)),
    Classifier::new(Filter::new(FilterKind::Filter3, 4, 4, 8), Quantizer::new(-0.142891, 0.0258736, 0.200632)),
    Classifier::new(Filter::new(FilterKind::Filter4, 0, 3, 5), Quantizer::new(-0.826319, -0.590612, -0.368214)),
    Classifier::new(Filter::new(FilterKind::Filter1, 2, 2, 9), Quantizer::new(-0.557409, -0.233035, 0.0534525)),
    Classifier::new(Filter::new(FilterKind::Filter2, 7, 3, 4), Quantizer::new(-0.0646826, 0.00620476, 0.0784847)),
    Classifier::new(Filter::new(FilterKind::Filter2, 6, 2, 16), Quantizer::new(-0.192387, -0.029699, 0.215855)),
    Classifier::new(Filter::new(FilterKind::Filter2, 1, 3, 2), Quantizer::new(-0.0397818, -0.00568076, 0.0292026)),
    Classifier::new(Filter::new(FilterKind::Filter5, 10, 1, 15), Quantizer::new(-0.53823, -0.369934, -0.190235)),
    Classifier::new(Filter::new(FilterKind::Filter3, 6, 2, 10), Quantizer::new(-0.124877, 0.0296483, 0.139239)),
    Classifier::new(Filter::new(FilterKind::Filter2, 1, 1, 14), Quantizer::new(-0.101475, 0.0225617, 0.231971)),
    Classifier::new(Filter::new(FilterKind::Filter3, 5, 6, 4), Quantizer::new(-0.0799915, -0.00729616, 0.063262)),
    Classifier::new(Filter::new(FilterKind::Filter1, 9, 2, 12), Quantizer::new(-0.272556, 0.019424, 0.302559)),
    Classifier::new(Filter::new(FilterKind::Filter3, 4, 2, 14), Quantizer::new(-0.164292, -0.0321188, 0.0846339)),
];

const CHROMA_FILTER_COEFFICIENTS: [f64; 5] = [0.25, 0.75, 1.0, 0.75, 0.25];
