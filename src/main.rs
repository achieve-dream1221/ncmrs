use crate::cli::Cli;
use crate::decoder::{decode_ncm, get_ncm_files};
use anyhow::Result;
use clap::Parser;
use indicatif::ProgressBar;
use std::path::Path;
use tracing_subscriber::fmt::time::LocalTime;

mod cli;
mod decoder;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    tracing_subscriber::fmt::fmt()
        .with_max_level(cli.verbose)
        .with_timer(LocalTime::rfc_3339())
        .init();
    let path = Path::new(&cli.input);
    if path.is_file() {
        decode_ncm(path, &cli.output).await?;
    } else {
        let pb = ProgressBar::new(0).with_style(
            indicatif::ProgressStyle::with_template(
                "[{elapsed_precise}] {wide_bar:.cyan/blue} {pos:>7}/{len:7} {msg}",
            )?
            .progress_chars("#>-"),
        );
        let ncm_files = get_ncm_files(path).await?;
        pb.set_length(ncm_files.len() as u64);
        for ncm_file in ncm_files {
            decode_ncm(&ncm_file, &cli.output).await?;
            pb.inc(1);
        }
        pb.finish_with_message("解码完成");
    }
    Ok(())
}
