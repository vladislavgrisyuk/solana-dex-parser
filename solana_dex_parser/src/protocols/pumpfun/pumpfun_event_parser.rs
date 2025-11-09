use anyhow::Result;
use bs58::encode as bs58_encode;

use crate::types::{ClassifiedInstruction, MemeEvent, TradeType};

use super::binary_reader::BinaryReader;
use super::constants::{
    discriminators::pumpfun_events, PUMP_FUN_PROGRAM_NAME, PUMP_SWAP_PROGRAM_NAME, SOL_MINT,
};
use super::util::{
    build_token_info, get_instruction_data, get_prev_instruction_by_index, get_trade_type,
    sort_by_idx, HasIdx,
};

use crate::core::transaction_adapter::TransactionAdapter;

pub struct PumpfunEventParser {
    adapter: TransactionAdapter,
}

impl PumpfunEventParser {
    pub fn new(adapter: TransactionAdapter) -> Self {
        Self { adapter }
    }

    pub fn parse_instructions(
        &self,
        instructions: &[ClassifiedInstruction],
    ) -> Result<Vec<MemeEvent>> {
        let mut events = Vec::new();
        for classified in instructions {
            let data = get_instruction_data(&classified.data)?;
            if data.len() < 16 {
                continue;
            }
            let discriminator = &data[..16];
            let payload = data[16..].to_vec();

            let event = if discriminator == pumpfun_events::TRADE {
                Some(self.decode_trade_event(payload)?)
            } else if discriminator == pumpfun_events::CREATE {
                Some(self.decode_create_event(payload)?)
            } else if discriminator == pumpfun_events::COMPLETE {
                Some(self.decode_complete_event(payload)?)
            } else if discriminator == pumpfun_events::MIGRATE {
                Some(self.decode_migrate_event(payload)?)
            } else {
                None
            };

            if let Some(mut meme_event) = event {
                if meme_event.event_type == TradeType::Buy
                    || meme_event.event_type == TradeType::Sell
                {
                    if let Some(prev) = get_prev_instruction_by_index(
                        instructions,
                        classified.outer_index,
                        classified.inner_index,
                    ) {
                        if let Some(account) = prev.data.accounts.get(3) {
                            meme_event.bonding_curve = Some(account.clone());
                        }
                    }
                }
                meme_event.signature = self.adapter.signature().to_string();
                meme_event.slot = self.adapter.slot();
                meme_event.timestamp = self.adapter.block_time();
                meme_event.idx = format!(
                    "{}-{}",
                    classified.outer_index,
                    classified.inner_index.unwrap_or(0)
                );
                events.push(meme_event);
            }
        }

        Ok(sort_by_idx(events))
    }

    fn decode_trade_event(&self, data: Vec<u8>) -> Result<MemeEvent> {
        let mut reader = BinaryReader::new(data);
        let mint = reader.read_pubkey()?;
        let quote_mint = SOL_MINT.to_string();
        let sol_amount = reader.read_u64()? as u128;
        let token_amount = reader.read_u64()? as u128;
        let is_buy = reader.read_u8()? == 1;
        let user = bs58_encode(reader.read_fixed_array(32)?).into_string();
        let timestamp = reader.read_i64()? as u64;
        let _virtual_sol = reader.read_u64()?;
        let _virtual_token = reader.read_u64()?;

        let mut fee_basis_points = None;
        let mut fee = None;
        let mut creator = None;
        let mut creator_fee_basis_points = None;
        let mut creator_fee = None;

        if reader.remaining() >= 52 {
            let _real_sol_reserves = reader.read_u64()?;
            let _real_token_reserves = reader.read_u64()?;
            let _fee_recipient = reader.read_pubkey()?;
            fee_basis_points = Some(reader.read_u16()? as u64);
            fee = Some(reader.read_u64()?);
            creator = Some(reader.read_pubkey()?);
            creator_fee_basis_points = Some(reader.read_u16()? as u64);
            creator_fee = Some(reader.read_u64()?);
        }

        let (input_mint, input_amount, input_decimals, output_mint, output_amount, output_decimals) =
            if is_buy {
                (
                    quote_mint.clone(),
                    sol_amount,
                    9,
                    mint.clone(),
                    token_amount,
                    6,
                )
            } else {
                (
                    mint.clone(),
                    token_amount,
                    6,
                    quote_mint.clone(),
                    sol_amount,
                    9,
                )
            };

        let input_token = build_token_info(
            &input_mint,
            input_amount,
            input_decimals,
            Some(user.clone()),
        );
        let output_token = build_token_info(
            &output_mint,
            output_amount,
            output_decimals,
            Some(user.clone()),
        );

        let event = MemeEvent {
            event_type: get_trade_type(&input_mint, &output_mint),
            timestamp,
            idx: String::new(),
            slot: 0,
            signature: String::new(),
            user: user.clone(),
            base_mint: mint,
            quote_mint,
            input_token: Some(input_token),
            output_token: Some(output_token),
            name: None,
            symbol: None,
            uri: None,
            decimals: None,
            total_supply: None,
            fee: fee.map(|v| v as f64),
            protocol_fee: fee_basis_points.map(|bps| bps as f64),
            platform_fee: None,
            share_fee: None,
            creator_fee: creator_fee.map(|v| v as f64),
            protocol: Some(PUMP_FUN_PROGRAM_NAME.to_string()),
            platform_config: None,
            creator,
            bonding_curve: None,
            pool: None,
            pool_dex: None,
            pool_a_reserve: None,
            pool_b_reserve: None,
            pool_fee_rate: creator_fee_basis_points.map(|bps| bps as f64),
        };

        Ok(event)
    }

