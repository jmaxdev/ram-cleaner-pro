use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "RAM Purger Pro",
    author = "jmaxdev",
    version = "1.0.0",
    about = "High-performance Windows NT memory purging and optimization utility"
)]
pub struct CliArgs {
    #[arg(short = 'p', long = "purge-now")]
    pub purge_now: bool,

    #[arg(short = 'd', long = "daemon")]
    pub daemon: bool,

    #[arg(short = 'g', long = "gui")]
    pub gui: bool,

    #[arg(short = 's', long = "status")]
    pub status: bool,

    #[arg(long = "threshold")]
    pub threshold: Option<f32>,

    #[arg(long = "interval")]
    pub interval: Option<u64>,
}
