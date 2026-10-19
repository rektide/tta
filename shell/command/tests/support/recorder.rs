//! Binary fixture: record exact process cwd, argv and a selected environment
//! value with length framing, so comparisons need no scripting language.
#![forbid(unsafe_code)]

use std::io::{self, Write};

fn main() -> io::Result<()> {
    let mut args = std::env::args_os().skip(1);
    let output = args.next().expect("recorder requires an output path");
    let mut file = std::fs::File::create(output)?;
    let values = [
        std::env::current_dir()?.into_os_string(),
        std::env::var_os("TTA_TEST_VALUE").unwrap_or_default(),
    ];
    for value in values.into_iter().chain(args) {
        let bytes = value.as_encoded_bytes();
        file.write_all(&(bytes.len() as u64).to_le_bytes())?;
        file.write_all(bytes)?;
    }
    Ok(())
}
