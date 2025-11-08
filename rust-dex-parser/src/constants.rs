pub mod dex_programs {
    pub const JUPITER: &str = "JUPITER";
    pub const RAYDIUM: &str = "RAYDIUM";
    pub const PUMPFUN: &str = "PUMPFUN";
    pub const ORCA: &str = "ORCA";
    pub const METEORA: &str = "METEORA";
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
        map.insert(dex_programs::PUMPFUN, "Pumpfun");
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
