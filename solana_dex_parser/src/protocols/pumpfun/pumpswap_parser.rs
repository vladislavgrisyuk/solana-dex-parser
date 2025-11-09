use crate::core::transaction_adapter::TransactionAdapter;
use crate::protocols::simple::TradeParser;
use crate::types::{ClassifiedInstruction, DexInfo, TradeInfo, TransferMap};

use super::pumpswap_event_parser::{PumpswapEvent, PumpswapEventData, PumpswapEventParser};
use super::util::{attach_token_transfers, build_pumpswap_buy_trade, build_pumpswap_sell_trade};

pub struct PumpswapParser {
    adapter: TransactionAdapter,
    dex_info: DexInfo,
    transfer_actions: TransferMap,
    classified_instructions: Vec<ClassifiedInstruction>,
    event_parser: PumpswapEventParser,
}

impl PumpswapParser {
    pub fn new(
        adapter: TransactionAdapter,
        dex_info: DexInfo,
        transfer_actions: TransferMap,
        classified_instructions: Vec<ClassifiedInstruction>,
    ) -> Self {
        let event_parser = PumpswapEventParser::new(adapter.clone());
        Self {
            adapter,
            dex_info,
            transfer_actions,
            classified_instructions,
            event_parser,
        }
    }

    fn parse_events(&self) -> Vec<PumpswapEvent> {
        match self
            .event_parser
            .parse_instructions(&self.classified_instructions)
        {
            Ok(events) => events,
            Err(err) => {
                tracing::error!("failed to parse pumpswap events: {err}");
                Vec::new()
            }
        }
    }

    fn create_buy_trade(
        &self,
        event: &PumpswapEvent,
        buy: &super::pumpswap_event_parser::PumpswapBuyEvent,
    ) -> Option<TradeInfo> {
        let input_info = self
            .adapter
            .token_account_info(&buy.user_quote_token_account)?;
        let output_info = self
            .adapter
            .token_account_info(&buy.user_base_token_account)?;
        let fee_info = self
            .adapter
            .token_account_info(&buy.protocol_fee_recipient_token_account)?;

        let input_decimals = self
            .adapter
            .token_decimals(&input_info.mint)
            .unwrap_or(input_info.decimals);
        let output_decimals = self
            .adapter
            .token_decimals(&output_info.mint)
            .unwrap_or(output_info.decimals);
        let fee_decimals = self
            .adapter
            .token_decimals(&fee_info.mint)
            .unwrap_or(fee_info.decimals);

        let trade = build_pumpswap_buy_trade(
            event,
            buy,
            (&input_info.mint, input_decimals),
            (&output_info.mint, output_decimals),
            (&fee_info.mint, fee_decimals),
            &self.dex_info,
        );

        Some(attach_token_transfers(
            &self.adapter,
            trade,
            &self.transfer_actions,
        ))
    }

    fn create_sell_trade(
        &self,
        event: &PumpswapEvent,
        sell: &super::pumpswap_event_parser::PumpswapSellEvent,
    ) -> Option<TradeInfo> {
        let input_info = self
            .adapter
            .token_account_info(&sell.user_base_token_account)?;
        let output_info = self
            .adapter
            .token_account_info(&sell.user_quote_token_account)?;
        let fee_info = self
            .adapter
            .token_account_info(&sell.protocol_fee_recipient_token_account)?;

        let input_decimals = self
            .adapter
            .token_decimals(&input_info.mint)
            .unwrap_or(input_info.decimals);
        let output_decimals = self
            .adapter
            .token_decimals(&output_info.mint)
            .unwrap_or(output_info.decimals);
        let fee_decimals = self
            .adapter
            .token_decimals(&fee_info.mint)
            .unwrap_or(fee_info.decimals);

        let trade = build_pumpswap_sell_trade(
            event,
            sell,
            (&input_info.mint, input_decimals),
            (&output_info.mint, output_decimals),
            (&fee_info.mint, fee_decimals),
            &self.dex_info,
        );

        Some(attach_token_transfers(
            &self.adapter,
            trade,
            &self.transfer_actions,
        ))
    }
}

impl TradeParser for PumpswapParser {
    fn process_trades(&mut self) -> Vec<TradeInfo> {
        let mut trades = Vec::new();
        for event in self.parse_events() {
            match &event.data {
                PumpswapEventData::Buy(buy) => {
                    if let Some(trade) = self.create_buy_trade(&event, buy) {
                        trades.push(trade);
                    }
                }
                PumpswapEventData::Sell(sell) => {
                    if let Some(trade) = self.create_sell_trade(&event, sell) {
                        trades.push(trade);
                    }
                }
                _ => {}
            }
        }
        trades
    }
}
