use base64::prelude::{Engine, BASE64_URL_SAFE_NO_PAD};
use chrono::Local;
use clap::Parser;
use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::Context;
use rusty_chromaprint::{Configuration, FingerprintCompressor, Fingerprinter};
use symphonia::core::audio::{AudioBufferRef, SampleBuffer};
use symphonia::core::codecs::{Decoder, DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error;
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

#[derive(Default, Debug, Clone)]
struct Algorithm(Configuration);

impl Algorithm {
    fn as_config(&self) -> &Configuration {
        &self.0
    }
}

impl TryFrom<&str> for Algorithm {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Algorithm, Self::Error> {
        let algorithm_id = value
            .parse::<u8>()
            .map_err(|_| "value must be between an integer between 0 and 4")?;
        let configuration = match algorithm_id {
            0 => Configuration::preset_test1(),
            1 => Configuration::preset_test2(),
            2 => Configuration::preset_test3(),
            3 => Configuration::preset_test4(),
            4 => Configuration::preset_test5(),
            _ => {
                return Err("unknown algorithm ID");
            }
        };
        debug_assert_eq!(configuration.id(), algorithm_id);
        let algorithm = Algorithm(configuration);
        Ok(algorithm)
    }
}

impl fmt::Display for Algorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.id().fmt(f)
    }
}

/// Generate fingerprints from audio files/streams.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Set the input format name
    #[arg(short, long)]
    format: Option<String>,

    /// Set the sample rate of the input audio
    #[arg(short, long)]
    rate: Option<usize>,

    /// Set the number of channels in the input audio
    #[arg(short, long)]
    channels: Option<usize>,

    /// Restrict the duration of the processed input audio
    #[arg(short, long, default_value_t = 120)]
    length: usize,

    /// Split the input audio into chunks of this duration
    #[arg(short = 'C', long)]
    chunk: Option<usize>,

    /// Set the algorithm method.
    #[arg(short, long, value_parser = |s: &str| Algorithm::try_from(s), default_value_t)]
    algorithm: Algorithm,

    /// Overlap the chunks slightly to make sure audio on the edges is fingerprinted
    #[arg(short, long)]
    overlap: bool,

    /// Output UNIX timestamps for chunked results, useful when fingerprinting real-time audio stream
    #[arg(short = 'T', long)]
    ts: bool,

    /// Output fingerprints in the uncompressed format
    #[arg(short = 'R', long)]
    raw: bool,

    /// Change the uncompressed format from unsigned integers to signed (for pg_acoustid compatibility)
    #[arg(short, long)]
    signed: bool,

    /// Print the output in a certain format
    #[arg(short='F', long, value_parser = |s: &str| OutputFormat::try_from(s), default_value = "text")]
    output_format: OutputFormat,

    /// File to analyze
    file: PathBuf,
}

impl Args {
    fn max_chunk_duration(&self) -> usize {
        self.chunk.unwrap_or(0)
    }

    fn to_result_printer(&self) -> ResultPrinter<'_> {
        ResultPrinter {
            config: self.algorithm.as_config(),
            abs_ts: self.ts,
            raw: self.raw,
            signed: self.signed,
            format: self.output_format,
            max_chunk_duration: self.max_chunk_duration(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum OutputFormat {
    Text,
    Json,
    Plain,
}

impl TryFrom<&str> for OutputFormat {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<OutputFormat, Self::Error> {
        match value {
            "text" => Ok(OutputFormat::Text),
            "json" => Ok(OutputFormat::Json),
            "plain" => Ok(OutputFormat::Plain),
            _ => Err("invalid result format"),
        }
    }
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            Self::Text => "text".fmt(f),
            Self::Json => "json".fmt(f),
            Self::Plain => "plain".fmt(f),
        }
    }
}

struct AudioReader {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    sample_rate: u32,
    channel_count: usize,
}

impl AudioReader {
    fn new(path: &impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let src = std::fs::File::open(path).context("failed to open file")?;
        let mss = MediaSourceStream::new(Box::new(src), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let meta_opts: MetadataOptions = Default::default();
        let fmt_opts: FormatOptions = Default::default();

        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &fmt_opts, &meta_opts)
            .context("unsupported format")?;

        let format = probed.format;

        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .context("no supported audio tracks")?;

        let track_id = track.id;

        let dec_opts: DecoderOptions = Default::default();

        let decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &dec_opts)
            .context("unsupported codec")?;

        let sample_rate = track
            .codec_params
            .sample_rate
            .context("missing sample rate")?;
        let channel_count = track
            .codec_params
            .channels
            .context("missing audio channels")?
            .count();

        Ok(Self {
            format,
            decoder,
            track_id,
            sample_rate,
            channel_count,
        })
    }

    fn next_buffer(&mut self) -> Result<AudioBufferRef<'_>, Error> {
        let packet = loop {
            let packet = match self.format.next_packet() {
                Ok(packet) => packet,
                err => break err,
            };

            if packet.track_id() != self.track_id {
                continue;
            }

            break Ok(packet);
        };
        packet.and_then(|pkt| self.decoder.decode(&pkt))
    }
}

fn get_current_timestamp() -> f64 {
    let now = Local::now();
    let usec = now.timestamp_micros();
    (usec as f64) / 1000000.0
}

