const MIN_SAMPLE_RATE: u32 = 1000;
const MAX_BUFFER_SIZE: usize = 1024 * 32;

pub trait AudioConsumer {
    fn reset(&mut self);
    fn consume(&mut self, data: &[i16]);
}

impl<C: AudioConsumer + ?Sized> AudioConsumer for Box<C> {
    fn reset(&mut self) {
        (**self).reset();
    }

    fn consume(&mut self, data: &[i16]) {
        (**self).consume(data);
    }
}

pub struct AudioProcessor<C: AudioConsumer> {
    buffer: Vec<i16>,
    buffer_offset: usize,
    channels: u32,
    consumer: C,
    target_sample_rate: u32,
    resampler: Option<()>,
}

impl<C: AudioConsumer> AudioProcessor<C> {
    pub(crate) fn new(sample_rate: u32, consumer: C) -> Self {
        Self {
            buffer: vec![0; MAX_BUFFER_SIZE],
            buffer_offset: 0,
            channels: 0,
            consumer,
            target_sample_rate: sample_rate,
            resampler: None,
        }
    }

    fn load_mono(&mut self, input: &[i16]) {
        for sample in input.iter().copied() {
            self.push_sample(sample);
        }
    }

    fn load_stereo(&mut self, input: &[i16]) {
        for sample in input.chunks_exact(2) {
            self.push_sample((((sample[0] as i32) + (sample[1] as i32)) / 2) as i16);
        }
    }

    fn load_multi_channel(&mut self, input: &[i16]) {
        for sample in input.chunks_exact(self.channels as usize) {
            self.push_sample((sample.iter()
                .copied()
                .map(|v| v as i32)
                .sum::<i32>() / sample.len() as i32) as i16
            );
        }
    }

    fn load(&mut self, input: &[i16]) -> usize {
        assert!(self.buffer_offset <= self.buffer.len());
        assert_eq!(input.len() % self.channels as usize, 0);

        let available_samples = input.len() / self.channels as usize;
        let max_samples = available_samples.min(self.available_space());
        let input = &input[..max_samples * self.channels as usize];

        match self.channels {
            1 => self.load_mono(input),
            2 => self.load_stereo(input),
            _ => self.load_multi_channel(input),
        }

        max_samples
    }

    fn resample(&mut self) -> Result<(), ()> {
        if let Some(_resampler) = self.resampler.as_mut() {
            todo!();
        } else {
            self.consumer.consume(&self.buffer[..self.buffer_offset]);
            self.buffer_offset = 0;
            Ok(())
        }
    }

    fn available_space(&self) -> usize {
        self.buffer.len() - self.buffer_offset
    }

    #[inline]
    fn push_sample(&mut self, value: i16) {
        self.buffer[self.buffer_offset] = value;
        self.buffer_offset += 1;
    }

    pub(crate) fn reset(&mut self, sample_rate: u32, channels: u32) -> Result<(), ResetError> {
        if channels == 0 {
            return Err(ResetError::NoChannels);
        }

        if sample_rate <= MIN_SAMPLE_RATE {
            return Err(ResetError::SampleRateTooLow);
        }

        if sample_rate != self.target_sample_rate {
            return Err(ResetError::CannotResample);
        }

        self.channels = channels;
        self.buffer_offset = 0;
        self.consumer.reset();

        Ok(())
    }

    pub(crate) fn flush(&mut self) {
        if self.buffer_offset > 0 {
            self.resample().unwrap();
        }
    }

    fn into_consumer(self) -> C {
        self.consumer
    }
}

impl<C: AudioConsumer> AudioConsumer for AudioProcessor<C> {
    fn reset(&mut self) {
        todo!();
    }

    fn consume(&mut self, data: &[i16]) {
        assert_eq!(data.len() % self.channels as usize, 0);

        let mut index = 0;
        while index < data.len() {
            let consumed = self.load(&data[index..]);
            index += consumed * self.channels as usize;
            if self.buffer.len() == self.buffer_offset {
                // Full buffer
                self.resample().unwrap();
            }
        }
    }
}

#[derive(Debug)]
pub(crate) enum ResetError {
    SampleRateTooLow,
    NoChannels,
    CannotResample,
}

#[cfg(test)]
mod tests {
    use crate::audio_processor::{AudioBuffer, AudioConsumer, AudioProcessor};
    use crate::read_s16le;

    #[test]
    fn pass_through() {
       let data = read_s16le("data/test_mono_44100.raw");
        let mut processor = AudioProcessor::new(44100, AudioBuffer::new());
        processor.reset(44100, 1).unwrap();
        processor.consume(&data);
        processor.flush();
        let buffer = processor.into_consumer();
       assert_eq!(buffer.data, data);
    }

    #[test]
    fn stereo_to_mono() {
        let data1 = read_s16le("data/test_mono_44100.raw");
        let data2 = read_s16le("data/test_stereo_44100.raw");

        let mut processor = AudioProcessor::new(44100, AudioBuffer::new());
        processor.reset(44100, 2).unwrap();
        processor.consume(&data2);
        processor.flush();
        let buffer = processor.into_consumer();
        assert_eq!(buffer.data, data1);
    }
}

struct AudioBuffer {
    data: Vec<i16>,
}

impl AudioBuffer {
    fn new() -> Self {
        Self {
            data: Vec::new(),
        }
    }
}

impl AudioConsumer for AudioBuffer {
    fn reset(&mut self) {
        self.data.clear();
    }

    fn consume(&mut self, data: &[i16]) {
        self.data.extend_from_slice(data);
    }
}