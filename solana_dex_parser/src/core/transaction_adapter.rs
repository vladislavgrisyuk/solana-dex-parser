use std::collections::{HashMap, HashSet};

use crate::constants::{SPL_TOKEN_INSTRUCTION_TYPES, TOKEN_2022_PROGRAM_ID, TOKEN_PROGRAM_ID, TOKENS};
use crate::types::{
    BalanceChange, ParseConfig, PoolEventType, SolanaInstruction, SolanaTransaction, TokenAmount,
    TokenBalance, TokenInfo, TransactionStatus,
};
use crate::utils::{decode_instruction_data, get_instruction_data, get_program_name};

/// Унифицированный адаптер доступа к данным транзакции (аналог TS TransactionAdapter)
#[derive(Clone, Debug)]
pub struct TransactionAdapter {
    tx: SolanaTransaction,
    config: Option<ParseConfig>,

    /// Аналог TS: accountKeys[]
    account_keys: Vec<String>,

    /// Аналог TS: splTokenMap (карта: токен-аккаунт → инфо о токене)
    spl_token_map: HashMap<String, TokenInfo>,

    /// Аналог TS: splDecimalsMap (карта: mint → decimals)
    spl_decimals_map: HashMap<String, u8>,
}

impl TransactionAdapter {
    pub fn new(tx: SolanaTransaction, config: Option<ParseConfig>) -> Self {
        let account_keys = Self::extract_account_keys(&tx);
        let mut adapter = Self {
            tx,
            config,
            account_keys,
            spl_token_map: HashMap::new(),
            spl_decimals_map: HashMap::new(),
        };

        adapter.extract_token_info(); // заполняем карты токенов, как в TS-конструкторе
        adapter
    }

    // ===== Соответствие геттерам из TS =====

    pub fn tx_message(&self) -> &crate::types::TxMessage {
        &self.tx.message
    }

    pub fn is_message_v0(&self) -> bool {
        self.tx.message.is_v0()
    }

    /// slot
    pub fn slot(&self) -> u64 {
        self.tx.slot
    }

    /// version
    pub fn version(&self) -> crate::types::Version {
        self.tx.version
    }

    /// blockTime
    pub fn block_time(&self) -> u64 {
        self.tx.block_time.unwrap_or(0)
    }

    /// signature
    pub fn signature(&self) -> &str {
        &self.tx.signature
    }

    /// instructions (outer)
    pub fn instructions(&self) -> &[SolanaInstruction] {
        &self.tx.instructions
    }

    /// innerInstructions
    pub fn inner_instructions(&self) -> &[crate::types::InnerInstruction] {
        &self.tx.inner_instructions
    }

    /// preBalances
    pub fn pre_balances(&self) -> Option<&[u64]> {
        self.tx.meta.pre_balances.as_deref()
    }

    /// postBalances
    pub fn post_balances(&self) -> Option<&[u64]> {
        self.tx.meta.post_balances.as_deref()
    }

    /// preTokenBalances
    pub fn pre_token_balances(&self) -> Option<&[TokenBalance]> {
        self.tx.pre_token_balances.as_deref()
    }

    /// postTokenBalances
    pub fn post_token_balances(&self) -> Option<&[TokenBalance]> {
        self.tx.post_token_balances.as_deref()
    }

    /// первый подписант
    pub fn signer(&self) -> String {
        self.signers().get(0).cloned().unwrap_or_else(|| "".to_string())
    }

    /// signers[]
    pub fn signers(&self) -> Vec<String> {
        // В TS берётся из message.header.numRequiredSignatures.
        // В нормализованных типах обычно уже есть signers. Если их нет — берём из self.account_keys[0..n]
        if let Some(n) = self.tx.message.num_required_signatures() {
            return self.account_keys.iter().take(n as usize).cloned().collect();
        }
        self.tx.signers.clone().or_else(|| {
            if !self.account_keys.is_empty() {
                Some(vec![self.account_keys[0].clone()])
            } else {
                Some(vec![])
            }
        }).unwrap_or_default()
    }

    /// fee()
    pub fn fee(&self) -> TokenAmount {
        let fee = self.tx.meta.fee.unwrap_or(0);
        TokenAmount {
            amount: fee.to_string(),
            ui_amount: Some(Self::convert_to_ui_amount(&fee.to_string(), 9)),
            decimals: 9,
        }
    }

