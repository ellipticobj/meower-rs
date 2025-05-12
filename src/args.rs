use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short, long)]
    pub add: Option<Vec<String>>,

    #[arg(short = 'd', long = "dry-run")]
    pub dryrun: bool,

    #[arg(name = "commitmessage")]
    pub commitmessage: Option<String>,
}

