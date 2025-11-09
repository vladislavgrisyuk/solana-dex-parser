use std::collections::HashMap;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use crate::config::ParseConfig;

/// Representation of a token amount inside a transaction.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TokenAmount {
    pub mint: String,
    pub amount: u64,
    pub decimals: u8,
}

impl TokenAmount {
    pub fn new(mint: impl Into<String>, amount: u64, decimals: u8) -> Self {
        Self {
            mint: mint.into(),
            amount,
            decimals,
        }
    }
}

impl Default for TokenAmount {
    fn default() -> Self {
        Self {
            mint: "SOL".to_string(),
            amount: 0,
            decimals: 9,
        }
    }
}

/// Token balance change helper struct used for SOL/token deltas.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BalanceChange {
    pub pre: i128,
    pub post: i128,
    pub change: i128,
}

/// Execution status for a Solana transaction.
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransactionStatus {
    #[serde(alias = "UNKNOWN")]
    Unknown,
    Success,
    Failed,
}

impl Default for TransactionStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Minimal instruction representation with bookkeeping indices.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClassifiedInstruction {
    pub program_id: String,
    pub outer_index: usize,
    pub inner_index: Option<usize>,
    pub data: SolanaInstruction,
}

/// Basic representation of a Solana instruction.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SolanaInstruction {
    pub program_id: String,
    pub accounts: Vec<String>,
    pub data: String,
}

/// Transfer data emitted by the meta simulation.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TransferData {
    pub program_id: String,
    pub from: String,
    pub to: String,
    pub amount: TokenAmount,
    pub idx: String,
}

/// High level trade information extracted from a transaction.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TradeInfo {
    pub program_id: String,
    pub amm: String,
    pub signature: String,
    pub idx: String,
    pub in_amount: TokenAmount,
    pub out_amount: TokenAmount,
    pub fee: Option<TokenAmount>,
}

/// High level liquidity pool event (add/remove liquidity etc.).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PoolEvent {
    pub program_id: String,
    pub event_type: String,
    pub mint_a: String,
    pub mint_b: String,
    pub liquidity: u64,
    pub signature: String,
    pub idx: String,
}

/// Meme/launch events emitted by platforms such as Pumpfun.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MemeEvent {
    pub program_id: String,
    pub event_type: String,
    pub signature: String,
    pub description: String,
}

/// Additional context information about the parsed transaction.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DexInfo {
    pub program_id: Option<String>,
    pub amm: Option<String>,
}

/// Aggregated parsing result returned by the Rust parser.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ParseResult {
    pub state: bool,
    #[serde(default)]
    pub fee: TokenAmount,
    #[serde(default)]
    pub aggregate_trade: Option<TradeInfo>,
    #[serde(default)]
    pub trades: Vec<TradeInfo>,
    #[serde(default)]
    pub liquidities: Vec<PoolEvent>,
    #[serde(default)]
    pub transfers: Vec<TransferData>,
    #[serde(default)]
    pub sol_balance_change: Option<BalanceChange>,
    #[serde(default)]
    pub token_balance_change: HashMap<String, BalanceChange>,
    #[serde(default)]
    pub meme_events: Vec<MemeEvent>,
    #[serde(default)]
    pub slot: u64,
    #[serde(default)]
    pub timestamp: u64,
    #[serde(default)]
    pub signature: String,
    #[serde(default)]
    pub signer: Vec<String>,
    #[serde(default)]
    pub compute_units: u64,
    #[serde(default)]
    pub tx_status: TransactionStatus,
    #[serde(default)]
    pub msg: Option<String>,
}

impl ParseResult {
    pub fn new() -> Self {
        Self {
            state: true,
            fee: TokenAmount::default(),
            aggregate_trade: None,
            trades: Vec::new(),
            liquidities: Vec::new(),
            transfers: Vec::new(),
            sol_balance_change: None,
            token_balance_change: HashMap::new(),
            meme_events: Vec::new(),
            slot: 0,
            timestamp: 0,
            signature: String::new(),
            signer: Vec::new(),
            compute_units: 0,
            tx_status: TransactionStatus::default(),
            msg: None,
        }
    }
}

/// Transaction meta information used by the adapter.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TransactionMeta {
    pub fee: u64,
    pub compute_units: u64,
    pub status: TransactionStatus,
    #[serde(default)]
    pub sol_balance_changes: HashMap<String, BalanceChange>,
    #[serde(default)]
    pub token_balance_changes: HashMap<String, HashMap<String, BalanceChange>>,
}

/// Simplified transaction representation consumed by the parser.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SolanaTransaction {
    pub slot: u64,
    pub signature: String,
    pub block_time: u64,
    #[serde(default)]
    pub signers: Vec<String>,
    #[serde(default)]
    pub instructions: Vec<SolanaInstruction>,
    #[serde(default)]
    pub transfers: Vec<TransferData>,
    #[serde(default)]
    pub meta: TransactionMeta,
}

/// Block representation for CLI parsing.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SolanaBlock {
    pub slot: u64,
    #[serde(default)]
    pub block_time: Option<u64>,
    #[serde(default)]
    pub transactions: Vec<SolanaTransaction>,
}

/// Input wrapper for CLI block parsing distinguishing between raw and parsed data.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BlockInput {
    Raw {
        transactions: Vec<serde_json::Value>,
    },
    Parsed {
        block: SolanaBlock,
    },
}

/// Wrapper returned by `parse_block` helper functions.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BlockParseResult {
    pub slot: u64,
    #[serde(default)]
    pub timestamp: Option<u64>,
    pub transactions: Vec<ParseResult>,
}

/// Convenience alias used by parsers.
pub type TransferMap = HashMap<String, Vec<TransferData>>;

/// Convenience alias used by parsers.
pub type InstructionList = Vec<ClassifiedInstruction>;

/// Helper trait for converting from raw JSON transactions.
pub trait FromJsonValue {
    fn from_value(value: &serde_json::Value, config: &ParseConfig) -> Result<SolanaTransaction>;
}

impl FromJsonValue for SolanaTransaction {
    fn from_value(value: &serde_json::Value, _config: &ParseConfig) -> Result<SolanaTransaction> {
        serde_json::from_value(value.clone())
            .map_err(|err| anyhow!("failed to deserialize transaction: {err}"))
    }
}
