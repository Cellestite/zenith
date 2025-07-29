use std::env;
use std::time::Instant;
use zenith::{GltfLoader};

fn main() {

    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <gltf_file_path>", args[0]);
        eprintln!("Example: {} content/mesh/cerberus/scene.gltf", args[0]);
        std::process::exit(1);
    }

    let gltf_path = &args[1];
    println!("Benchmarking GLTF loading performance for: {}", gltf_path);
    println!("========================================");

    // Warm up the disk cache by doing one read
    println!("Warming up disk cache...");
    let _ = std::fs::read(gltf_path).unwrap();

    const NUM_ITERATIONS: usize = 5;
    
    // Benchmark traditional file loading
    println!("\nTesting traditional file loading ({} iterations):", NUM_ITERATIONS);
    let mut traditional_times = Vec::new();
    
    for i in 1..=NUM_ITERATIONS {
        let start = Instant::now();
        let result = GltfLoader::load_from_file(gltf_path);
        let duration = start.elapsed();
        
        match result {
            Ok(model) => {
                traditional_times.push(duration);
                println!("  Iteration {}: {:?} ({} meshes, {} materials)", 
                    i, duration, model.meshes.len(), model.materials.materials.len());
            }
            Err(e) => {
                eprintln!("  Iteration {} failed: {}", i, e);
                return;
            }
        }
    }

    // Benchmark memory-mapped loading
    println!("\nTesting memory-mapped loading ({} iterations):", NUM_ITERATIONS);
    let mut mmap_times = Vec::new();
    
    for i in 1..=NUM_ITERATIONS {
        let start = Instant::now();
        let result = GltfLoader::load_from_file_mmap(gltf_path);
        let duration = start.elapsed();
        
        match result {
            Ok(model) => {
                mmap_times.push(duration);
                println!("  Iteration {}: {:?} ({} meshes, {} materials)", 
                    i, duration, model.meshes.len(), model.materials.materials.len());
            }
            Err(e) => {
                eprintln!("  Iteration {} failed: {}", i, e);
                return;
            }
        }
    }

    // Calculate averages
    let avg_traditional = traditional_times.iter().sum::<std::time::Duration>() / traditional_times.len() as u32;
    let avg_mmap = mmap_times.iter().sum::<std::time::Duration>() / mmap_times.len() as u32;

    // Results
    println!("\n========================================");
    println!("BENCHMARK RESULTS:");
    println!("Traditional loading average: {:?}", avg_traditional);
    println!("Memory-mapped loading average: {:?}", avg_mmap);
    
    let speedup = avg_traditional.as_secs_f64() / avg_mmap.as_secs_f64();
    if speedup > 1.0 {
        println!("Memory-mapped loading is {:.2}x FASTER", speedup);
    } else {
        println!("Traditional loading is {:.2}x faster", 1.0 / speedup);
    }
    
    let time_saved = avg_traditional.saturating_sub(avg_mmap);
    println!("Time saved per load: {:?}", time_saved);
    
    // File size info
    if let Ok(metadata) = std::fs::metadata(gltf_path) {
        let file_size = metadata.len();
        println!("File size: {} bytes ({:.2} MB)", file_size, file_size as f64 / 1024.0 / 1024.0);
        
        let traditional_throughput = file_size as f64 / avg_traditional.as_secs_f64() / 1024.0 / 1024.0;
        let mmap_throughput = file_size as f64 / avg_mmap.as_secs_f64() / 1024.0 / 1024.0;
        
        println!("Traditional throughput: {:.2} MB/s", traditional_throughput);
        println!("Memory-mapped throughput: {:.2} MB/s", mmap_throughput);
    }
}