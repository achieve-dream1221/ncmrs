use clap::Parser;
/// ncm 解码器
#[derive(Parser, Debug)]
#[command(infer_subcommands = true, infer_long_args = true)]
#[command(arg_required_else_help = true)]
pub struct Cli {
    #[arg(short, long, help = "输入ncm文件/ncm目录")]
    pub input: String,
    #[arg(short, long, help = "输出目录")]
    pub output: String,
    #[command(flatten)]
    pub verbose: clap_verbosity_flag::Verbosity,
}
