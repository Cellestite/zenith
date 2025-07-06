use zenith::launch_default;

fn main() {
    let engine_loop = pollster::block_on(launch_default())
        .expect("Failed to create zenith engine loop!");

    engine_loop
        .run()
        .expect("Failed to run zenith engine loop!");
}