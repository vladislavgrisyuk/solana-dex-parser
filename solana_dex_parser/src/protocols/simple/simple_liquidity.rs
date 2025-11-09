use crate::core::transaction_adapter::TransactionAdapter;
use crate::types::{ClassifiedInstruction, PoolEvent, TransferData, TransferMap};

use super::LiquidityParser;

pub struct SimpleLiquidityParser {
    adapter: TransactionAdapter,
    transfer_actions: TransferMap,
    classified_instructions: Vec<ClassifiedInstruction>,
}

impl SimpleLiquidityParser {
    pub fn new(
        adapter: TransactionAdapter,
        transfer_actions: TransferMap,
        classified_instructions: Vec<ClassifiedInstruction>,
    ) -> Self {
        Self {
            adapter,
            transfer_actions,
            classified_instructions,
        }
    }

    pub fn boxed(
        adapter: TransactionAdapter,
        transfer_actions: TransferMap,
        classified_instructions: Vec<ClassifiedInstruction>,
    ) -> Box<dyn LiquidityParser> {
        Box::new(Self::new(
            adapter,
            transfer_actions,
            classified_instructions,
        ))
    }
}

impl LiquidityParser for SimpleLiquidityParser {
    fn process_liquidity(&mut self) -> Vec<PoolEvent> {
        let mut events = Vec::new();
        for instruction in &self.classified_instructions {
            let liquidity = self
                .transfer_actions
                .get(&instruction.program_id)
                .map(|transfers| transfers.iter().map(|t| t.amount.amount).sum())
                .unwrap_or(0);
            events.push(PoolEvent {
                program_id: instruction.program_id.clone(),
                event_type: "liquidity".to_string(),
                mint_a: instruction
                    .data
                    .accounts
                    .get(0)
                    .cloned()
                    .unwrap_or_default(),
                mint_b: instruction
                    .data
                    .accounts
                    .get(1)
                    .cloned()
                    .unwrap_or_default(),
                liquidity,
                signature: self.adapter.signature().to_string(),
                idx: format!("{}", instruction.outer_index),
            });
        }
        events
    }
}
