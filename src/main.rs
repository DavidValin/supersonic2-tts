use clap::Parser;
use anyhow::Result;
use supersonic2_tts::TtsEngine;
use std::path::PathBuf;

/// CLI arguments for the new example
#[derive(Parser, Debug)]
struct Args {
    /// Text to synthesize
    #[arg(long, short, required = true)]
    text: String,

    /// Voice style id (file name inside assets/voice_styles)
    #[arg(long, short)]
    voice: Option<String>,

    /// Output WAV file path
    #[arg(long, short, default_value = "")]
    output: String,

    /// Language for synthesis (e.g., "en", "es", etc.)
    #[arg(long, short, default_value = "./supersonic2-model")]
    root_models_path: String,

    #[arg(long, short, default_value = "en")]
    language: String,

    /// Speech speed factor (1.0 = normal, >1.0 faster, <1.0 slower)
    #[arg(long, short, default_value = "1.0")]
    speed: f32,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize the engine
    let root_path = PathBuf::from(&args.root_models_path);
    let onnx_dir = root_path.join("onnx");
    let engine = TtsEngine::new(onnx_dir, root_path, true).await?;

    // Synthesize
    let wav = engine
        .synthesize_with_options(
            &args.text,
            args.voice.as_deref(),
            args.speed,
            1.0,
            Some(&args.language),
        )
        .await?;

    if args.output.is_empty() {
        // Play directly
        engine.play_wav(&wav).await?;
        println!("Playing audio");
    } else {
        engine.save_wav(&args.output, &wav).await?;
        println!("Saved WAV to {}", args.output);
    }
    Ok(())
}
