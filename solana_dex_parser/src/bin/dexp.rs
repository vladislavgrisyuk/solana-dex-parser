use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use serde_json::Value;
use solana_dex_parser::types::FromJsonValue;
use solana_dex_parser::{DexParser, ParseConfig, SolanaBlock, SolanaTransaction};

#[derive(Parser)]
#[command(author, version, about = "Parse Solana DEX transactions", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse a single transaction JSON file
    ParseTx {
        /// Path to a JSON file containing a transaction
        #[arg(long)]
        file: PathBuf,
        /// Output mode
        #[arg(long, value_enum, default_value = "all")]
        mode: TxMode,
    },
    /// Parse a block JSON file
    ParseBlock {
        /// Path to a JSON file containing block information
        #[arg(long)]
        file: PathBuf,
        /// Block parsing mode
        #[arg(long, value_enum, default_value = "parsed")]
        mode: BlockMode,
    },
}

#[derive(Clone, ValueEnum)]
enum TxMode {
    All,
    Trades,
    Liquidity,
    Transfers,
}

#[derive(Clone, ValueEnum)]
enum BlockMode {
    Raw,
    Parsed,
}

fn read_json(file: &PathBuf) -> Result<Value> {
    let data = fs::read_to_string(file).with_context(|| format!("failed to read {:?}", file))?;
    serde_json::from_str(&data).with_context(|| format!("failed to parse JSON in {:?}", file))
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let parser = DexParser::new();
    let config = ParseConfig::default();

    match cli.command {
        Commands::ParseTx { file, mode } => {
            let value = read_json(&file)?;
            let tx =
                SolanaTransaction::from_value(&value, &config).map_err(|err| anyhow!("{err}"))?;
            let output = match mode {
                TxMode::All => serde_json::to_value(parser.parse_all(tx, Some(config)))?,
                TxMode::Trades => serde_json::to_value(parser.parse_trades(tx, Some(config)))?,
                TxMode::Liquidity => {
                    serde_json::to_value(parser.parse_liquidity(tx, Some(config)))?
                }
                TxMode::Transfers => {
                    serde_json::to_value(parser.parse_transfers(tx, Some(config)))?
                }
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        Commands::ParseBlock { file, mode } => {
            let value = read_json(&file)?;
            match mode {
                BlockMode::Raw => {
                    let txs: Vec<Value> = serde_json::from_value(value)?;
                    let result = parser.parse_block_raw(&txs, Some(config))?;
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
                BlockMode::Parsed => {
                    let block: SolanaBlock = serde_json::from_value(value)?;
                    let result = parser.parse_block_parsed(&block, Some(config));
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
            }
        }
    }

    Ok(())
}
