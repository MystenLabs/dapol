use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short, long)]
    pub name: String,

    #[arg(short, long, default_value_t = 1)]
    pub count: u8,
}
