fn main() -> Result<(), String> {
    let piet = piet_tools::load("examples/hw6_big.png", 5)?;
    piet.execute().run();
    Ok(())
}
