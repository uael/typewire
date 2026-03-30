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

  /// Keep the `typewire_schemas` section in the binary after extraction
  #[arg(long)]
  no_strip: bool,
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

  let schemas = typewire_schema::decode::parse_section(schema_bytes)?;

  let output = match cli.lang {
    Lang::Typescript => typewire_schema::typescript::generate(&schemas),
  };

  match cli.output {
    Some(path) => std::fs::write(&path, &output)?,
    None => print!("{output}"),
  }

  if !cli.no_strip {
    if obj.format() == object::BinaryFormat::Wasm {
      let mut module = walrus::Module::from_buffer(&data)?;
      let ids: Vec<_> =
        module.customs.iter().filter(|(_, s)| s.name() == SECTION_NAME).map(|(id, _)| id).collect();
      for id in ids {
        module.customs.delete(id);
      }
      let stripped = module.emit_wasm();
      // Write to a temporary file then rename for crash-safe replacement.
      let tmp = cli.binary.with_extension("typewire-tmp");
      std::fs::write(&tmp, stripped)?;
      std::fs::rename(&tmp, &cli.binary)?;
    } else {
      eprintln!(
        "warning: stripping is only supported for WASM binaries, skipping for {} (use --no-strip to silence)",
        cli.binary.display()
      );
    }
  }

  Ok(())
}
