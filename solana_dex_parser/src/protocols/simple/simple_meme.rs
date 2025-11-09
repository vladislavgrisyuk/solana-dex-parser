use crate::core::transaction_adapter::TransactionAdapter;
use crate::types::{MemeEvent, TransferData, TransferMap};

use super::MemeEventParser;

pub struct SimpleMemeParser {
    adapter: TransactionAdapter,
    transfer_actions: TransferMap,
}

impl SimpleMemeParser {
    pub fn new(adapter: TransactionAdapter, transfer_actions: TransferMap) -> Self {
        Self {
            adapter,
            transfer_actions,
        }
    }

    pub fn boxed(
        adapter: TransactionAdapter,
        transfer_actions: HashMap<String, Vec<TransferData>>,
    ) -> Box<dyn MemeEventParser> {
        Box::new(Self::new(adapter, transfer_actions))
    }
}

impl MemeEventParser for SimpleMemeParser {
    fn process_events(&mut self) -> Vec<MemeEvent> {
        self.transfer_actions
            .values()
            .flat_map(|transfers| transfers.iter())
            .map(|transfer| MemeEvent {
                program_id: transfer.program_id.clone(),
                event_type: "meme-event".to_string(),
                signature: self.adapter.signature().to_string(),
                description: format!(
                    "{} -> {} {}",
                    transfer.from, transfer.to, transfer.amount.amount
                ),
            })
            .collect()
    }
}
