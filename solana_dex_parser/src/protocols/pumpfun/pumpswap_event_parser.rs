use crate::core::transaction_adapter::TransactionAdapter;
use crate::types::ClassifiedInstruction;
use anyhow::Result;

use super::binary_reader::BinaryReader;
use super::constants::discriminators::pumpswap_events;
use super::util::{get_instruction_data, sort_by_idx, HasIdx};

#[derive(Clone, Debug, PartialEq)]
pub enum PumpswapEventType {
    Create,
    Add,
    Remove,
    Buy,
    Sell,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PumpswapEvent {
    pub event_type: PumpswapEventType,
    pub data: PumpswapEventData,
    pub slot: u64,
    pub timestamp: u64,
    pub signature: String,
    pub idx: String,
    pub signer: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PumpswapEventData {
    Buy(PumpswapBuyEvent),
    Sell(PumpswapSellEvent),
    Create(PumpswapCreatePoolEvent),
    Deposit(PumpswapDepositEvent),
    Withdraw(PumpswapWithdrawEvent),
}

#[derive(Clone, Debug, PartialEq)]
pub struct PumpswapBuyEvent {
    pub timestamp: u64,
    pub base_amount_out: u64,
    pub max_quote_amount_in: u64,
    pub user_base_token_reserves: u64,
    pub user_quote_token_reserves: u64,
    pub pool_base_token_reserves: u64,
    pub pool_quote_token_reserves: u64,
    pub quote_amount_in: u64,
    pub lp_fee_basis_points: u64,
    pub lp_fee: u64,
    pub protocol_fee_basis_points: u64,
    pub protocol_fee: u64,
    pub quote_amount_in_with_lp_fee: u64,
    pub user_quote_amount_in: u64,
    pub pool: String,
    pub user: String,
    pub user_base_token_account: String,
    pub user_quote_token_account: String,
    pub protocol_fee_recipient: String,
    pub protocol_fee_recipient_token_account: String,
    pub coin_creator: String,
    pub coin_creator_fee_basis_points: u64,
    pub coin_creator_fee: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PumpswapSellEvent {
    pub timestamp: u64,
    pub base_amount_in: u64,
    pub min_quote_amount_out: u64,
    pub user_base_token_reserves: u64,
    pub user_quote_token_reserves: u64,
    pub pool_base_token_reserves: u64,
    pub pool_quote_token_reserves: u64,
    pub quote_amount_out: u64,
    pub lp_fee_basis_points: u64,
    pub lp_fee: u64,
    pub protocol_fee_basis_points: u64,
    pub protocol_fee: u64,
    pub quote_amount_out_without_lp_fee: u64,
    pub user_quote_amount_out: u64,
    pub pool: String,
    pub user: String,
    pub user_base_token_account: String,
    pub user_quote_token_account: String,
    pub protocol_fee_recipient: String,
    pub protocol_fee_recipient_token_account: String,
    pub coin_creator: String,
    pub coin_creator_fee_basis_points: u64,
    pub coin_creator_fee: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PumpswapCreatePoolEvent {
    pub timestamp: u64,
    pub index: u16,
    pub creator: String,
    pub base_mint: String,
    pub quote_mint: String,
    pub base_mint_decimals: u8,
    pub quote_mint_decimals: u8,
    pub base_amount_in: u64,
    pub quote_amount_in: u64,
    pub pool_base_amount: u64,
    pub pool_quote_amount: u64,
    pub minimum_liquidity: u64,
    pub initial_liquidity: u64,
    pub lp_token_amount_out: u64,
    pub pool_bump: u8,
    pub pool: String,
    pub lp_mint: String,
    pub user_base_token_account: String,
    pub user_quote_token_account: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PumpswapDepositEvent {
    pub timestamp: u64,
    pub lp_token_amount_out: u64,
    pub max_base_amount_in: u64,
    pub max_quote_amount_in: u64,
    pub user_base_token_reserves: u64,
    pub user_quote_token_reserves: u64,
    pub pool_base_token_reserves: u64,
    pub pool_quote_token_reserves: u64,
    pub base_amount_in: u64,
    pub quote_amount_in: u64,
    pub lp_mint_supply: u64,
    pub pool: String,
    pub user: String,
    pub user_base_token_account: String,
    pub user_quote_token_account: String,
    pub user_pool_token_account: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PumpswapWithdrawEvent {
    pub timestamp: u64,
    pub lp_token_amount_in: u64,
    pub min_base_amount_out: u64,
    pub min_quote_amount_out: u64,
    pub user_base_token_reserves: u64,
    pub user_quote_token_reserves: u64,
    pub pool_base_token_reserves: u64,
    pub pool_quote_token_reserves: u64,
    pub base_amount_out: u64,
    pub quote_amount_out: u64,
    pub lp_mint_supply: u64,
    pub pool: String,
    pub user: String,
    pub user_base_token_account: String,
    pub user_quote_token_account: String,
    pub user_pool_token_account: String,
}

pub struct PumpswapEventParser {
    adapter: TransactionAdapter,
}

impl PumpswapEventParser {
    pub fn new(adapter: TransactionAdapter) -> Self {
        Self { adapter }
    }

    pub fn parse_instructions(
        &self,
        instructions: &[ClassifiedInstruction],
    ) -> Result<Vec<PumpswapEvent>> {
        let mut events = Vec::new();
        for classified in instructions {
            let data = get_instruction_data(&classified.data)?;
            if data.len() < 16 {
                continue;
            }
            let discriminator = &data[..16];
            let payload = data[16..].to_vec();

            let event_type = if discriminator == pumpswap_events::CREATE_POOL {
                Some(PumpswapEventType::Create)
            } else if discriminator == pumpswap_events::ADD_LIQUIDITY {
                Some(PumpswapEventType::Add)
            } else if discriminator == pumpswap_events::REMOVE_LIQUIDITY {
                Some(PumpswapEventType::Remove)
            } else if discriminator == pumpswap_events::BUY {
                Some(PumpswapEventType::Buy)
            } else if discriminator == pumpswap_events::SELL {
                Some(PumpswapEventType::Sell)
            } else {
                None
            };

            if let Some(event_type) = event_type {
                let data = self.decode_event(&event_type, payload)?;
                let event = PumpswapEvent {
                    event_type,
                    data,
                    slot: self.adapter.slot(),
                    timestamp: self.adapter.block_time(),
                    signature: self.adapter.signature().to_string(),
                    idx: format!(
                        "{}-{}",
                        classified.outer_index,
                        classified.inner_index.unwrap_or(0)
                    ),
                    signer: Some(self.adapter.signers().to_vec()),
                };
                events.push(event);
            }
        }

        Ok(sort_by_idx(events))
    }

    fn decode_event(
        &self,
        event_type: &PumpswapEventType,
        data: Vec<u8>,
    ) -> Result<PumpswapEventData> {
        match event_type {
            PumpswapEventType::Buy => Ok(PumpswapEventData::Buy(self.decode_buy_event(data)?)),
            PumpswapEventType::Sell => Ok(PumpswapEventData::Sell(self.decode_sell_event(data)?)),
            PumpswapEventType::Create => {
                Ok(PumpswapEventData::Create(self.decode_create_event(data)?))
            }
            PumpswapEventType::Add => {
                Ok(PumpswapEventData::Deposit(self.decode_add_liquidity(data)?))
            }
            PumpswapEventType::Remove => Ok(PumpswapEventData::Withdraw(
                self.decode_remove_liquidity(data)?,
            )),
        }
    }

    fn decode_buy_event(&self, data: Vec<u8>) -> Result<PumpswapBuyEvent> {
        let mut reader = BinaryReader::new(data);
        Ok(PumpswapBuyEvent {
            timestamp: read_timestamp(&mut reader)?,
            base_amount_out: reader.read_u64()?,
            max_quote_amount_in: reader.read_u64()?,
            user_base_token_reserves: reader.read_u64()?,
            user_quote_token_reserves: reader.read_u64()?,
            pool_base_token_reserves: reader.read_u64()?,
            pool_quote_token_reserves: reader.read_u64()?,
            quote_amount_in: reader.read_u64()?,
            lp_fee_basis_points: reader.read_u64()?,
            lp_fee: reader.read_u64()?,
            protocol_fee_basis_points: reader.read_u64()?,
            protocol_fee: reader.read_u64()?,
            quote_amount_in_with_lp_fee: reader.read_u64()?,
            user_quote_amount_in: reader.read_u64()?,
            pool: reader.read_pubkey()?,
            user: reader.read_pubkey()?,
            user_base_token_account: reader.read_pubkey()?,
            user_quote_token_account: reader.read_pubkey()?,
            protocol_fee_recipient: reader.read_pubkey()?,
            protocol_fee_recipient_token_account: reader.read_pubkey()?,
            coin_creator: if reader.remaining() > 0 {
                reader.read_pubkey()?
            } else {
                "11111111111111111111111111111111".to_string()
            },
            coin_creator_fee_basis_points: if reader.remaining() > 0 {
                reader.read_u64()?
            } else {
                0
            },
            coin_creator_fee: if reader.remaining() > 0 {
                reader.read_u64()?
            } else {
                0
            },
        })
    }

    fn decode_sell_event(&self, data: Vec<u8>) -> Result<PumpswapSellEvent> {
        let mut reader = BinaryReader::new(data);
        Ok(PumpswapSellEvent {
            timestamp: read_timestamp(&mut reader)?,
            base_amount_in: reader.read_u64()?,
            min_quote_amount_out: reader.read_u64()?,
            user_base_token_reserves: reader.read_u64()?,
            user_quote_token_reserves: reader.read_u64()?,
            pool_base_token_reserves: reader.read_u64()?,
            pool_quote_token_reserves: reader.read_u64()?,
            quote_amount_out: reader.read_u64()?,
            lp_fee_basis_points: reader.read_u64()?,
            lp_fee: reader.read_u64()?,
            protocol_fee_basis_points: reader.read_u64()?,
            protocol_fee: reader.read_u64()?,
            quote_amount_out_without_lp_fee: reader.read_u64()?,
            user_quote_amount_out: reader.read_u64()?,
            pool: reader.read_pubkey()?,
            user: reader.read_pubkey()?,
            user_base_token_account: reader.read_pubkey()?,
            user_quote_token_account: reader.read_pubkey()?,
            protocol_fee_recipient: reader.read_pubkey()?,
            protocol_fee_recipient_token_account: reader.read_pubkey()?,
            coin_creator: if reader.remaining() > 0 {
                reader.read_pubkey()?
            } else {
                "11111111111111111111111111111111".to_string()
            },
            coin_creator_fee_basis_points: if reader.remaining() > 0 {
                reader.read_u64()?
            } else {
                0
            },
            coin_creator_fee: if reader.remaining() > 0 {
                reader.read_u64()?
            } else {
                0
            },
        })
    }

    fn decode_add_liquidity(&self, data: Vec<u8>) -> Result<PumpswapDepositEvent> {
        let mut reader = BinaryReader::new(data);
        Ok(PumpswapDepositEvent {
            timestamp: read_timestamp(&mut reader)?,
            lp_token_amount_out: reader.read_u64()?,
            max_base_amount_in: reader.read_u64()?,
            max_quote_amount_in: reader.read_u64()?,
            user_base_token_reserves: reader.read_u64()?,
            user_quote_token_reserves: reader.read_u64()?,
            pool_base_token_reserves: reader.read_u64()?,
            pool_quote_token_reserves: reader.read_u64()?,
            base_amount_in: reader.read_u64()?,
            quote_amount_in: reader.read_u64()?,
            lp_mint_supply: reader.read_u64()?,
            pool: reader.read_pubkey()?,
            user: reader.read_pubkey()?,
            user_base_token_account: reader.read_pubkey()?,
            user_quote_token_account: reader.read_pubkey()?,
            user_pool_token_account: reader.read_pubkey()?,
        })
    }

    fn decode_create_event(&self, data: Vec<u8>) -> Result<PumpswapCreatePoolEvent> {
        let mut reader = BinaryReader::new(data);
        Ok(PumpswapCreatePoolEvent {
            timestamp: read_timestamp(&mut reader)?,
            index: reader.read_u16()?,
            creator: reader.read_pubkey()?,
            base_mint: reader.read_pubkey()?,
            quote_mint: reader.read_pubkey()?,
            base_mint_decimals: reader.read_u8()?,
            quote_mint_decimals: reader.read_u8()?,
            base_amount_in: reader.read_u64()?,
            quote_amount_in: reader.read_u64()?,
            pool_base_amount: reader.read_u64()?,
            pool_quote_amount: reader.read_u64()?,
            minimum_liquidity: reader.read_u64()?,
            initial_liquidity: reader.read_u64()?,
            lp_token_amount_out: reader.read_u64()?,
            pool_bump: reader.read_u8()?,
            pool: reader.read_pubkey()?,
            lp_mint: reader.read_pubkey()?,
            user_base_token_account: reader.read_pubkey()?,
            user_quote_token_account: reader.read_pubkey()?,
        })
    }

    fn decode_remove_liquidity(&self, data: Vec<u8>) -> Result<PumpswapWithdrawEvent> {
        let mut reader = BinaryReader::new(data);
        Ok(PumpswapWithdrawEvent {
            timestamp: read_timestamp(&mut reader)?,
            lp_token_amount_in: reader.read_u64()?,
            min_base_amount_out: reader.read_u64()?,
            min_quote_amount_out: reader.read_u64()?,
            user_base_token_reserves: reader.read_u64()?,
            user_quote_token_reserves: reader.read_u64()?,
            pool_base_token_reserves: reader.read_u64()?,
            pool_quote_token_reserves: reader.read_u64()?,
            base_amount_out: reader.read_u64()?,
            quote_amount_out: reader.read_u64()?,
            lp_mint_supply: reader.read_u64()?,
            pool: reader.read_pubkey()?,
            user: reader.read_pubkey()?,
            user_base_token_account: reader.read_pubkey()?,
            user_quote_token_account: reader.read_pubkey()?,
            user_pool_token_account: reader.read_pubkey()?,
        })
    }
}

impl HasIdx for PumpswapEvent {
    fn idx(&self) -> &str {
        &self.idx
    }
}

fn read_timestamp(reader: &mut BinaryReader) -> Result<u64> {
    let value = reader.read_i64()?;
    Ok(if value >= 0 { value as u64 } else { 0 })
}
