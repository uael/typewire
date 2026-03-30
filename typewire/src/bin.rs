use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use object::{Object, ObjectSection};
use typewire_schema::coded::SECTION_NAME;

#[derive(Parser)]
#[command(name = "typewire", about = "Generate bindings from compiled Typewire schemas")]
struct Cli {
  /// Path to the compiled binary (WASM, ELF, or Mach-O)
  binary: PathBuf,

  /// Target language for binding generation
  #[arg(short, long, default_value = "typescript")]
  lang: Lang,

  /// Output file (stdout if omitted)
  #[arg(short, long)]
  output: Option<PathBuf>,

  /// Strip the `typewire_schemas` section from the binary after extraction
  #[arg(long)]
  strip: bool,
}

#[derive(Clone, ValueEnum)]
enum Lang {
  Typescript,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let cli = Cli::parse();
  let data = std::fs::read(&cli.binary)?;
  let obj = object::File::parse(&*data)?;
  let section = obj
    .section_by_name(SECTION_NAME)
    .ok_or_else(|| format!("no {SECTION_NAME} section found in {}", cli.binary.display()))?;
  let schema_bytes = section.data()?;

  let output = match cli.lang {
    Lang::Typescript => typewire_schema::typescript::generate(schema_bytes)?,
  };

  match cli.output {
    Some(path) => std::fs::write(&path, &output)?,
    None => print!("{output}"),
  }

  if cli.strip {
    let mut module = walrus::Module::from_buffer(&data)?;
    let ids: Vec<_> =
      module.customs.iter().filter(|(_, s)| s.name() == SECTION_NAME).map(|(id, _)| id).collect();
    for id in ids {
      module.customs.delete(id);
    }
    let stripped = module.emit_wasm();
    std::fs::write(&cli.binary, stripped)?;
  }

  Ok(())
}
