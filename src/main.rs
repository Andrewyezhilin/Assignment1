use std::{
    error::Error,
    fs::File,
    io::{Read, stdin},
    path::{Path, PathBuf},
};

use clap::Parser;
use ortalab::score;
use ortalib::Round;

#[derive(Parser)]
struct Opts {
    file: PathBuf,

    #[arg(long)]
    explain: bool,
}

#[cfg(not(tarpaulin_include))]
fn main() -> Result<(), Box<dyn Error>> {
    let opts = Opts::parse();
    let round = parse_round(&opts)?;
    let (chips, mult) = score(round);
    println!("{}", (chips * mult).floor());
    Ok(())
}

#[cfg(not(tarpaulin_include))]
fn parse_round(opts: &Opts) -> Result<Round, Box<dyn Error>> {
    let mut input = String::new();
    if opts.file == Path::new("-") {
        stdin().read_to_string(&mut input)?;
    } else {
        File::open(&opts.file)?.read_to_string(&mut input)?;
    }
    Ok(serde_yaml::from_str(&input)?)
}
