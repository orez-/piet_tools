use std::env;

fn main() -> Result<(), String> {
    let args = env::args().collect::<Vec<_>>();
    let (filename, codel_size) = match args.as_slice() {
        [_, f, c] => (f, c),
        _ => { return Err("usage: pieti filename codel-size".to_string()); },
    };
    let codel_size = codel_size.parse()
        .map_err(|_| "codel-size must be an integer".to_string())?;
    if codel_size == 0 {
        return Err("codel-size must be non-zero".to_string())
    }

    let piet = piet_tools::load(filename, codel_size)?;
    piet.execute().run();
    println!();
    Ok(())
}
