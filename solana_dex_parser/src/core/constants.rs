pub mod dex_programs {
    pub const JUPITER: &str = "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4";
    pub const RAYDIUM: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
    pub const PUMP_FUN: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";
    pub const PUMP_SWAP: &str = "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA";
    pub const ORCA: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";
    pub const METEORA: &str = "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo";
    pub const UNKNOWN: &str = "UNKNOWN";
}

pub mod dex_program_names {
    use super::dex_programs;
    use once_cell::sync::Lazy;
    use std::collections::HashMap;

    static PROGRAM_NAME: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
        let mut map = HashMap::new();
        map.insert(dex_programs::JUPITER, "Jupiter");
        map.insert(dex_programs::RAYDIUM, "Raydium");
        map.insert(dex_programs::PUMP_FUN, "Pumpfun");
        map.insert(dex_programs::PUMP_SWAP, "Pumpswap");
        map.insert(dex_programs::ORCA, "Orca");
        map.insert(dex_programs::METEORA, "Meteora");
        map
    });

    pub fn name(program_id: &str) -> &'static str {
        PROGRAM_NAME
            .get(program_id)
            .copied()
            .unwrap_or("Unknown DEX")
    }
}
