use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None, disable_version_flag = true)]
pub struct Args {
    #[arg(short, long)]
    pub add: Option<Vec<String>>,

    #[arg(short = 'd', long = "dry-run")]
    pub dryrun: bool,

    #[arg(name = "commitmessage")]
    pub commitmessage: Option<String>,

    #[arg(long = "version", short = 'V')]
    pub version: bool,
}
