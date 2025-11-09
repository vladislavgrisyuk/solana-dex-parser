# Solana DEX Parser (Rust)

This repository now ships a Rust implementation of the [`solana-dex-parser`](https://github.com/vladislavgrisyuk/solana-dex-parser) API.  
The crate exposes the same public surface as the original TypeScript package while providing a native binary for
batch processing transactions and blocks.

> **Status:** experimental port focusing on parity with the existing TypeScript structures. The TypeScript source is
> still available under `src/` for reference.

## Features

- ✅ `DexParser` API with `parse_all`, `parse_trades`, `parse_liquidity`, and `parse_transfers` helpers.
- ✅ Block helpers for raw (`getBlock`) and parsed (`getParsedBlock`) payloads.
- ✅ JSON output compatible with the TypeScript library (field names preserved via `serde` attributes).
- ✅ Optional CLI (`dexp`) for quick inspection of saved RPC payloads.

## Installation

Add the crate to an existing Cargo project:

```bash
cargo add solana-dex-parser
```

To build the command line utility enable the `cli` feature:

```bash
cargo install solana-dex-parser --features cli
```

## Usage

```rust
use solana_dex_parser::{DexParser, ParseConfig, SolanaTransaction};

fn main() -> anyhow::Result<()> {
    // Transaction JSON that matches the TypeScript structure
    let tx_json = std::fs::read_to_string("transaction.json")?;
    let tx: SolanaTransaction = serde_json::from_str(&tx_json)?;

    let parser = DexParser::new();
    let trades = parser.parse_trades(tx.clone(), Some(ParseConfig::default()));
    println!("{}", serde_json::to_string_pretty(&trades)?);

    Ok(())
}
```

### CLI

The CLI is provided behind the `cli` feature as `dexp`:

```bash
# Parse a single transaction dump
cargo run --features cli --bin dexp -- parse-tx --file fixtures/tx.json --mode all

# Parse a block dump
cargo run --features cli --bin dexp -- parse-block --file fixtures/block.json --mode parsed
```

Available modes:

- `parse-tx`: `all`, `trades`, `liquidity`, `transfers`
- `parse-block`: `raw` (array of transactions) or `parsed` (block object)

### Configuration

`ParseConfig` mirrors the TypeScript options. All fields are optional and default to the
same values used in the original package.

| Field | JSON key | Description | Default |
|-------|----------|-------------|---------|
| `try_unknown_dex` | `tryUnknowDEX` | Attempt heuristic parsing for unknown programs | `true` |
| `program_ids` | `programIds` | Only process the listed program IDs | `None` |
| `ignore_program_ids` | `ignoreProgramIds` | Skip the listed program IDs | `None` |
| `throw_error` | `throwError` | Propagate parser errors | `false` |
| `aggregate_trades` | `aggregateTrades` | Include the aggregated trade summary | `true` |

## Testing

Integration fixtures live under `solana_dex_parser/tests`. Run the suite with:

```bash
cargo test
```

## Protocol coverage

The reference implementation wires simple parser adapters for the core DEX families shipped in the repository:
Jupiter-style swaps, Raydium-style pools, Pumpfun/Pumpswap flows, Orca-like pools, and Meteora liquidity events.
Additional protocol specific logic can be layered on top of the `protocols` module.

## License

Released under the MIT license. See [LICENSE](LICENSE) for details.
