
use crate::compiler::compile_filtergraph;
use crate::dsl::parse_script;
use std::{
    path::Path,
    process::Command,
};

pub fn render_preview(
    script_text: &str,
    output_path: &Path,
    duration_seconds: u32,
) -> Result<(), String> {
    let script = parse_script(script_text)?;

    if duration_seconds == 0 || duration_seconds > 15 {
        return Err("Preview duration must be between 1 and 15 seconds".into());
    }

    let input = Path::new(&script.source);

    if !input.is_file() {
        return Err(format!(
            "Source video not found: {}",
            input.display()
        ));
    }

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Cannot create output directory: {e}"))?;
    }

    // Compile the DSL operations into an FFmpeg filtergraph.
    // An empty filtergraph means no effects are needed.
    let mut filtergraph = compile_filtergraph(&script.operations);

    if filtergraph.is_empty() {
        filtergraph = "null".to_string();
    }

    // Use FFMPEG_PATH if configured; otherwise search PATH.
    let ffmpeg = std::env::var_os("FFMPEG_PATH")
        .unwrap_or_else(|| "ffmpeg".into());

    let result = Command::new(&ffmpeg)
        .arg("-y")
        .arg("-i")
        .arg(&script.source)
        .arg("-t")
        .arg(duration_seconds.to_string())
        .arg("-vf")
        .arg(&filtergraph)
        .arg("-an")
        .arg("-c:v")
        .arg("libx264")
        .arg("-preset")
        .arg("ultrafast")
        .arg("-crf")
        .arg("28")
        .arg("-pix_fmt")
        .arg("yuv420p")
        .arg("-movflags")
        .arg("+faststart")
        .arg(output_path)
        .output()
        .map_err(|e| {
            format!(
                "Could not launch FFmpeg. Add it to PATH or set \
                 FFMPEG_PATH to ffmpeg.exe. Details: {e}"
            )
        })?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        return Err(format!("FFmpeg render failed:\n{stderr}"));
    }

    Ok(())
}
