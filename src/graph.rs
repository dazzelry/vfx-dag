use std::collections::HashMap;
use image::ImageReader;

/// Per-evaluation context handed to every processor.
#[derive(Clone, Copy)]
pub struct FrameCtx {
    /// Frame index on the global timeline.
    pub global: u32,
    /// Frame index relative to the node's own start_frame (0 at the node's first frame).
    pub local: u32,
}

/// `inputs[i]` is `None` when dependency `i` is not active at this frame.
/// Returning `None` means "this node produces nothing at this frame".
pub trait Processable {
    fn process(&self, ctx: FrameCtx, inputs: &[Option<FrameBuffer>]) -> Option<FrameBuffer>;
}

#[derive(Clone)]
pub struct FrameBuffer {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<[u8; 4]>,
}

impl FrameBuffer {
    /// Save as PNG (creates parent directories if needed).
    pub fn save_png(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        let raw: Vec<u8> = self.pixels.iter().flat_map(|p| p.iter().copied()).collect();
        let img = image::RgbaImage::from_raw(self.width, self.height, raw)
            .ok_or("pixel buffer size does not match width*height")?;
        img.save(path)?;
        Ok(())
    }
}

/// Helper: first input, if present and active.
fn first_input(inputs: &[Option<FrameBuffer>]) -> Option<&FrameBuffer> {
    inputs.get(0).and_then(|i| i.as_ref())
}

pub struct PassThrough; // stub processor for clips with no real effect yet
impl Processable for PassThrough {
    fn process(&self, _ctx: FrameCtx, inputs: &[Option<FrameBuffer>]) -> Option<FrameBuffer> {
        first_input(inputs).cloned()
    }
}

/// Generates synthetic gradient frames for testing.
/// In production, this would load from disk; for now it creates colored test patterns.
pub struct ClipSource {
    pub color: [u8; 3], // RGB base color
    pub width: u32,
    pub height: u32,
}

impl Processable for ClipSource {
    fn process(&self, ctx: FrameCtx, _inputs: &[Option<FrameBuffer>]) -> Option<FrameBuffer> {
        let mut pixels = Vec::with_capacity((self.width * self.height) as usize);
        
        for y in 0..self.height {
            for x in 0..self.width {
                // Simple gradient: vary by frame and position
                let frame_shift = (ctx.local as f32) / 100.0;
                let x_norm = (x as f32) / (self.width as f32);
                let y_norm = (y as f32) / (self.height as f32);
                
                let r = ((self.color[0] as f32 * (x_norm + frame_shift)).clamp(0.0, 255.0)) as u8;
                let g = ((self.color[1] as f32 * y_norm).clamp(0.0, 255.0)) as u8;
                let b = ((self.color[2] as f32 * (1.0 - x_norm)).clamp(0.0, 255.0)) as u8;
                
                pixels.push([r, g, b, 255]);
            }
        }

        Some(FrameBuffer {
            width: self.width,
            height: self.height,
            pixels,
        })
    }
}

pub struct Node {
    pub name: String,
    pub start_frame: u32,
    pub end_frame: u32,
    pub dependencies: Vec<String>,
    pub processor: Box<dyn Processable>,
}

/// Color correction: brightness, contrast, saturation
pub struct ColorCorrection {
    pub brightness: f32, // additive, -255..255
    pub contrast: f32,   // multiplicative, 0.5..2.0 typical
    pub saturation: f32, // 0.0 = grayscale, 1.0 = normal, 2.0 = super-saturated
}

