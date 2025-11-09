use std::collections::HashMap;

use crate::config::ParseConfig;
use crate::types::{
    BalanceChange, InnerInstruction, SolanaInstruction, SolanaTransaction, TokenAmount, TokenInfo,
    TransactionStatus, TransferData, TransferMap,
};

#[derive(Clone, Debug)]
pub struct TransactionAdapter {
    tx: SolanaTransaction,
    config: ParseConfig,
    token_accounts: HashMap<String, TokenInfo>,
    token_decimals: HashMap<String, u8>,
}

impl TransactionAdapter {
    pub fn new(tx: SolanaTransaction, config: ParseConfig) -> Self {
        let (token_accounts, token_decimals) = Self::extract_token_maps(&tx);
        Self {
            tx,
            config,
            token_accounts,
            token_decimals,
        }
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

    pub fn inner_instructions(&self) -> &[InnerInstruction] {
        &self.tx.inner_instructions
    }

    pub fn transfers(&self) -> &[TransferData] {
        &self.tx.transfers
    }

    pub fn instruction_accounts<'a>(&self, instruction: &'a SolanaInstruction) -> &'a [String] {
        instruction.accounts.as_slice()
    }

    pub fn token_account_info(&self, account: &str) -> Option<&TokenInfo> {
        self.token_accounts.get(account)
    }

    pub fn token_decimals(&self, mint: &str) -> Option<u8> {
        self.token_decimals.get(mint).copied()
    }

    pub fn fee(&self) -> TokenAmount {
        let amount = self.tx.meta.fee.to_string();
        let ui_amount = Some(self.tx.meta.fee as f64 / 1_000_000_000f64);
        TokenAmount::new(amount, 9, ui_amount)
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

    fn extract_token_maps(
        tx: &SolanaTransaction,
    ) -> (HashMap<String, TokenInfo>, HashMap<String, u8>) {
        let mut accounts: HashMap<String, TokenInfo> = HashMap::new();
        let mut decimals: HashMap<String, u8> = HashMap::new();

        for transfer in &tx.transfers {
            let info = &transfer.info;
            let amount = info
                .token_amount
                .ui_amount
                .unwrap_or_else(|| info.token_amount.amount.parse::<f64>().unwrap_or(0.0));
            let token_info = TokenInfo {
                mint: info.mint.clone(),
                amount,
                amount_raw: info.token_amount.amount.clone(),
                decimals: info.token_amount.decimals,
                authority: info.authority.clone(),
                destination: Some(info.destination.clone()),
                destination_owner: info.destination_owner.clone(),
                destination_balance: info.destination_balance.clone(),
                destination_pre_balance: info.destination_pre_balance.clone(),
                source: Some(info.source.clone()),
                source_balance: info.source_balance.clone(),
                source_pre_balance: info.source_pre_balance.clone(),
                destination_balance_change: None,
                source_balance_change: None,
                balance_change: info.sol_balance_change.clone(),
            };

            accounts
                .entry(info.source.clone())
                .or_insert_with(|| token_info.clone());
            accounts
                .entry(info.destination.clone())
                .or_insert_with(|| token_info.clone());

            decimals
                .entry(info.mint.clone())
                .or_insert(info.token_amount.decimals);
        }

        (accounts, decimals)
    }
}
