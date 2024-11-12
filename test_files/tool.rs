use std::{fs, env};
use anyhow::{bail, Result};
use bpe_rs::bpe;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 4 {
        bail!("Command should be: <executable> <encode/decode> <input> <output>");
    }

    let usage = args.get(1).unwrap();

    if usage != "decode" && usage != "encode" {
        bail!("Usage must be encode or decode");
    }


    let input_name = args.get(2).unwrap();
    let output_name = args.get(3).unwrap();
    let file = fs::read(input_name)?;

    let out = if usage == "decode" {
        bpe::decode(&file, bpe::DEFAULT_STACK_SIZE)
    } else {
        bpe::encode(&file)
    };

    fs::write(output_name, out)?;

    Ok(())
}
