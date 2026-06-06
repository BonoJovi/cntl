mod cli;
mod object;

fn main() -> anyhow::Result<()> {
    cli::run()
}