impl Processable for ColorCorrection {
    fn process(&self, _ctx: FrameCtx, inputs: &[Option<FrameBuffer>]) -> Option<FrameBuffer> {
        let src = first_input(inputs)?;
        let mut out = src.clone();

        for px in out.pixels.iter_mut() {
            let (r, g, b, a) = (px[0] as f32, px[1] as f32, px[2] as f32, px[3]);

            // Brightness (additive)
            let r = (r + self.brightness).clamp(0.0, 255.0);
            let g = (g + self.brightness).clamp(0.0, 255.0);
            let b = (b + self.brightness).clamp(0.0, 255.0);

            // Contrast (multiplicative around midpoint)
            let midpoint = 128.0;
            let r = ((r - midpoint) * self.contrast + midpoint).clamp(0.0, 255.0);
            let g = ((g - midpoint) * self.contrast + midpoint).clamp(0.0, 255.0);
            let b = ((b - midpoint) * self.contrast + midpoint).clamp(0.0, 255.0);

            // Saturation (relative to luminance)
            let luminance = 0.299 * r + 0.587 * g + 0.114 * b;
            let r = (luminance + (r - luminance) * self.saturation).clamp(0.0, 255.0);
            let g = (luminance + (g - luminance) * self.saturation).clamp(0.0, 255.0);
            let b = (luminance + (b - luminance) * self.saturation).clamp(0.0, 255.0);

            px[0] = r as u8;
            px[1] = g as u8;
            px[2] = b as u8;
            px[3] = a;
        }
        Some(out)
    }
}

/// Blend two layers with a specified blend mode
pub struct Blend {
    pub mode: BlendMode,
    pub opacity: f32, // 0.0..1.0
}

#[derive(Clone, Copy)]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
}

impl Processable for Blend {
    fn process(&self, _ctx: FrameCtx, inputs: &[Option<FrameBuffer>]) -> Option<FrameBuffer> {
        if inputs.len() < 2 {
            panic!("Blend requires exactly 2 inputs (background, foreground)");
        }

        // Layer inactive at this frame -> pass the other layer through untouched.
        let (bg, fg) = match (&inputs[0], &inputs[1]) {
            (Some(bg), Some(fg)) => (bg, fg),
            (Some(bg), None) => return Some(bg.clone()),
            (None, Some(fg)) => return Some(fg.clone()),
            (None, None) => return None,
        };

        if bg.width != fg.width || bg.height != fg.height {
            panic!(
                "Blend: inputs must have same dimensions (bg {}x{}, fg {}x{})",
                bg.width, bg.height, fg.width, fg.height
            );
        }

        let mut out = bg.clone();
        let opacity = self.opacity.clamp(0.0, 1.0);

        for i in 0..out.pixels.len() {
            let bg_px = bg.pixels[i];
            let fg_px = fg.pixels[i];

            let bg_rgb = [bg_px[0] as f32 / 255.0, bg_px[1] as f32 / 255.0, bg_px[2] as f32 / 255.0];
            let fg_rgb = [fg_px[0] as f32 / 255.0, fg_px[1] as f32 / 255.0, fg_px[2] as f32 / 255.0];
            let fg_alpha = (fg_px[3] as f32 / 255.0) * opacity;

            let blended = match self.mode {
                BlendMode::Normal => fg_rgb,
                BlendMode::Multiply => [
                    bg_rgb[0] * fg_rgb[0],
                    bg_rgb[1] * fg_rgb[1],
                    bg_rgb[2] * fg_rgb[2],
                ],
                BlendMode::Screen => [
                    1.0 - (1.0 - bg_rgb[0]) * (1.0 - fg_rgb[0]),
                    1.0 - (1.0 - bg_rgb[1]) * (1.0 - fg_rgb[1]),
                    1.0 - (1.0 - bg_rgb[2]) * (1.0 - fg_rgb[2]),
                ],
                BlendMode::Overlay => {
                    let overlay_component = |bg: f32, fg: f32| {
                        if bg < 0.5 {
                            2.0 * bg * fg
                        } else {
                            1.0 - 2.0 * (1.0 - bg) * (1.0 - fg)
                        }
                    };
                    [
                        overlay_component(bg_rgb[0], fg_rgb[0]),
                        overlay_component(bg_rgb[1], fg_rgb[1]),
                        overlay_component(bg_rgb[2], fg_rgb[2]),
                    ]
                }
            };

            let result = [
                bg_rgb[0] * (1.0 - fg_alpha) + blended[0] * fg_alpha,
                bg_rgb[1] * (1.0 - fg_alpha) + blended[1] * fg_alpha,
                bg_rgb[2] * (1.0 - fg_alpha) + blended[2] * fg_alpha,
            ];

            out.pixels[i] = [
                (result[0] * 255.0).round() as u8,
                (result[1] * 255.0).round() as u8,
                (result[2] * 255.0).round() as u8,
                std::cmp::max(bg_px[3], fg_px[3]),
            ];
        }
        Some(out)
    }
}

