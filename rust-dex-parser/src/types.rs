use std::collections::HashMap;

/// Representation of a token amount inside a transaction.
#[derive(Clone, Debug, PartialEq)]
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
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BalanceChange {
    pub pre: i128,
    pub post: i128,
    pub change: i128,
}

/// Execution status for a Solana transaction.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum TransactionStatus {
    #[default]
    Unknown,
    Success,
    Failed,
}

/// Parser configuration mirroring the TypeScript variant.
#[derive(Clone, Debug, PartialEq)]
pub struct ParseConfig {
    pub try_unknown_dex: bool,
    pub program_ids: Option<Vec<String>>,
    pub ignore_program_ids: Option<Vec<String>>,
    pub aggregate_trades: bool,
    pub throw_error: bool,
}

impl Default for ParseConfig {
    fn default() -> Self {
        Self {
            try_unknown_dex: true,
            program_ids: None,
            ignore_program_ids: None,
            aggregate_trades: true,
            throw_error: false,
        }
    }
}

/// Minimal instruction representation with bookkeeping indices.
#[derive(Clone, Debug, PartialEq)]
pub struct ClassifiedInstruction {
    pub program_id: String,
    pub outer_index: usize,
    pub inner_index: Option<usize>,
    pub data: SolanaInstruction,
}

/// Basic representation of a Solana instruction.
#[derive(Clone, Debug, PartialEq)]
pub struct SolanaInstruction {
    pub program_id: String,
    pub accounts: Vec<String>,
    pub data: String,
}

/// Transfer data emitted by the meta simulation.
#[derive(Clone, Debug, PartialEq)]
pub struct TransferData {
    pub program_id: String,
    pub from: String,
    pub to: String,
    pub amount: TokenAmount,
    pub idx: String,
}

/// High level trade information extracted from a transaction.
#[derive(Clone, Debug, PartialEq)]
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
#[derive(Clone, Debug, PartialEq)]
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
#[derive(Clone, Debug, PartialEq)]
pub struct MemeEvent {
    pub program_id: String,
    pub event_type: String,
    pub signature: String,
    pub description: String,
}

/// Additional context information about the parsed transaction.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DexInfo {
    pub program_id: Option<String>,
    pub amm: Option<String>,
}

/// Aggregated parsing result returned by the Rust parser.
#[derive(Clone, Debug, PartialEq)]
pub struct ParseResult {
    pub state: bool,
    pub fee: TokenAmount,
    pub aggregate_trade: Option<TradeInfo>,
    pub trades: Vec<TradeInfo>,
    pub liquidities: Vec<PoolEvent>,
    pub transfers: Vec<TransferData>,
    pub sol_balance_change: Option<BalanceChange>,
    pub token_balance_change: HashMap<String, BalanceChange>,
    pub meme_events: Vec<MemeEvent>,
    pub slot: u64,
    pub timestamp: u64,
    pub signature: String,
    pub signer: Vec<String>,
    pub compute_units: u64,
    pub tx_status: TransactionStatus,
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
            tx_status: TransactionStatus::Unknown,
            msg: None,
        }
    }
}

/// Transaction meta information used by the adapter.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TransactionMeta {
    pub fee: u64,
    pub compute_units: u64,
    pub status: TransactionStatus,
    pub sol_balance_changes: HashMap<String, BalanceChange>,
    pub token_balance_changes: HashMap<String, HashMap<String, BalanceChange>>,
}

/// Simplified transaction representation consumed by the parser.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SolanaTransaction {
    pub slot: u64,
    pub signature: String,
    pub block_time: u64,
    pub signers: Vec<String>,
    pub instructions: Vec<SolanaInstruction>,
    pub transfers: Vec<TransferData>,
    pub meta: TransactionMeta,
}
