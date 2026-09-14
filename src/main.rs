use std::collections::HashMap;

#[derive(Debug, Clone)]
struct Node {
    name: String,
    // --- New Timeline Attributes ---
    start_frame: u32,
    end_frame: u32,
    dependencies: Vec<String>,
}

fn main() {
    // 1. Define our asset timeline/graph
    let mut graph: HashMap<String, Node> = HashMap::new();

    // Background clip runs from frame 0 to 100
    graph.insert("Load_Background_Clip".to_string(), Node {
        name: "Load_Background_Clip".to_string(),
        start_frame: 0,
        end_frame: 100,
        dependencies: vec![],
    });

    // An adjustment layer/effect clip that only appears between frames 30 and 60
    graph.insert("Apply_Flash_Blur".to_string(), Node {
        name: "Apply_Flash_Blur".to_string(),
        start_frame: 30,
        end_frame: 60,
        dependencies: vec![], 
    });

    // The compositor layer merges everything. It depends on both clips, 
    // but its actual lifespan matches the background.
    graph.insert("Composite_Layers".to_string(), Node {
        name: "Composite_Layers".to_string(),
        start_frame: 0,
        end_frame: 100,
        dependencies: vec!["Load_Background_Clip".to_string(), "Apply_Flash_Blur".to_string()], 
    });

    println!("--- Initializing ProseCut Timeline & DAG Engine ---");
    
    // 2. Simulating a 5-frame jump sequence to test timeline boundaries
    // Instead of looping all 100 frames, let's sample critical points:
    let sample_frames = vec![10, 45, 80]; 
    let execution_order = vec!["Load_Background_Clip", "Apply_Flash_Blur", "Composite_Layers"];

    for current_frame in sample_frames {
        println!("\n🎬 [RENDERING FRAME {}]", current_frame);

        for task_name in &execution_order {
            let node = &graph[*task_name];

            // --- TIMELINE FILTER ---
            // Check if this node is even alive on the current frame
            if current_frame >= node.start_frame && current_frame <= node.end_frame {
                println!("  ↳ Task '{}' is ACTIVE", node.name);
                
                // Evaluate dependencies
                if node.dependencies.is_empty() {
                    println!("       -> Executing baseline track.");
                } else {
                    // Check which dependencies are ALSO active right now
                    let active_deps: Vec<&String> = node.dependencies
                        .iter()
                        .filter(|dep| {
                            let dep_node = &graph[*dep];
                            current_frame >= dep_node.start_frame && current_frame <= dep_node.end_frame
                        })
                        .collect();

                    if active_deps.is_empty() {
                        println!("       -> Dependencies configured, but none are active on this frame. Compositing baseline only.");
                    } else {
                        println!("       -> Wait! Evaluation includes active dependencies: {:?}", active_deps);
                        println!("       -> Compositing layers together!");
                    }
                }
            } else {
                println!("  ↳ Task '{}' is INACTIVE (Skipping)", node.name);
            }
        }
    }
}