use std::env;

fn main() -> Result<(), String> {
    let args = env::args().collect::<Vec<_>>();
    let filename = match args.as_slice() {
        [_, filename, ..] => filename,
        _ => { return Err("usage: pietasm filename".to_string()); },
    };

    let piet = piet_tools::asm::load(filename)?;
    let out_filename = format!("{filename}.png");
    piet_tools::save(piet, &out_filename, 10)
        .map_err(|e| e.to_string())?;
    println!("File saved to {out_filename}");
    Ok(())
}
