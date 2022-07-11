use piet_tools::PietCode;
use std::env;

fn parse_codel_size(arg: &str) -> Result<u32, String> {
    let codel_size = arg.parse()
        .map_err(|_| "codel-size must be an integer".to_string())?;
    if codel_size == 0 {
        return Err("codel-size must be non-zero".to_string())
    }
    Ok(codel_size)
}

fn parse_run_args(args: &[&str]) -> Result<(), String> {
    let (filename, codel_size) = match args {
        [f, c] => (f, c),
        _ => { return Err("usage: pietasm run filename codel-size".to_string()); }
    };

    let codel_size = parse_codel_size(codel_size)?;
    let (piet, _) = build(filename, codel_size)?;
    piet.execute().run();
    println!();
    Ok(())
}

fn parse_build_args(args: &[&str]) -> Result<(), String> {
    let (filename, codel_size) = match args {
        [f, c] => (f, c),
        _ => { return Err("usage: pietasm build filename codel-size".to_string()); }
    };

    let codel_size = parse_codel_size(codel_size)?;
    let (_, out_filename) = build(filename, codel_size)?;

    println!("File saved to {out_filename}");
    Ok(())
}

fn build(filename: &str, codel_size: u32) -> Result<(PietCode, String), String> {
    let piet = piet_tools::asm::load(filename)?;
    let out_filename = format!("{filename}.png");
    piet_tools::save(&piet, &out_filename, codel_size)
        .map_err(|e| e.to_string())?;
    Ok((piet, out_filename))
}

fn main() -> Result<(), String> {
    let owned_args: Vec<_> = env::args().collect();
    let args: Vec<_> = owned_args.iter().map(|x| x.as_str()).collect();
    match args.as_slice() {
        [_, "build", rest @ ..] => parse_build_args(rest),
        [_, "run", rest @ ..] => parse_run_args(rest),
        _ => Err("usage: pietasm [build | run] [args]".to_string()),
    }
}
