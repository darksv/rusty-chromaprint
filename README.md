# rusty-chromaprint
This is an in-progress port of [chromaprint](https://github.com/acoustid/chromaprint):

> Chromaprint is an audio fingerprint library developed for the [AcoustID](https://acoustid.org/) project. It's designed to identify near-identical audio and the fingerprints it generates are as compact as possible to achieve that. It's not a general purpose audio fingerprinting solution. It trades precision and robustness for search performance. The target use cases are full audio file identification, duplicate audio file detection and long audio stream monitoring.

## Usage
To calculate a fingerprint for an audio stream simply create a new `Fingerprinter` 
and give it all the audio samples:

```rust
use rusty_chromaprint::{Configuration, Fingerprinter};

fn main() {
    // Use a preset configuration. This must be always the same for the audio fingerprints 
    // that are going to be compared against each other.
    let mut printer = Fingerprinter::new(&Configuration::preset_test2());
    
    // Sampling rate is set to 44100 and stream has 2 audio channels. It is expected that samples 
    // are interleaved: in this case left channel samples are placed at even indices 
    // and right channel - at odd ones.
    printer.start(44100, 2).unwrap();
    
    // Process a few samples...
    printer.consume(&[-100, -100, -50, -50, 1000, 1000]);
    // ... and add some more...
    printer.consume(&more_samples);
    
    // Make sure that all the sample are processed.
    printer.finish();
    
    // Get the fingerprint.
    let fingerprint = printer.fingerprint();

    println!("fingerprint = {:08x?}", &fingerprint);
}
```

For a complete example check out [`compare`](https://github.com/darksv/rusty-chromaprint/blob/main/compare/src/main.rs) from this repository
which is using [Symphonia](https://github.com/pdeljanov/Symphonia) to decode various audio formats. It compares two files and prints out their common segments
```
cargo run --release --bin compare -- audio1.mp3 audio2.wav
```

```
  #  |          File 1          |          File 2          |  Duration  |  Score  
-----+--------------------------+--------------------------+------------+---------
   1 | 0:00:04.83 -- 0:00:19.44 | 0:00:00.00 -- 0:00:14.61 | 0:00:14.61 |   0.69
```

For more details on comparing audio fingerprints reach out to the [documentation](https://docs.rs/rusty-chromaprint/latest/rusty_chromaprint/fn.match_fingerprints.html).
