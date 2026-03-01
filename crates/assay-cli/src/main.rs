use clap::Parser;

#[derive(Parser)]
#[command(
    name = "assay",
    version,
    about = "Agentic development kit with spec-driven workflows"
)]
struct Cli {}

fn main() {
    let _cli = Cli::parse();
    println!("assay {}", env!("CARGO_PKG_VERSION"));
}
