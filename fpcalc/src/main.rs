use clap::Parser;
use std::path::PathBuf;
use std::fmt;
use rusty_chromaprint::Configuration;

#[derive(Default, Debug, Clone)]
struct Algorithm(Configuration);

impl TryFrom<&str> for Algorithm {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Algorithm, Self::Error> {
        let algorithm_id = value.parse::<u8>().map_err(|_| "value must be between an integer between 0 and 4")?;
        let configuration = match algorithm_id {
            0 => Configuration::preset_test1(),
            1 => Configuration::preset_test2(),
            2 => Configuration::preset_test3(),
            3 => Configuration::preset_test4(),
            4 => Configuration::preset_test5(),
            _ => { return Err("unknown algorithm ID"); },
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

    /// Print the output in JSON format
    #[arg(short, long)]
    json: bool,

    /// Print the output in text format
    #[arg(short, long)]
    text: bool,

    /// Print the just the fingerprint in text format
    #[arg(short, long)]
    plain: bool,

    /// File to analyze
    file: PathBuf,
}

fn main() {
    let _args = Args::parse();

    todo!();
}
