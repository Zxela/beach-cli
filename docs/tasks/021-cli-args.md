---
id: "021"
title: "Add --plan CLI argument with clap"
status: pending
depends_on: ["017", "020"]
test_file: tests/cli_args.rs
---

# 021: Add --plan CLI argument with clap

## Objective

Add command-line argument parsing using clap to allow launching directly into Plan Trip mode, optionally with a pre-selected activity.

## Acceptance Criteria

- [ ] Add `clap` dependency to Cargo.toml
- [ ] `vanbeach --plan` opens directly in PlanTrip state
- [ ] `vanbeach --plan swim` opens PlanTrip with Swimming selected
- [ ] `vanbeach --plan sunset` opens PlanTrip with Sunset selected
- [ ] Invalid activity name prints error and exits
- [ ] Running without flags works as before (starts in Loading → BeachList)
- [ ] `vanbeach --help` shows usage information

## Technical Notes

From TECHNICAL_DESIGN.md:
```rust
use clap::Parser;

#[derive(Parser)]
#[command(name = "vanbeach")]
#[command(about = "Vancouver beach conditions and trip planning")]
struct Cli {
    /// Open directly in plan mode
    #[arg(long)]
    plan: Option<Option<String>>,  // --plan or --plan swim
}
```

Add to Cargo.toml:
```toml
clap = { version = "4", features = ["derive"] }
```

Activity parsing should use `Activity::from_str()` from task 012.

## Files to Create/Modify

- `Cargo.toml` - Add clap dependency
- `src/main.rs` - Add CLI struct and argument handling
- `tests/cli_args.rs` (new) - Integration tests for CLI

## Test Requirements

Create `tests/cli_args.rs`:
- Test `--plan` flag sets initial state to PlanTrip
- Test `--plan swim` sets state to PlanTrip and activity to Swimming
- Test `--plan invalid` produces error
- Test no flags starts in Loading state
- Test `--help` exits with success
