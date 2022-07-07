use std::env;

fn main() -> Result<(), String> {
    let args = env::args().collect::<Vec<_>>();
    let filename = match args.as_slice() {
        [_, filename, ..] => filename,
        _ => { return Err("usage: pietasm filename".to_string()); },
    };

    let piet = piet_tools::asm::load(filename)?;
    println!("{piet:?}");
    Ok(())
}
