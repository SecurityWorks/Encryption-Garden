use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;

use age::{Decryptor, Encryptor};
use age::x25519::{Identity, Recipient};

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};

#[derive(Parser)]
#[command(name = "a", version, author, about = "🔐 Production-safe age file encryptor.")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Encrypt {
        #[arg(short, long)]
        input: PathBuf,

        #[arg(short, long)]
        output: PathBuf,

        #[arg(short, long)]
        recipient: String,
    },

    Decrypt {
        #[arg(short, long)]
        input: PathBuf,

        #[arg(short, long)]
        output: PathBuf,

        #[arg(short, long)]
        key: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Encrypt { input, output, recipient } => {
            encrypt(input, output, &recipient)
        }
        Commands::Decrypt { input, output, key } => {
            decrypt(input, output, key)
        }
    }
}

fn encrypt(input: PathBuf, output: PathBuf, recipient_str: &str) -> Result<()> {
    let recipient = recipient_str
        .parse::<Recipient>()
        .map_err(|_| anyhow!("❌ Invalid recipient key"))?;
    
    let recipients: Vec<Box<dyn age::Recipient>> = vec![Box::new(recipient)];
    let encryptor = Encryptor::with_recipients(recipients.iter().map(|r| r.as_ref()))
        .expect("No recipient provided");

    let infile = File::open(&input)?;
    let outfile = File::create(&output)?;
    let metadata = infile.metadata()?;

    let pb = progress_bar(metadata.len());
    let mut reader = pb.wrap_read(BufReader::new(infile));
    let mut writer = encryptor.wrap_output(BufWriter::new(outfile))?;

    std::io::copy(&mut reader, &mut writer)?;
    writer.finish()?;
    pb.finish_with_message("✅ Encrypted");
    Ok(())
}

fn decrypt(input: PathBuf, output: PathBuf, key_path: PathBuf) -> Result<()> {
    let key_content = std::fs::read_to_string(&key_path)?;
    let identity_line = key_content
        .lines()
        .find(|line| line.starts_with("AGE-SECRET-KEY"))
        .ok_or_else(|| anyhow!("❌ No AGE-SECRET-KEY found"))?;

    let identity = identity_line
        .parse::<Identity>()
        .map_err(|_| anyhow!("❌ Invalid identity key"))?;

    let infile = File::open(&input)?;
    let outfile = File::create(&output)?;
    let metadata = infile.metadata()?;

    let pb = progress_bar(metadata.len());
    let decryptor = Decryptor::new(BufReader::new(pb.wrap_read(infile)))?;

    let identities: Vec<Box<dyn age::Identity>> = vec![Box::new(identity)];
    let mut reader = decryptor.decrypt(identities.iter().map(|i| i.as_ref()))?;

    let mut writer = BufWriter::new(outfile);
    std::io::copy(&mut reader, &mut writer)?;
    pb.finish_with_message("✅ Decrypted");
    Ok(())
}

fn progress_bar(len: u64) -> ProgressBar {
    let pb = ProgressBar::new(len);
    pb.set_style(
        ProgressStyle::with_template(
            "[{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})",
        )
        .unwrap(),
    );
    pb
}