/// Simple box blur: average each pixel with its neighbors
pub struct BoxBlur {
    pub radius: u32,
}

impl Processable for BoxBlur {
    fn process(&self, _ctx: FrameCtx, inputs: &[Option<FrameBuffer>]) -> Option<FrameBuffer> {
        let src = first_input(inputs)?;
        let mut out = src.clone();

        let r = self.radius as i32;
        let width = src.width as i32;
        let height = src.height as i32;

        for y in 0..height {
            for x in 0..width {
                let mut sum = [0u32; 4];
                let mut count = 0u32;

                for dy in -r..=r {
                    for dx in -r..=r {
                        let nx = (x + dx).clamp(0, width - 1);
                        let ny = (y + dy).clamp(0, height - 1);
                        let idx = (ny * width + nx) as usize;

                        let px = src.pixels[idx];
                        for c in 0..4 {
                            sum[c] += px[c] as u32;
                        }
                        count += 1;
                    }
                }

                let idx = (y * width + x) as usize;
                for c in 0..4 {
                    out.pixels[idx][c] = (sum[c] / count) as u8;
                }
            }
        }
        Some(out)
    }
}

pub struct GraphBuilder {
    graph: HashMap<String, Node>,
}

impl GraphBuilder {
    pub fn new() -> Self {
        Self { graph: HashMap::new() }
    }

    pub fn add_clip(mut self, name: &str, start: u32, end: u32, processor: Box<dyn Processable>) -> Self {
        self.graph.insert(
            name.to_string(),
            Node {
                name: name.to_string(),
                start_frame: start,
                end_frame: end,
                dependencies: vec![],
                processor,
            },
        );
        self
    }

    pub fn depends_on(mut self, node: &str, dep: &str) -> Self {
        self.graph
            .get_mut(node)
            .unwrap_or_else(|| panic!("depends_on: node '{node}' has not been added"))
            .dependencies
            .push(dep.to_string());
        self
    }

    pub fn build(self) -> (HashMap<String, Node>, Vec<String>) {
        let order = Self::topo_sort(&self.graph);
        (self.graph, order)
    }

    /// Kahn's algorithm: repeatedly peel off nodes with in-degree 0.
    /// If we run out of such nodes before consuming the whole graph,
    /// there's a cycle among whatever's left.
    fn topo_sort(graph: &HashMap<String, Node>) -> Vec<String> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut dependents: HashMap<String, Vec<String>> = HashMap::new();

        for (name, node) in graph {
            in_degree.entry(name.clone()).or_insert(0);
            for dep in &node.dependencies {
                if !graph.contains_key(dep) {
                    panic!("Node '{}' depends on undeclared node '{}'", name, dep);
                }
                *in_degree.entry(name.clone()).or_insert(0) += 1;
                dependents.entry(dep.clone()).or_default().push(name.clone());
            }
        }

        let mut ready: Vec<String> = in_degree
            .iter()
            .filter(|&(_, &deg)| deg == 0)
            .map(|(name, _)| name.clone())
            .collect();
        ready.sort();

        let mut order = Vec::with_capacity(graph.len());
        let mut queue: std::collections::VecDeque<String> = ready.into();

        while let Some(name) = queue.pop_front() {
            order.push(name.clone());
            if let Some(children) = dependents.get(&name) {
                let mut newly_ready = Vec::new();
                for child in children {
                    let deg = in_degree.get_mut(child).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        newly_ready.push(child.clone());
                    }
                }
                newly_ready.sort();
                for n in newly_ready {
                    queue.push_back(n);
                }
            }
        }

        if order.len() != graph.len() {
            let stuck: Vec<&String> = graph.keys().filter(|k| !order.contains(k)).collect();
            panic!("Cycle detected in dependency graph — could not order: {:?}", stuck);
        }

        order
    }
}