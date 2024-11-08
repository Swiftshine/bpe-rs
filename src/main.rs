use std::{fs, env};
use anyhow::{bail, Result};
use bpe_rs::bpe;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        bail!("Usage should be: <executable> <input> <output>");
    }

    let input_name = args.get(1).unwrap();
    let output_name = args.get(2).unwrap();
    let file = fs::read(input_name)?;
    let out = bpe::decode(&file, bpe::DEFAULT_STACK_SIZE);

    fs::write(output_name, out)?;

    Ok(())
}
