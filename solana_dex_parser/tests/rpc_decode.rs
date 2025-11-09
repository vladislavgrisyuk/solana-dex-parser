use std::collections::HashMap;
use std::str::FromStr;

use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_transaction_status::{
    EncodedConfirmedTransactionWithStatusMeta, EncodedTransaction, UiCompiledInstruction, UiInstruction,
    UiMessage, UiParsedInstruction, UiTransactionEncoding, UiTransactionStatusMeta,
};
use solana_dex_parser::{DexParser, ParseConfig, SolanaTransaction};
use solana_dex_parser::types::{BalanceChange, SolanaInstruction, TransactionMeta, TransactionStatus};

#[test]
#[ignore]
fn fetch_and_decode_live_transaction() -> Result<()> {
    let rpc_url = std::env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
    let requested_signature = std::env::var("24GCzszYAuKnAi6pTnVm8e2gntfAgR86wgeu5SVc3gBMhLV8ugsGXRGFCdw2d428wu7TGFuAM2D7oLJPujHje1PX").ok();

    let tx = fetch_transaction(&rpc_url, requested_signature)?;
    let parser = DexParser::new();
    let result = parser.parse_all(tx, Some(ParseConfig::default()));

    // Help manual debugging by showing a readable summary of what we parsed.
    let summary: Value = serde_json::to_value(&result)?;
    println!("Parsed result summary: {}", serde_json::to_string_pretty(&summary)?);

    // We do not assert on concrete trade output, but the parser should at least produce metadata.
    assert!(!result.signature.is_empty());
    assert!(result.slot > 0);

    Ok(())
}

fn fetch_transaction(rpc_url: &str, explicit_signature: Option<String>) -> Result<SolanaTransaction> {
    let client = RpcClient::new(rpc_url.to_string());
    let signature = if let Some(sig) = explicit_signature {
        Signature::from_str(&sig).context("invalid SOLANA_TX_SIGNATURE")?
    } else {
        fetch_recent_signature(&client)?
    };

    let config = RpcTransactionConfig {
        encoding: Some(UiTransactionEncoding::Json),
        commitment: Some(CommitmentConfig::confirmed()),
        max_supported_transaction_version: Some(0),
    };
    let encoded = client
        .get_transaction_with_config(&signature, config)
        .with_context(|| format!("failed to fetch transaction {signature}"))?;

    convert_transaction(encoded)
}

fn fetch_recent_signature(client: &RpcClient) -> Result<Signature> {
    let address = Pubkey::from_str("11111111111111111111111111111111")?;
    let mut signatures = client.get_signatures_for_address(&address)?;
    let sig = signatures
        .drain(..)
        .next()
        .context("no signatures returned for system program")?;
    Signature::from_str(&sig.signature).context("invalid signature from RPC response")
}

fn convert_transaction(tx: EncodedConfirmedTransactionWithStatusMeta) -> Result<SolanaTransaction> {
    let meta = tx
        .transaction
        .meta
        .as_ref()
        .context("transaction missing status meta")?;
    let (instructions, account_keys, signers, signature) = extract_message(&tx.transaction.transaction)?;

    let solana_tx = SolanaTransaction {
        slot: tx.slot,
        signature,
        block_time: tx.block_time.unwrap_or_default() as u64,
        signers,
        instructions,
        transfers: Vec::new(),
        meta: TransactionMeta {
            fee: meta.fee,
            compute_units: Option::<u64>::from(meta.compute_units_consumed.clone()).unwrap_or(0),
            status: if meta.err.is_some() {
                TransactionStatus::Failed
            } else {
                TransactionStatus::Success
            },
            sol_balance_changes: collect_sol_balance_changes(meta, &account_keys),
            token_balance_changes: HashMap::new(),
        },
    };

    Ok(solana_tx)
}

fn collect_sol_balance_changes(
    meta: &UiTransactionStatusMeta,
    account_keys: &[String],
) -> HashMap<String, BalanceChange> {
    let mut changes = HashMap::new();
    for (idx, key) in account_keys.iter().enumerate() {
        if let (Some(pre), Some(post)) = (meta.pre_balances.get(idx), meta.post_balances.get(idx)) {
            if pre != post {
                changes.insert(
                    key.clone(),
                    BalanceChange {
                        pre: *pre as i128,
                        post: *post as i128,
                        change: *post as i128 - *pre as i128,
                    },
                );
            }
        }
    }
    changes
}

fn extract_message(
    encoded: &EncodedTransaction,
) -> Result<(Vec<SolanaInstruction>, Vec<String>, Vec<String>, String)> {
    let ui_tx = match encoded {
        EncodedTransaction::Json(tx) => tx,
        _ => return Err(anyhow!("expected JSON encoded transaction")),
    };
    let signature = ui_tx
        .signatures
        .first()
        .cloned()
        .ok_or_else(|| anyhow!("transaction missing signature"))?;

    match &ui_tx.message {
        UiMessage::Raw(raw) => {
            let instructions = raw
                .instructions
                .iter()
                .map(|ix| convert_compiled_instruction(ix, &raw.account_keys))
                .collect();
            let signers = raw
                .account_keys
                .iter()
                .take(raw.header.num_required_signatures as usize)
                .cloned()
                .collect();
            Ok((instructions, raw.account_keys.clone(), signers, signature))
        }
        UiMessage::Parsed(parsed) => {
            let account_keys: Vec<String> = parsed
                .account_keys
                .iter()
                .map(|account| account.pubkey.clone())
                .collect();
            let instructions = parsed
                .instructions
                .iter()
                .map(|ix| convert_parsed_instruction(ix, &account_keys))
                .collect();
            let signers = parsed
                .account_keys
                .iter()
                .filter(|account| account.signer)
                .map(|account| account.pubkey.clone())
                .collect();
            Ok((instructions, account_keys, signers, signature))
        }
    }
}

fn convert_compiled_instruction(
    instruction: &UiCompiledInstruction,
    account_keys: &[String],
) -> SolanaInstruction {
    let program_id = account_keys
        .get(instruction.program_id_index as usize)
        .cloned()
        .unwrap_or_default();
    let accounts = instruction
        .accounts
        .iter()
        .filter_map(|index| account_keys.get(*index as usize).cloned())
        .collect();
    SolanaInstruction {
        program_id,
        accounts,
        data: instruction.data.clone(),
    }
}

fn convert_parsed_instruction(
    instruction: &UiInstruction,
    account_keys: &[String],
) -> SolanaInstruction {
    match instruction {
        UiInstruction::Compiled(compiled) => convert_compiled_instruction(compiled, account_keys),
        UiInstruction::Parsed(parsed) => match parsed {
            UiParsedInstruction::PartiallyDecoded(instruction) => SolanaInstruction {
                program_id: instruction.program_id.clone(),
                accounts: instruction.accounts.clone(),
                data: instruction.data.clone(),
            },
            UiParsedInstruction::Parsed(instruction) => SolanaInstruction {
                program_id: instruction.program_id.clone(),
                accounts: Vec::new(),
                data: instruction.parsed.to_string(),
            },
        },
    }
}
