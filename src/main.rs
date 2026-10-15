mod commands;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    commands::run().map_err(|error| color_eyre::eyre::eyre!("{error:#}"))
}
