use crate::core::constants::dex_program_names;
use crate::core::instruction_classifier::InstructionClassifier;
use crate::core::transaction_adapter::TransactionAdapter;
use crate::types::{DexInfo, FeeInfo, PoolEvent, TradeInfo, TradeType, TransferData, TransferMap};

#[derive(Clone, Debug)]
pub struct TransactionUtils {
    adapter: TransactionAdapter,
}

impl TransactionUtils {
    pub fn new(adapter: TransactionAdapter) -> Self {
        Self { adapter }
    }

    pub fn get_dex_info(&self, classifier: &InstructionClassifier) -> DexInfo {
        let program_id = classifier.get_all_program_ids().into_iter().next();
        let amm = program_id
            .as_ref()
            .map(|id| dex_program_names::name(id).to_string());
        DexInfo {
            program_id,
            amm,
            route: None,
        }
    }

    pub fn get_transfer_actions(&self) -> TransferMap {
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

        let input = transfers.first()?;
        let output = transfers.get(1)?;
        let program_id = dex_info
            .program_id
            .clone()
            .unwrap_or_else(|| input.program_id.clone());
        let amm = dex_info
            .amm
            .clone()
            .unwrap_or_else(|| dex_program_names::name(&program_id).to_string());

        let input_token = Self::transfer_to_token_info(input);
        let output_token = Self::transfer_to_token_info(output);

        Some(TradeInfo {
            trade_type: TradeType::Swap,
            pool: Vec::new(),
            input_token,
            output_token,
            slippage_bps: None,
            fee: None,
            fees: Vec::new(),
            user: Some(input.info.source.clone()),
            program_id: Some(program_id),
            amm: Some(amm),
            amms: None,
            route: dex_info.route.clone(),
            slot: self.adapter.slot(),
            timestamp: self.adapter.block_time(),
            signature: self.adapter.signature().to_string(),
            idx: input.idx.clone(),
            signer: Some(self.adapter.signers().to_vec()),
        })
    }

    pub fn attach_trade_fee(&self, mut trade: TradeInfo) -> TradeInfo {
        let fee_amount = self.adapter.fee();
        if fee_amount.amount != "0" {
            let fee = FeeInfo {
                mint: "SOL".to_string(),
                amount: fee_amount.ui_amount.unwrap_or(0.0),
                amount_raw: fee_amount.amount.clone(),
                decimals: fee_amount.decimals,
                dex: None,
                fee_type: None,
                recipient: None,
            };
            trade.fee = Some(fee);
        }
        trade
    }

    pub fn attach_token_transfer_info(
        &self,
        trade: TradeInfo,
        _transfer_actions: &TransferMap,
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

impl TransactionUtils {
    fn transfer_to_token_info(transfer: &TransferData) -> crate::types::TokenInfo {
        let amount = transfer.info.token_amount.ui_amount.unwrap_or_else(|| {
            transfer
                .info
                .token_amount
                .amount
                .parse::<f64>()
                .unwrap_or(0.0)
        });

        crate::types::TokenInfo {
            mint: transfer.info.mint.clone(),
            amount,
            amount_raw: transfer.info.token_amount.amount.clone(),
            decimals: transfer.info.token_amount.decimals,
            authority: transfer.info.authority.clone(),
            destination: Some(transfer.info.destination.clone()),
            destination_owner: transfer.info.destination_owner.clone(),
            destination_balance: transfer.info.destination_balance.clone(),
            destination_pre_balance: transfer.info.destination_pre_balance.clone(),
            source: Some(transfer.info.source.clone()),
            source_balance: transfer.info.source_balance.clone(),
            source_pre_balance: transfer.info.source_pre_balance.clone(),
            destination_balance_change: None,
            source_balance_change: None,
            balance_change: transfer.info.sol_balance_change.clone(),
        }
    }
}
