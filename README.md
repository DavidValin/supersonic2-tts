# supersonic2-tts

Extremely fast tts for rust with realistic voices, different styles, speed support.
Perfect for embedded devices.

* Model size: 234 MB [⬇️Download](https://github.com/DavidValin/supersonic2-tts/releases/download/1.2.0/supersonic2-model.tgz)

## Language support / Voice styles

(Each language has 5 female and 5 male voice styles)

* 🇬🇧 en (English)
* 🇪🇸 es (Spanish)
* 🇫🇷 fr (French)
* 🇰🇷 ko (Korean)
* 🇵🇹 pt (Portuguese)

## Download the model

The model (ONNX files + voice styles) is packaged as a single archive in the GitHub release:
https://github.com/DavidValin/supersonic2-tts/releases/download/1.2.0/supersonic2-model.tgz

Use the bundled script (curl + tar, ~234 MB, no git-lfs needed; the archive includes the model LICENSE):
```sh
sh scripts/download-model.sh ./supersonic2-model
```

Expected layout:
```
supersonic2-model/
├── LICENSE            (BigScience Open RAIL-M, the model license)
├── onnx/
│   ├── duration_predictor.onnx
│   ├── text_encoder.onnx
│   ├── vector_estimator.onnx
│   ├── vocoder.onnx
│   ├── tts.json
│   └── unicode_indexer.json
└── voice_styles/
    ├── M1.json ... M5.json
    └── F1.json ... F5.json
```

## Quickstart

A small cli is provided in `src/main.rs`.  It exposes the same options as the library but as a command‑line interface.

Download the supersonic 2 model (see above), then synthetize and play it:
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

With the best voice quality (12, slower; default is 5, the fastest):
```sh
cargo run -- \
    --root-models-path ./supersonic2-model \
    --text "Hey man! supersonic 2 is as fast as a porche. How do you like it so far?" \
    --voice F4 \
    --language en \
    --voice-quality 12 \
    --output output.wav
```

All options:
```
-t, --text <TEXT>                 Text to synthesize
-v, --voice <VOICE>               Voice style id (M1-M5, F1-F5) or path to a voice style JSON [default: M1]
-o, --output <OUTPUT>             Output WAV file path (plays the audio when omitted)
-r, --root-models-path <PATH>     Root folder of the model [default: ./supersonic2-model]
-l, --language <LANGUAGE>         Language code (en, es, fr, ko, pt) [default: en]
-s, --speed <SPEED>               Speech speed (1.0 = normal, >1.0 faster, <1.0 slower) [default: 1.0]
-q, --voice-quality <QUALITY>     Voice quality (denoising steps): 5 (fastest) .. 12 (best) [default: 5]
-g, --gpu                         Synthesize on the GPU (needs a build with a GPU feature, see below)
    --gpu-device <ID>             GPU device id to use with --gpu (0 = first GPU) [default: 0]
```

## GPU synthesis

By default everything runs on the CPU and no GPU feature is enabled. To
synthesize on a GPU, build with the feature that matches your hardware:

| Feature    | Hardware                        | Notes                                                                 |
|------------|---------------------------------|-----------------------------------------------------------------------|
| `cuda`     | NVIDIA (Linux/Windows x86_64)   | Needs the CUDA toolkit and cuDNN runtime libraries installed          |
| `tensorrt` | NVIDIA (Linux/Windows x86_64)   | Needs TensorRT; unsupported ops fall back to CUDA (implies `cuda`)    |
| `rocm`     | AMD (Linux x86_64)              | No prebuilt binaries: point `ORT_LIB_LOCATION` at a ROCm build of ONNX Runtime |
| `directml` | Any GPU on Windows              | Uses DirectML                                                         |
| `coreml`   | Apple Silicon / macOS           | Uses CoreML (GPU / Neural Engine)                                     |

For `cuda` and `tensorrt` the `ort` crate downloads a matching prebuilt ONNX
Runtime at build time (CUDA 12 by default; set `ORT_CUDA_VERSION=13` for CUDA 13).
DirectML and CoreML are part of the standard Windows / macOS binaries. ONNX
Runtime ships no prebuilt ROCm binaries, so `rocm` requires a self-built
ONNX Runtime with ROCm enabled, found through `ORT_LIB_LOCATION`.

```sh
cargo build --release --features cuda

./target/release/main \
    --root-models-path ./supersonic2-model \
    --text "Now running on the GPU" \
    --voice F4 \
    --gpu \
    --output output.wav
```

`--gpu` fails with an error (instead of silently falling back to the CPU) when
the binary was built without a GPU feature or the driver / runtime libraries
are missing. Without `--gpu` a GPU build still synthesizes on the CPU.

From the library:

```rust
use supersonic2_tts::{Device, TtsEngine, gpu_support_compiled};

let device = if gpu_support_compiled() { Device::gpu() } else { Device::Cpu };
let engine = TtsEngine::new_with_device(onnx, base, false, device).await?;
```

`Device::Gpu { device_id }` selects the GPU when several are installed.

## Using as library

Download the supersonic 2 model (see above).

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
            Some(12),       // voice quality (5 fastest .. 12 best, None = default 5)
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
  the engine at any directory structure (runs on the CPU).
* `TtsEngine::new_with_device(onnx_dir, base_path, verbose, device)` – same, on a
  chosen `Device` (`Device::Cpu` or `Device::Gpu { device_id }`, see "GPU synthesis").
* `gpu_support_compiled()` / `compiled_gpu_backends()` – whether (and which) GPU
  backends were compiled in through the crate features.
* `synthesize_with_options(text, voice, speed, gain, language, voice_quality)` – synthesize
  text with optional voice style, speed, gain, language and voice quality (`voice_quality` =
  denoising steps: 5 fastest .. 12 best, `None` = engine default 5; values outside that range
  return an error).
* `TtsEngine::with_voice_quality(n)` – change the engine default voice quality (default 5,
  validated to 5..=12).
* `play_wav` / `save_wav` – helper methods for playback and file
  persistence.

## LICENSE

* Source code: MIT
* Supersonic2 model: BigScience Open RAIL-M License