    /// computeUnits
    pub fn compute_units(&self) -> u64 {
        self.tx.meta.compute_units_consumed.unwrap_or(0)
    }

    /// txStatus: success/failed/unknown
    pub fn tx_status(&self) -> TransactionStatus {
        if self.tx.meta.err.is_none() {
            if self.tx.meta.pre_balances.is_some() || self.tx.meta.post_balances.is_some() {
                TransactionStatus::Success
            } else {
                TransactionStatus::Unknown
            }
        } else {
            TransactionStatus::Failed
        }
    }

    // ===== Account keys (аналог TS extractAccountKeys) =====

    fn extract_account_keys(tx: &SolanaTransaction) -> Vec<String> {
        // Пытаемся собрать уникальные ключи: из message (статические) + из loaded addresses,
        // плюс все аккаунты встречающиеся в инструкциях/inner-инструкциях (на случай отсутствия адрес-таблиц)
        let mut set: HashSet<String> = HashSet::new();

        // из message (v0/legacy)
        for k in tx.message.static_account_keys() {
            set.insert(k.clone());
        }
        for k in tx.message.loaded_writable() {
            set.insert(k.clone());
        }
        for k in tx.message.loaded_readonly() {
            set.insert(k.clone());
        }

        // из outer instructions (индексы уже должны быть резолвнуты до баз58 строк в нормализаторе)
        for ix in &tx.instructions {
            for acc in &ix.accounts {
                set.insert(acc.clone());
            }
            set.insert(ix.program_id.clone());
        }

        // из inner
        for inner in &tx.inner_instructions {
            for ix in &inner.instructions {
                for acc in &ix.accounts {
                    set.insert(acc.clone());
                }
                set.insert(ix.program_id.clone());
            }
        }

        let mut out: Vec<String> = set.into_iter().collect();
        out.sort();
        out
    }

    pub fn address_table_lookups(&self) -> &[crate::types::AddressTableLookup] {
        self.tx.message.address_table_lookups()
    }

    pub fn address_table_lookup_keys(&self) -> Vec<String> {
        self.address_table_lookups()
            .iter()
            .map(|l| l.account_key.clone())
            .collect()
    }

    // ===== Унификация одной инструкции (аналог getInstruction) =====

    /// Вернёт унифицированную инструкцию с декодированным data (как TS getInstruction)
    pub fn get_instruction(&self, instruction: &SolanaInstruction) -> SolanaInstruction {
        let mut ix = instruction.clone();
        // Если исходный нормализатор не заполнял data как bytes — можно декодировать здесь
        // (в проекте, вероятно, уже bytes). Оставлю вызов на случай совместимости:
        if ix.data.is_empty() && !ix.encoded_data.is_empty() {
            ix.data = decode_instruction_data(&ix.encoded_data);
        }
        ix
    }

    pub fn get_inner_instruction(&self, outer_index: usize, inner_index: usize) -> Option<&SolanaInstruction> {
        self.inner_instructions()
            .iter()
            .find(|s| s.index == outer_index)
            .and_then(|s| s.instructions.get(inner_index))
    }

