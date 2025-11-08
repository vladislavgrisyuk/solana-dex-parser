use std::collections::HashMap;

use crate::constants::dex_program_names;
use crate::transaction_adapter::TransactionAdapter;
use crate::types::{DexInfo, PoolEvent, TradeInfo, TransferData};

#[derive(Clone, Debug)]
pub struct TransactionUtils {
    adapter: TransactionAdapter,
}

impl TransactionUtils {
    pub fn new(adapter: TransactionAdapter) -> Self {
        Self { adapter }
    }

    pub fn get_dex_info(
        &self,
        classifier: &crate::instruction_classifier::InstructionClassifier,
    ) -> DexInfo {
        let program_id = classifier.get_all_program_ids().into_iter().next();
        let amm = program_id
            .as_ref()
            .map(|id| dex_program_names::name(id).to_string());
        DexInfo { program_id, amm }
    }

    pub fn get_transfer_actions(&self) -> HashMap<String, Vec<TransferData>> {
        self.adapter.get_transfer_actions()
    }

    pub fn process_swap_data(
        &self,
        transfers: &[TransferData],
        dex_info: &DexInfo,
    ) -> Option<TradeInfo> {
        if transfers.len() < 2 {
            return None;
        }

        let input = transfers.first()?.clone();
        let output = transfers.get(1)?.clone();
        let program_id = dex_info
            .program_id
            .clone()
            .unwrap_or_else(|| input.program_id.clone());
        let amm = dex_info
            .amm
            .clone()
            .unwrap_or_else(|| dex_program_names::name(&program_id).to_string());

        Some(TradeInfo {
            program_id,
            amm,
            signature: self.adapter.signature().to_string(),
            idx: input.idx.clone(),
            in_amount: input.amount,
            out_amount: output.amount,
            fee: None,
        })
    }

    pub fn attach_trade_fee(&self, mut trade: TradeInfo) -> TradeInfo {
        let fee_amount = self.adapter.fee();
        if fee_amount.amount > 0 {
            trade.fee = Some(fee_amount);
        }
        trade
    }

    pub fn attach_token_transfer_info(
        &self,
        trade: TradeInfo,
        _transfer_actions: &HashMap<String, Vec<TransferData>>,
    ) -> TradeInfo {
        trade
    }

    pub fn attach_user_balance_to_lps(&self, pools: Vec<PoolEvent>) -> Vec<PoolEvent> {
        if let Some(signer) = self.adapter.signer() {
            pools
                .into_iter()
                .map(|mut pool| {
                    pool.idx = format!("{}-{}", signer, pool.idx);
                    pool
                })
                .collect()
        } else {
            pools
        }
    }
}