pub fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let result_printer = args.to_result_printer();

    let mut reader = AudioReader::new(&args.file).context("initializing audio reader")?;

    let config = args.algorithm.as_config();
    let mut printer = Fingerprinter::new(config);

    let channel_count: u32 = reader
        .channel_count
        .try_into()
        .context("converting sample rate")?;
    printer
        .start(reader.sample_rate, channel_count)
        .context("initializing fingerprinter")?;

    let mut sample_buf = None;

    let mut ts: f64 = 0.0;
    if args.ts {
        ts = get_current_timestamp();
    }

    let sample_rate = usize::try_from(reader.sample_rate).context("invalid sample rate")?;

    let mut stream_size = 0;
    let stream_limit = args.length * sample_rate;

    let mut chunk_size = 0;
    let chunk_limit = args.max_chunk_duration() * sample_rate;

    let mut extra_chunk_limit = 0;
    let mut overlap: f64 = 0.0;

    if chunk_limit > 0 && args.overlap {
        extra_chunk_limit = config.delay();
        overlap = (config.delay() as f64) * 1.0 / (sample_rate as f64) / 1000.0;
    }

    let mut first_chunk = true;

    loop {
        let audio_buf = match reader.next_buffer() {
            Ok(buffer) => buffer,
            Err(Error::DecodeError(err)) => Err(Error::DecodeError(err))?,
            Err(_) => break,
        };

        if sample_buf.is_none() {
            let spec = *audio_buf.spec();
            let duration = audio_buf.capacity() as u64;
            sample_buf = Some(SampleBuffer::<i16>::new(duration, spec));
        }

        if let Some(buf) = &mut sample_buf {
            let (stream_done, mut frame_size) = if stream_limit > 0 {
                let remaining = stream_limit - stream_size;
                let frame_size = audio_buf.frames();
                (frame_size > remaining, frame_size.min(remaining))
            } else {
                (false, audio_buf.frames())
            };
            stream_size += frame_size;

            if frame_size == 0 {
                if stream_done {
                    break;
                } else {
                    continue;
                }
            }

            let first_part_size = frame_size;
            let (chunk_done, first_part_size) = if chunk_limit > 0 {
                let remaining = chunk_limit + extra_chunk_limit - chunk_size;
                (first_part_size > remaining, first_part_size.min(remaining))
            } else {
                (false, first_part_size)
            };

            buf.copy_interleaved_ref(audio_buf);
            let frame_data = buf.samples();
            printer.consume(&frame_data[..first_part_size * reader.channel_count]);

            chunk_size += first_part_size;

            if chunk_done {
                printer.finish();

                let chunk_duration = (chunk_size - extra_chunk_limit) as f64 * 1.0
                    / f64::from(reader.sample_rate)
                    + overlap;
                result_printer.print_result(&printer, first_chunk, ts, chunk_duration);

                if args.ts {
                    ts = get_current_timestamp();
                } else {
                    ts += chunk_duration;
                }

                if args.overlap {
                    printer = Fingerprinter::new(config);
                    ts -= overlap;
                } else {
                    printer
                        .start(reader.sample_rate, channel_count)
                        .context("initializing fingerprinter")?;
                }

                if first_chunk {
                    extra_chunk_limit = 0;
                    first_chunk = false;
                }

                chunk_size = 0;
            }

            frame_size -= first_part_size;
            if frame_size > 0 {
                printer.consume(
                    &frame_data[(first_part_size * reader.channel_count)
                        ..(frame_size * reader.channel_count)],
                );
            }

            chunk_size += frame_size;

            if stream_done {
                break;
            }
        }
    }

    printer.finish();

    if chunk_size > 0 {
        let chunk_duration =
            (chunk_size - extra_chunk_limit) as f64 * 1.0 / f64::from(reader.sample_rate) + overlap;
        result_printer.print_result(&printer, first_chunk, ts, chunk_duration);
    }

    Ok(())
}

struct ResultPrinter<'a> {
    config: &'a Configuration,
    abs_ts: bool,
    raw: bool,
    signed: bool,
    format: OutputFormat,
    max_chunk_duration: usize,
}

impl<'a> ResultPrinter<'a> {
    fn print_result(&self, printer: &Fingerprinter, first: bool, timestamp: f64, duration: f64) {
        let raw_fingerprint = printer.fingerprint();
        let fp = if self.raw {
            if self.signed {
                // FIXME: Use `u32.case_signed()` once it becomes stable.
                raw_fingerprint
                    .iter()
                    .map(|x| *x as i32)
                    .map(|x| x.to_string())
                    .collect::<Vec<String>>()
                    .join(",")
            } else {
                raw_fingerprint
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<String>>()
                    .join(",")
            }
        } else {
            let compressed_fingerprint =
                FingerprintCompressor::from(self.config).compress(raw_fingerprint);
            BASE64_URL_SAFE_NO_PAD.encode(&compressed_fingerprint)
        };

        match self.format {
            OutputFormat::Text => {
                if !first {
                    println!();
                }

                if self.abs_ts {
                    println!("TIMESTAMP={timestamp:.2}");
                }
                println!("DURATION={duration}");
                println!("FINGERPRINT={fp}");
            }
            OutputFormat::Json => {
                if self.max_chunk_duration != 0 {
                    if self.raw {
                        println!("{{\"timestamp\": {timestamp:.2}, \"duration\": {duration:.2}, \"fingerprint\": [{fp}]}}");
                    } else {
                        println!("{{\"timestamp\": {timestamp:.2}, \"duration\": {duration:.2}, \"fingerprint\": \"{fp}\"}}");
                    }
                } else {
                    if self.raw {
                        println!("{{\"duration\": {duration:.2}, \"fingerprint\": [{fp}]}}");
                    } else {
                        println!("{{\"duration\": {duration:.2}, \"fingerprint\": \"{fp}\"}}");
                    }
                }
            }
            OutputFormat::Plain => {
                println!("{fp}");
            }
        }
    }
}
