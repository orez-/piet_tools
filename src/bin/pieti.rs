use std::env;

fn main() -> Result<(), String> {
    let args = env::args().collect::<Vec<_>>();
    let filename = match args.as_slice() {
        [_, filename, ..] => filename,
        _ => { return Err("usage: pieti filename".to_string()); },
    };

    let piet = piet_tools::load(filename, 5)?;
    piet.execute().run();
    println!();
    Ok(())
}
