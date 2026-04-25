# supersonic2-tts

Extremely fast tts for rust with realistic voices, different styles, speed support.
Perfect for embedded devices.

* Model size: 234 MB [⬇️Download](https://github.com/DavidValin/supersonic2-tts/releases/download/1.0.3/supersonic2-model.tgz)

## Language support / Voice styles

(Each language has 5 female and 5 male voice styles)

* 🇬🇧 en (English)
* 🇪🇸 es (Spanish)
* 🇫🇷 fr (French)
* 🇰🇷 ko (Korean)
* 🇵🇹 pt (Portuguese)

## Quickstart

A small cli is provided in `src/main.rs`.  It exposes the same options as the library but as a command‑line interface.

Unzip the supersonic2 model [⬇️Download](https://github.com/DavidValin/supersonic2-tts/releases/download/1.0.3/supersonic2-model.tgz):
```sh
tar xvf model/supersonic2-model.tgz
```

Synthetize and play it:
```sh
cargo run -- \
    --root-models-path ./supersonic2-model \
    --text "Hey man! supersonic 2 is as fast as a porche. How do you like it so far?" \
    --voice F4 \
    --language en \
    --speed 1.2
```

Synthetize and save it:
```sh
cargo run -- \
    --root-models-path ./supersonic2-model \
    --text 'Hola amigo!. ¡Éste sistema de audio es increíble y rapidísimo! ¿Qué te parece?' \
    --voice M1 \
    --language es \
    --speed 1.5 \
    --output output.wav
```

## Using as library

Unzip the supersonic2 model:
```
tar xvf model/supersonic2-model.tgz
```

Install crate, add dependency to your Cargo.toml
```
cargo install supersonic2-tts
```

Use the library (example):

```rust
use supersonic2_tts::TtsEngine;
use std::path::PathBuf;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Path to the folder that contains `onnx` and voice style JSON files
    let base = PathBuf::from("./supersonic2-model");
    let onnx = base.join("onnx");

    // Create a new engine with the custom base path
    let engine = TtsEngine::new(onnx, base, false).await?;

    // Synthesize a phrase in Spanish using the M3 voice style
    let wav = engine
        .synthesize_with_options(
            "Absolute incredible system! It's very fast, don't you think?",
            Some("M3"),     // voice style
            1.5,            // speed
            1.0,            // gain
            Some("es"),     // language
        )
        .await?;

    // Play it back immediately
    engine.play_wav(&wav).await?;

    // Or write it to a file
    engine.save_wav("output.wav", &wav).await?;
    Ok(())
}
```

The public API is intentionally minimal:

* `TtsEngine::new()` – loads the default assets next to the binary.
* `TtsEngine::new(onnx_dir, base_path, verbose)` – allows you to point
  the engine at any directory structure.
* `synthesize_with_options(...)` – synthesize text with optional voice
  style, speed, gain and language.
* `play_wav` / `save_wav` – helper methods for playback and file
  persistence.

## LICENSE

* Source code: MIT
* Supersonic2 model: BigScience Open RAIL-M License
