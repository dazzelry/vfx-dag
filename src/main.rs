mod graph;
mod ffmpeg;
mod dsl;
mod compiler;
use graph::{
    Blend, BlendMode, BoxBlur, ClipSource, ColorCorrection, FrameBuffer, FrameCtx, GraphBuilder, Node,
};
use std::collections::{HashMap, HashSet};

/// Evaluate one frame. Returns `None` if `root` is inactive (or produces nothing) at `frame_idx`.
pub fn evaluate_frame(
    graph: &HashMap<String, Node>,
    frame_idx: u32,
    root: &str,
) -> Option<FrameBuffer> {
    let mut cache: HashMap<String, Option<FrameBuffer>> = HashMap::new();
    let mut visiting = HashSet::new();
    eval_node(graph, frame_idx, root, &mut cache, &mut visiting)
}

fn eval_node(
    graph: &HashMap<String, Node>,
    frame_idx: u32,
    name: &str,
    cache: &mut HashMap<String, Option<FrameBuffer>>,
    visiting: &mut HashSet<String>,
) -> Option<FrameBuffer> {
    if let Some(buf) = cache.get(name) {
        return buf.clone();
    }
    if !visiting.insert(name.to_string()) {
        panic!("cycle detected at node {name}");
    }

    let node = graph
        .get(name)
        .unwrap_or_else(|| panic!("unknown node '{name}'"));

    // Outside the node's active range: it contributes nothing.
    let output = if frame_idx < node.start_frame || frame_idx > node.end_frame {
        None
    } else {
        let inputs: Vec<Option<FrameBuffer>> = node
            .dependencies
            .iter()
            .map(|dep| eval_node(graph, frame_idx, dep, cache, visiting))
            .collect();

        let ctx = FrameCtx {
            global: frame_idx,
            local: frame_idx - node.start_frame,
        };
        node.processor.process(ctx, &inputs)
    };

    visiting.remove(name);
    cache.insert(name.to_string(), output.clone());
    output
}

fn main() {
    // 1. Load background clip
    // 2. Color-correct it
    // 3. Load a foreground/overlay clip (frames 30..=70 only)
    // 4. Blur the foreground
    // 5. Composite with Screen blend (background passes through when fg is inactive)
    let (graph, execution_order) = GraphBuilder::new()
        // Background: blue-ish gradient
        .add_clip("bg_load", 0, 100, Box::new(ClipSource {
            color: [50, 100, 200],
            width: 640,
            height: 480,
        }))
        .add_clip("bg_color", 0, 100, Box::new(ColorCorrection {
            brightness: 30.0,
            contrast: 1.1,
            saturation: 1.2,
        }))
        .depends_on("bg_color", "bg_load")
        // Foreground: red-ish gradient (active frames 30..=70 only)
        .add_clip("fg_load", 30, 70, Box::new(ClipSource {
            color: [255, 100, 50],
            width: 640,
            height: 480,
        }))
        .add_clip("fg_blur", 30, 70, Box::new(BoxBlur { radius: 3 }))
        .depends_on("fg_blur", "fg_load")
        // Composite: blend fg onto bg with Screen mode
        .add_clip("composite", 0, 100, Box::new(Blend {
            mode: BlendMode::Screen,
            opacity: 0.8,
        }))
        .depends_on("composite", "bg_color")
        .depends_on("composite", "fg_blur")
        .build();

    println!("--- ProseCut: Effect Processor Pipeline ---");
    println!("Execution order: {:?}", execution_order);
    println!();

    // Spot-check frames before fg, during fg, and after fg.
    for frame_idx in [15, 50, 85] {
        println!("Frame {}", frame_idx);
        match evaluate_frame(&graph, frame_idx, "composite") {
            None => println!("  (inactive range)"),
            Some(out) => println!(
                "  Output: {}x{}, {} pixels",
                out.width,
                out.height,
                out.pixels.len()
            ),
        }
    }

    // Render the full range to PNGs.
    // Assemble with: ffmpeg -framerate 24 -start_number 0 -i out/frame_%04d.png -pix_fmt yuv420p out.mp4
    println!();
    println!("Rendering frames 0..=100 to out/ ...");
    for frame_idx in 0..=100u32 {
        match evaluate_frame(&graph, frame_idx, "composite") {
            Some(out) => {
                let path = format!("out/frame_{:04}.png", frame_idx);
                if let Err(e) = out.save_png(&path) {
                    eprintln!("failed to write {path}: {e}");
                    return;
                }
            }
            None => eprintln!("frame {frame_idx}: composite inactive, skipped"),
        }
    }
    println!("Done.");
}