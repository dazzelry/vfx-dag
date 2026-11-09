use crate::dsl::Operation;

pub fn compile_filtergraph(operations: &[Operation]) -> String {
    let mut filters = Vec::new();

    for operation in operations {
        match operation {
            Operation::Scale { width, height } => {
                filters.push(format!("scale={width}:{height}"));
            }

            Operation::HorizontalFlip => {
                filters.push("hflip".to_string());
            }

            Operation::Blur { radius } if *radius > 0 => {
                filters.push(format!("boxblur={radius}:1"));
            }

            Operation::Blur { .. } => {}

            Operation::Color {
                contrast,
                saturation,
            } => {
                filters.push(format!(
                    "eq=contrast={contrast}:saturation={saturation}"
                ));
            }
        }
    }

    if filters.is_empty() {
        "null".to_string()
    } else {
        filters.join(",")
    }
}