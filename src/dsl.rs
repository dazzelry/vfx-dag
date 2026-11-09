
#[derive(Debug, Clone)]
pub enum Operation {
    Scale { width: u32, height: u32 },
    HorizontalFlip,
    Blur { radius: u32 },
    Color { contrast: f32, saturation: f32 },
}

#[derive(Debug, Clone)]
pub struct Script {
    pub source: String,
    pub operations: Vec<Operation>,
    pub output: String,
}

pub fn parse_script(text: &str) -> Result<Script, String> {
    let mut source: Option<String> = None;
    let mut operations = Vec::new();
    let mut output: Option<String> = None;

    for (index, raw_line) in text.lines().enumerate() {
        let line = raw_line.trim();
        let line_no = index + 1;

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut parts = line.split_whitespace();
        let command = parts.next().unwrap();

        let error = |msg: &str| {
            format!("Line {line_no}: {msg}")
        };

        match command {
            "source" => {
                if source.is_some() {
                    return Err(error("only one source is supported for now"));
                }

                let path = parts
                    .next()
                    .ok_or_else(|| error(r#"expected source "path.mp4""#))?;

                if !path.starts_with('"')
                    || !path.ends_with('"')
                    || path.len() < 3
                {
                    return Err(error(r#"path must be quoted: source "test.mp4""#));
                }

                if parts.next().is_some() {
                    return Err(error("unexpected arguments after source path"));
                }

                source = Some(path[1..path.len() - 1].to_string());
            }

            "scale" => {
                let width = parse_u32(parts.next(), line_no, "width")?;
                let height = parse_u32(parts.next(), line_no, "height")?;
                ensure_no_more(parts, line_no)?;

                if width == 0 || height == 0 {
                    return Err(error("scale dimensions must be positive"));
                }

                operations.push(Operation::Scale { width, height });
            }

            "hflip" => {
                ensure_no_more(parts, line_no)?;
                operations.push(Operation::HorizontalFlip);
            }

            "blur" => {
                let radius = parse_u32(parts.next(), line_no, "radius")?;
                ensure_no_more(parts, line_no)?;

                if radius > 20 {
                    return Err(error("blur radius must be between 0 and 20"));
                }

                operations.push(Operation::Blur { radius });
            }

            "color" => {
                let contrast = parse_f32(parts.next(), line_no, "contrast")?;
                let saturation = parse_f32(parts.next(), line_no, "saturation")?;
                ensure_no_more(parts, line_no)?;

                if !(0.0..=3.0).contains(&contrast)
                    || !(0.0..=3.0).contains(&saturation)
                {
                    return Err(error(
                        "contrast and saturation must be between 0 and 3",
                    ));
                }

                operations.push(Operation::Color {
                    contrast,
                    saturation,
                });
            }

            "output" => {
                if output.is_some() {
                    return Err(error("only one output declaration is supported"));
                }

                let path = parts
                    .next()
                    .ok_or_else(|| error(r#"expected output "path.mp4""#))?;

                if !path.starts_with('"')
                    || !path.ends_with('"')
                    || path.len() < 3
                {
                    return Err(error(
                        r#"output path must be quoted: output "final.mp4""#
                    ));
                }

                if parts.next().is_some() {
                    return Err(error("unexpected arguments after output path"));
                }

                output = Some(path[1..path.len() - 1].to_string());
            }

            _ => return Err(error(&format!("unknown operation '{command}'"))),
        }
    }

    let source = source.ok_or("Missing source declaration")?;
    let output = output.ok_or("Missing output declaration")?;

    Ok(Script {
        source,
        operations,
        output,
    })
    }

fn parse_u32(
    value: Option<&str>,
    line: usize,
    name: &str,
) -> Result<u32, String> {
    value
        .ok_or_else(|| format!("Line {line}: missing {name}"))?
        .parse()
        .map_err(|_| format!("Line {line}: invalid {name}"))
}

fn parse_f32(
    value: Option<&str>,
    line: usize,
    name: &str,
) -> Result<f32, String> {
    let parsed: f32 = value
        .ok_or_else(|| format!("Line {line}: missing {name}"))?
        .parse()
        .map_err(|_| format!("Line {line}: invalid {name}"))?;

    if !parsed.is_finite() {
        return Err(format!("Line {line}: {name} must be finite"));
    }

    Ok(parsed)
}

fn ensure_no_more<'a>(
    mut parts: impl Iterator<Item = &'a str>,
    line: usize,
) -> Result<(), String> {
    if parts.next().is_some() {
        Err(format!("Line {line}: too many arguments"))
    } else {
        Ok(())
    }
}