    /// Возвращает список аккаунтов инструкции
    pub fn get_instruction_accounts<'a>(&self, instruction: &'a SolanaInstruction) -> &'a [String] {
        &instruction.accounts
    }

    /// Compiled или Parsed (в TS это проверка по наличию programIdIndex/parsed)
    pub fn is_compiled_instruction(&self, instruction: &SolanaInstruction) -> bool {
        instruction.parsed.is_none()
    }

    /// Тип инструкции: если parsed → name, иначе первый байт data как строка (аналог TS getInstructionType)
    pub fn get_instruction_type(&self, instruction: &SolanaInstruction) -> Option<String> {
        if let Some(parsed) = &instruction.parsed {
            return parsed.type_name.clone();
        }
        let data = get_instruction_data(instruction);
        data.first().map(|b| b.to_string())
    }

    /// programId (аналог TS getInstructionProgramId)
    pub fn get_instruction_program_id<'a>(&self, instruction: &'a SolanaInstruction) -> &'a str {
        &instruction.program_id
    }

    /// Получить ключ по индексу (если список есть). Для совместимости с TS.
    pub fn get_account_key(&self, index: usize) -> String {
        self.account_keys.get(index).cloned().unwrap_or_default()
    }

    /// Индекс адреса в account_keys
    pub fn get_account_index(&self, address: &str) -> Option<usize> {
        self.account_keys.iter().position(|k| k == address)
    }

    // ===== Владелец токен-аккаунта (аналог TS getTokenAccountOwner) =====

    pub fn get_token_account_owner(&self, account_key: &str) -> Option<String> {
        if let Some(post) = self.post_token_balances() {
            if let Some(b) = post.iter().find(|b| b.account == account_key) {
                return b.owner.clone();
            }
        }
        if let Some(pre) = self.pre_token_balances() {
            if let Some(b) = pre.iter().find(|b| b.account == account_key) {
                return b.owner.clone();
            }
        }
        None
    }

    // ===== Балансы (SOL / токены) — как в TS =====

    pub fn get_account_balance(&self, account_keys: &[String]) -> Vec<Option<TokenAmount>> {
        account_keys
            .iter()
            .map(|key| {
                let idx = self.get_account_index(key)?;
                let post = self.post_balances()?.get(idx).copied().unwrap_or(0);
                Some(TokenAmount {
                    amount: post.to_string(),
                    ui_amount: Some(Self::convert_to_ui_amount(&post.to_string(), 9)),
                    decimals: 9,
                })
            })
            .collect()
    }

    pub fn get_account_pre_balance(&self, account_keys: &[String]) -> Vec<Option<TokenAmount>> {
        account_keys
            .iter()
            .map(|key| {
                let idx = self.get_account_index(key)?;
                let pre = self.pre_balances()?.get(idx).copied().unwrap_or(0);
                Some(TokenAmount {
                    amount: pre.to_string(),
                    ui_amount: Some(Self::convert_to_ui_amount(&pre.to_string(), 9)),
                    decimals: 9,
                })
            })
            .collect()
    }

    pub fn get_token_account_balance(&self, account_keys: &[String]) -> Vec<Option<TokenAmount>> {
        let post = match self.post_token_balances() {
            Some(v) => v,
            None => return vec![None; account_keys.len()],
        };

        account_keys
            .iter()
            .map(|key| {
                post.iter()
                    .find(|b| &b.account == key)
                    .map(|b| b.ui_token_amount.clone())
            })
            .collect()
    }

    pub fn get_token_account_pre_balance(&self, account_keys: &[String]) -> Vec<Option<TokenAmount>> {
        let pre = match self.pre_token_balances() {
            Some(v) => v,
            None => return vec![None; account_keys.len()],
        };

        account_keys
            .iter()
            .map(|key| {
                pre.iter()
                    .find(|b| &b.account == key)
                    .map(|b| b.ui_token_amount.clone())
            })
            .collect()
    }

    // ===== Карты токенов (mint/decimals) — как в TS =====

    pub fn is_supported_token(&self, mint: &str) -> bool {
        TOKENS.values().any(|m| m == &mint)
    }

    pub fn get_instruction_program_id_string(&self, instruction: &SolanaInstruction) -> String {
        self.get_instruction_program_id(instruction).to_string()
    }

    pub fn get_token_decimals(&self, mint: &str) -> u8 {
        *self.spl_decimals_map.get(mint).unwrap_or(&0)
    }

    pub fn get_pool_event_base(&self, r#type: PoolEventType, program_id: &str) -> crate::types::PoolEventBase {
        crate::types::PoolEventBase {
            user: self.signer(),
            r#type,
            program_id: program_id.to_string(),
            amm: get_program_name(program_id),
            slot: self.slot(),
            timestamp: self.block_time(),
            signature: self.signature().to_string(),
        }
    }

    // ===== Баланс-дифы по всем аккаунтам (как TS getAccountSolBalanceChanges / getAccountTokenBalanceChanges) =====

    pub fn get_account_sol_balance_changes(&self, is_owner: bool) -> HashMap<String, BalanceChange> {
        let mut changes: HashMap<String, BalanceChange> = HashMap::new();
        let Some(pre) = self.pre_balances() else { return changes; };
        let Some(post) = self.post_balances() else { return changes; };

        for (index, key) in self.account_keys.iter().enumerate() {
            let account_key = if is_owner {
                self.get_token_account_owner(key).unwrap_or_else(|| key.clone())
            } else {
                key.clone()
            };

            let pre_balance = pre.get(index).copied().unwrap_or(0);
            let post_balance = post.get(index).copied().unwrap_or(0);
            let change = (post_balance as i128) - (pre_balance as i128);
            if change == 0 {
                continue;
            }
            let to_amount = |lamports: u64| TokenAmount {
                amount: lamports.to_string(),
                ui_amount: Some(Self::convert_to_ui_amount(&lamports.to_string(), 9)),
                decimals: 9,
            };

            changes.insert(
                account_key,
                BalanceChange {
                    pre: to_amount(pre_balance),
                    post: to_amount(post_balance),
                    change: TokenAmount {
                        amount: change.to_string(),
                        ui_amount: Some(change as f64 / 1e9),
                        decimals: 9,
                    },
                },
            );
        }
        changes
    }

    pub fn get_account_token_balance_changes(&self, is_owner: bool) -> HashMap<String, HashMap<String, BalanceChange>> {
        let mut changes: HashMap<String, HashMap<String, BalanceChange>> = HashMap::new();

        // pre
        if let Some(pre) = self.pre_token_balances() {
            for b in pre {
                if b.mint.is_empty() {
                    continue;
                }
                let key = &b.account;
                let account_key = if is_owner {
                    self.get_token_account_owner(key).unwrap_or_else(|| key.clone())
                } else {
                    key.clone()
                };

                let entry = changes.entry(account_key).or_default();
                entry.entry(b.mint.clone()).or_insert(BalanceChange {
                    pre: b.ui_token_amount.clone(),
                    post: TokenAmount {
                        amount: "0".into(),
                        ui_amount: Some(0.0),
                        decimals: b.ui_token_amount.decimals,
                    },
                    change: TokenAmount {
                        amount: "0".into(),
                        ui_amount: Some(0.0),
                        decimals: b.ui_token_amount.decimals,
                    },
                });
            }
        }

        // post + diff
        if let Some(post) = self.post_token_balances() {
            for b in post {
                if b.mint.is_empty() {
                    continue;
                }
                let key = &b.account;
                let account_key = if is_owner {
                    self.get_token_account_owner(key).unwrap_or_else(|| key.clone())
                } else {
                    key.clone()
                };

                let entry = changes.entry(account_key).or_default();
                if let Some(ch) = entry.get_mut(&b.mint) {
                    // считаем дельту raw amounts
                    let pre_raw = ch.pre.amount.parse::<i128>().unwrap_or(0);
                    let post_raw = b.ui_token_amount.amount.parse::<i128>().unwrap_or(0);
                    let diff = post_raw - pre_raw;

                    ch.post = b.ui_token_amount.clone();
                    ch.change = TokenAmount {
                        amount: diff.to_string(),
                        ui_amount: Some(
                            b.ui_token_amount.ui_amount.unwrap_or(0.0) - ch.pre.ui_amount.unwrap_or(0.0),
                        ),
                        decimals: b.ui_token_amount.decimals,
                    };

                    if diff == 0 {
                        entry.remove(&b.mint);
                    }
                } else {
                    // pre не было, считаем pre = 0
                    entry.insert(
                        b.mint.clone(),
                        BalanceChange {
                            pre: TokenAmount {
                                amount: "0".into(),
                                ui_amount: Some(0.0),
                                decimals: b.ui_token_amount.decimals,
                            },
                            post: b.ui_token_amount.clone(),
                            change: b.ui_token_amount.clone(),
                        },
                    );
                }
            }
        }

        // почистим пустые
        changes.retain(|_, m| !m.is_empty());
        changes
    }

    // ===== Внутренняя логика извлечения токенов (как в TS extractTokenInfo) =====

    fn extract_token_info(&mut self) {
        self.extract_token_balances();
        self.extract_token_from_instructions();

        // Добавляем SOL, если нет
        if !self.spl_token_map.contains_key(TOKENS.SOL) {
            self.spl_token_map.insert(
                TOKENS.SOL.to_string(),
                TokenInfo {
                    mint: TOKENS.SOL.to_string(),
                    amount: 0.0,
                    amount_raw: "0".into(),
                    decimals: 9,
                    ..TokenInfo::default()
                },
            );
        }
        if !self.spl_decimals_map.contains_key(TOKENS.SOL) {
            self.spl_decimals_map.insert(TOKENS.SOL.to_string(), 9);
        }
    }

    /// Аналог TS extractTokenBalances()
    fn extract_token_balances(&mut self) {
        if let Some(post) = self.post_token_balances() {
            for balance in post {
                if balance.mint.is_empty() {
                    continue;
                }
                let account_key = &balance.account;
                if !self.spl_token_map.contains_key(account_key) {
                    let token_info = TokenInfo {
                        mint: balance.mint.clone(),
                        amount: balance.ui_token_amount.ui_amount.unwrap_or(0.0),
                        amount_raw: balance.ui_token_amount.amount.clone(),
                        decimals: balance.ui_token_amount.decimals,
                        ..TokenInfo::default()
                    };
                    self.spl_token_map.insert(account_key.clone(), token_info);
                }
                self.spl_decimals_map
                    .entry(balance.mint.clone())
                    .or_insert(balance.ui_token_amount.decimals);
            }
        }
    }

    /// Аналог TS extractTokenFromInstructions()
    fn extract_token_from_instructions(&mut self) {
        // outer
        for ix in self.instructions() {
            if self.is_compiled_instruction(ix) {
                self.extract_from_compiled_transfer(ix);
            } else {
                self.extract_from_parsed_transfer(ix);
            }
        }
        // inner
        for inner in self.inner_instructions() {
            for ix in &inner.instructions {
                if self.is_compiled_instruction(ix) {
                    self.extract_from_compiled_transfer(ix);
                } else {
                    self.extract_from_parsed_transfer(ix);
                }
            }
        }
    }

    /// Аналог TS setTokenInfo()
    fn set_token_info(
        &mut self,
        source: Option<&str>,
        destination: Option<&str>,
        mint: Option<&str>,
        decimals: Option<u8>,
    ) {
        if let Some(src) = source {
            if self.spl_token_map.contains_key(src) && mint.is_some() && decimals.is_some() {
                self.spl_token_map.insert(
                    src.to_string(),
                    TokenInfo {
                        mint: mint.unwrap().to_string(),
                        amount: 0.0,
                        amount_raw: "0".into(),
                        decimals: decimals.unwrap(),
                        ..TokenInfo::default()
                    },
                );
            } else if !self.spl_token_map.contains_key(src) {
                self.spl_token_map.insert(
                    src.to_string(),
                    TokenInfo {
                        mint: mint.unwrap_or(TOKENS.SOL).to_string(),
                        amount: 0.0,
                        amount_raw: "0".into(),
                        decimals: decimals.unwrap_or(9),
                        ..TokenInfo::default()
                    },
                );
            }
        }

        if let Some(dst) = destination {
            if self.spl_token_map.contains_key(dst) && mint.is_some() && decimals.is_some() {
                self.spl_token_map.insert(
                    dst.to_string(),
                    TokenInfo {
                        mint: mint.unwrap().to_string(),
                        amount: 0.0,
                        amount_raw: "0".into(),
                        decimals: decimals.unwrap(),
                        ..TokenInfo::default()
                    },
                );
            } else if !self.spl_token_map.contains_key(dst) {
                self.spl_token_map.insert(
                    dst.to_string(),
                    TokenInfo {
                        mint: mint.unwrap_or(TOKENS.SOL).to_string(),
                        amount: 0.0,
                        amount_raw: "0".into(),
                        decimals: decimals.unwrap_or(9),
                        ..TokenInfo::default()
                    },
                );
            }
        }

        if let (Some(m), Some(d)) = (mint, decimals) {
            self.spl_decimals_map.entry(m.to_string()).or_insert(d);
        }
    }

    /// Аналог TS extractFromParsedTransfer()
    fn extract_from_parsed_transfer(&mut self, ix: &SolanaInstruction) {
        if ix.parsed.is_none() || ix.program_id.is_empty() {
            return;
        }
        let pid = &ix.program_id;
        if pid != TOKEN_PROGRAM_ID && pid != TOKEN_2022_PROGRAM_ID {
            return;
        }
        let info = match &ix.parsed {
            Some(p) => &p.info,
            None => return,
        };

        let source = info.source.as_deref();
        let destination = info.destination.as_deref();
        let mint = info.mint.as_deref();
        let decimals = info.decimals;

        if source.is_none() && destination.is_none() {
            return;
        }
        self.set_token_info(source, destination, mint, decimals);
    }

    /// Аналог TS extractFromCompiledTransfer()
    fn extract_from_compiled_transfer(&mut self, ix: &SolanaInstruction) {
        // bytes (у нас уже Vec<u8>)
        let decoded = get_instruction_data(ix);
        if decoded.is_empty() {
            return;
        }
        let program_id = &ix.program_id;
        if program_id != TOKEN_PROGRAM_ID && program_id != TOKEN_2022_PROGRAM_ID {
            return;
        }

        let accounts = &ix.accounts;
        if accounts.is_empty() {
            return;
        }

        // NOTE: в TS accounts — индексы в accountKeys; в наших нормализованных типах уже строки-адреса.
        // Поэтому берём их как есть по позициям.
        let mut source: Option<&str> = None;
        let mut destination: Option<&str> = None;
        let mut mint: Option<&str> = None;
        let mut decimals: Option<u8> = None;

        match decoded[0] {
            x if x == SPL_TOKEN_INSTRUCTION_TYPES.Transfer => {
                if accounts.len() < 2 {
                    return;
                }
                source = Some(&accounts[0]);
                destination = Some(&accounts[1]);
            }
            x if x == SPL_TOKEN_INSTRUCTION_TYPES.TransferChecked => {
                if accounts.len() < 3 {
                    return;
                }
                source = Some(&accounts[0]);
                mint = Some(&accounts[1]);
                destination = Some(&accounts[2]);
                // byte offset 9 как в TS
                decimals = decoded.get(9).copied();
            }
            x if x == SPL_TOKEN_INSTRUCTION_TYPES.InitializeMint => {
                if accounts.len() < 2 {
                    return;
                }
                mint = Some(&accounts[0]);
                destination = Some(&accounts[1]);
            }
            x if x == SPL_TOKEN_INSTRUCTION_TYPES.MintTo => {
                if accounts.len() < 2 {
                    return;
                }
                mint = Some(&accounts[0]);
                destination = Some(&accounts[1]);
            }
            x if x == SPL_TOKEN_INSTRUCTION_TYPES.MintToChecked => {
                if accounts.len() < 2 {
                    return;
                }
                mint = Some(&accounts[0]);
                destination = Some(&accounts[1]);
                decimals = decoded.get(9).copied();
            }
            x if x == SPL_TOKEN_INSTRUCTION_TYPES.Burn => {
                if accounts.len() < 2 {
                    return;
                }
                source = Some(&accounts[0]);
                mint = Some(&accounts[1]);
            }
            x if x == SPL_TOKEN_INSTRUCTION_TYPES.BurnChecked => {
                if accounts.len() < 2 {
                    return;
                }
                source = Some(&accounts[0]);
                mint = Some(&accounts[1]);
                decimals = decoded.get(9).copied();
            }
            x if x == SPL_TOKEN_INSTRUCTION_TYPES.CloseAccount => {
                if accounts.len() < 2 {
                    return;
                }
                source = Some(&accounts[0]);
                destination = Some(&accounts[1]);
            }
            _ => {}
        }

        self.set_token_info(source, destination, mint, decimals);
    }

    // ===== Вспомогательные =====

    fn convert_to_ui_amount(raw: &str, decimals: u8) -> f64 {
        let val = raw.parse::<f64>().unwrap_or(0.0);
        if decimals == 0 {
            return val;
        }
        let scale = 10f64.powi(decimals as i32);
        val / scale
    }

    // Публичный доступ к картам, если нужно
    pub fn spl_token_map(&self) -> &HashMap<String, TokenInfo> {
        &self.spl_token_map
    }
    pub fn spl_decimals_map(&self) -> &HashMap<String, u8> {
        &self.spl_decimals_map
    }
    pub fn account_keys(&self) -> &[String] {
        &self.account_keys
    }
}
