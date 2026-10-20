use std::io::{self, BufReader};

use analysis::Analyzer;
use ipc::{Lines, serve};

fn main() -> io::Result<()> {
    let mut transport = Lines::new(BufReader::new(io::stdin().lock()), io::stdout().lock());
    serve(&mut Analyzer::new(), &mut transport)
}
