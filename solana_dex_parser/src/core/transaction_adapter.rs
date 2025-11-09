use std::collections::HashMap;

use crate::config::ParseConfig;
use crate::types::{
    BalanceChange, SolanaInstruction, SolanaTransaction, TokenAmount, TransactionStatus,
    TransferData, TransferMap,
};

#[derive(Clone, Debug)]
pub struct TransactionAdapter {
    tx: SolanaTransaction,
    config: ParseConfig,
}

impl TransactionAdapter {
    pub fn new(tx: SolanaTransaction, config: ParseConfig) -> Self {
        Self { tx, config }
    }

    pub fn slot(&self) -> u64 {
        self.tx.slot
    }

    pub fn block_time(&self) -> u64 {
        self.tx.block_time
    }

    pub fn signature(&self) -> &str {
        &self.tx.signature
    }

    pub fn signers(&self) -> &[String] {
        &self.tx.signers
    }

    pub fn signer(&self) -> Option<&String> {
        self.tx.signers.first()
    }

    pub fn instructions(&self) -> &[SolanaInstruction] {
        &self.tx.instructions
    }

    pub fn transfers(&self) -> &[TransferData] {
        &self.tx.transfers
    }

    pub fn fee(&self) -> TokenAmount {
        TokenAmount::new("SOL", self.tx.meta.fee, 9)
    }

    pub fn compute_units(&self) -> u64 {
        self.tx.meta.compute_units
    }

    pub fn tx_status(&self) -> TransactionStatus {
        self.tx.meta.status
    }

    pub fn signer_sol_balance_change(&self) -> Option<&BalanceChange> {
        self.signer()
            .and_then(|signer| self.tx.meta.sol_balance_changes.get(signer))
    }

    pub fn signer_token_balance_changes(&self) -> Option<&HashMap<String, BalanceChange>> {
        self.signer()
            .and_then(|signer| self.tx.meta.token_balance_changes.get(signer))
    }

    pub fn get_transfer_actions(&self) -> TransferMap {
        let mut map: TransferMap = HashMap::new();
        for transfer in &self.tx.transfers {
            map.entry(transfer.program_id.clone())
                .or_default()
                .push(transfer.clone());
        }
        map
    }

    pub fn is_supported_token(&self, _transfer: &TransferData) -> bool {
        // In this simplified implementation we assume every transfer mint is supported.
        true
    }

    pub fn config(&self) -> &ParseConfig {
        &self.config
    }
}
