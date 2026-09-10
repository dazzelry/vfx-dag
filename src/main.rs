use std::collections::HashMap;

// 1. Define what a "Node" (Task) looks like in our DAG
#[derive(Debug, Clone)]
struct Node {
    name: String,
    dependencies: Vec<String>,
}

fn main() {
    // 2. Create a list of video editing tasks
    let mut graph: HashMap<String, Node> = HashMap::new();

    graph.insert("Load_Clip".to_string(), Node {
        name: "Load_Clip".to_string(),
        dependencies: vec![], // No dependencies, can start immediately
    });

    graph.insert("Apply_Blur".to_string(), Node {
        name: "Apply_Blur".to_string(),
        dependencies: vec![], // No dependencies, can start immediately
    });

    graph.insert("Composite_Layers".to_string(), Node {
        name: "Composite_Layers".to_string(),
        // This task CANNOT run until the first two are finished!
        dependencies: vec!["Load_Clip".to_string(), "Apply_Blur".to_string()], 
    });

    // 3. Simulating a basic execution order
    println!("--- Initializing ProseCut DAG Engine ---");
    
    // In a real engine, a "Topological Sort" algorithm calculates this order.
    // For day one, we will just simulate running them in dependency order:
    let execution_order = vec!["Load_Clip", "Apply_Blur", "Composite_Layers"];

    for task_name in execution_order {
        let node = &graph[task_name];
        println!("Checking dependencies for: {}...", node.name);
        
        if node.dependencies.is_empty() {
            println!("   -> Success: No dependencies. Executing task!");
        } else {
            println!("   -> Wait! Checking if {:?} are done...", node.dependencies);
            println!("   -> Success: Dependencies cleared. Executing task!");
        }
    }
}