    fn decode_create_event(&self, data: Vec<u8>) -> Result<MemeEvent> {
        let mut reader = BinaryReader::new(data);
        let name = reader.read_string()?;
        let symbol = reader.read_string()?;
        let uri = reader.read_string()?;
        let mint = bs58_encode(reader.read_fixed_array(32)?).into_string();
        let bonding_curve = bs58_encode(reader.read_fixed_array(32)?).into_string();
        let user = bs58_encode(reader.read_fixed_array(32)?).into_string();

        let mut creator = None;
        let mut timestamp = self.adapter.block_time();
        if reader.remaining() >= 16 {
            creator = Some(reader.read_pubkey()?);
            let ts = reader.read_i64()?;
            if ts >= 0 {
                timestamp = ts as u64;
            }
        }
        let mut virtual_token_reserves = None;
        let mut virtual_sol_reserves = None;
        let mut real_token_reserves = None;
        let mut token_total_supply = None;
        if reader.remaining() >= 32 {
            virtual_token_reserves = Some(reader.read_u64()? as f64);
            virtual_sol_reserves = Some(reader.read_u64()? as f64);
            real_token_reserves = Some(reader.read_u64()? as f64);
            token_total_supply = Some(reader.read_u64()?);
        }

        Ok(MemeEvent {
            event_type: TradeType::Create,
            timestamp,
            idx: String::new(),
            slot: 0,
            signature: String::new(),
            user,
            base_mint: mint,
            quote_mint: SOL_MINT.to_string(),
            input_token: None,
            output_token: None,
            name: Some(name),
            symbol: Some(symbol),
            uri: Some(uri),
            decimals: None,
            total_supply: token_total_supply,
            fee: None,
            protocol_fee: None,
            platform_fee: None,
            share_fee: None,
            creator_fee: None,
            protocol: Some(PUMP_FUN_PROGRAM_NAME.to_string()),
            platform_config: None,
            creator,
            bonding_curve: Some(bonding_curve),
            pool: None,
            pool_dex: None,
            pool_a_reserve: virtual_token_reserves,
            pool_b_reserve: virtual_sol_reserves,
            pool_fee_rate: real_token_reserves,
        })
    }

    fn decode_complete_event(&self, data: Vec<u8>) -> Result<MemeEvent> {
        let mut reader = BinaryReader::new(data);
        let user = bs58_encode(reader.read_fixed_array(32)?).into_string();
        let mint = bs58_encode(reader.read_fixed_array(32)?).into_string();
        let bonding_curve = bs58_encode(reader.read_fixed_array(32)?).into_string();
        let ts = reader.read_i64()?;
        let timestamp = if ts >= 0 { ts as u64 } else { 0 };

        Ok(MemeEvent {
            event_type: TradeType::Complete,
            timestamp,
            idx: String::new(),
            slot: 0,
            signature: String::new(),
            user,
            base_mint: mint,
            quote_mint: SOL_MINT.to_string(),
            input_token: None,
            output_token: None,
            name: None,
            symbol: None,
            uri: None,
            decimals: None,
            total_supply: None,
            fee: None,
            protocol_fee: None,
            platform_fee: None,
            share_fee: None,
            creator_fee: None,
            protocol: Some(PUMP_FUN_PROGRAM_NAME.to_string()),
            platform_config: None,
            creator: None,
            bonding_curve: Some(bonding_curve),
            pool: None,
            pool_dex: None,
            pool_a_reserve: None,
            pool_b_reserve: None,
            pool_fee_rate: None,
        })
    }

    fn decode_migrate_event(&self, data: Vec<u8>) -> Result<MemeEvent> {
        let mut reader = BinaryReader::new(data);
        let user = bs58_encode(reader.read_fixed_array(32)?).into_string();
        let mint = bs58_encode(reader.read_fixed_array(32)?).into_string();
        let _mint_amount = reader.read_u64()?;
        let _sol_amount = reader.read_u64()?;
        let pool_migrate_fee = reader.read_u64()? as u128;
        let bonding_curve = bs58_encode(reader.read_fixed_array(32)?).into_string();
        let ts = reader.read_i64()?;
        let timestamp = if ts >= 0 { ts as u64 } else { 0 };
        let pool = reader.read_pubkey()?;

        Ok(MemeEvent {
            event_type: TradeType::Migrate,
            timestamp,
            idx: String::new(),
            slot: 0,
            signature: String::new(),
            user,
            base_mint: mint,
            quote_mint: SOL_MINT.to_string(),
            input_token: None,
            output_token: None,
            name: None,
            symbol: None,
            uri: None,
            decimals: None,
            total_supply: None,
            fee: Some(pool_migrate_fee as f64),
            protocol_fee: None,
            platform_fee: None,
            share_fee: None,
            creator_fee: None,
            protocol: Some(PUMP_FUN_PROGRAM_NAME.to_string()),
            platform_config: None,
            creator: None,
            bonding_curve: Some(bonding_curve),
            pool: Some(pool.clone()),
            pool_dex: Some(PUMP_SWAP_PROGRAM_NAME.to_string()),
            pool_a_reserve: None,
            pool_b_reserve: None,
            pool_fee_rate: None,
        })
    }
}

impl HasIdx for MemeEvent {
    fn idx(&self) -> &str {
        &self.idx
    }
}
