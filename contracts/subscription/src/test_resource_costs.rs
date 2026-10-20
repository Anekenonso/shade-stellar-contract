extern crate std;

use super::*;
use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::token::StellarAssetClient;
use soroban_sdk::{vec, Address, Env, String, Vec};

const MONTHLY: u64 = 2_592_000;
const AMOUNT: i128 = 1_000;

// Per-transaction network limits used to decide whether a batch is safe.
// Source: Stellar network settings (Protocol 23). Update with the network.
const LIMIT_INSTRUCTIONS: i64 = 100_000_000;
const LIMIT_MEM_BYTES: i64 = 41_943_040;
const LIMIT_READ_ENTRIES: u32 = 100;
const LIMIT_WRITE_ENTRIES: u32 = 50;
const LIMIT_WRITE_BYTES: u32 = 132_096;
const LIMIT_EVENT_BYTES: u32 = 16_384;

struct Row {
    size: u32,
    instructions: i64,
    mem_bytes: i64,
    read_entries: u32,
    write_entries: u32,
    write_bytes: u32,
    event_bytes: u32,
}

impl Row {
    fn within_limits(&self) -> bool {
        self.instructions <= LIMIT_INSTRUCTIONS
            && self.mem_bytes <= LIMIT_MEM_BYTES
            && self.read_entries <= LIMIT_READ_ENTRIES
            && self.write_entries <= LIMIT_WRITE_ENTRIES
            && self.write_bytes <= LIMIT_WRITE_BYTES
            && self.event_bytes <= LIMIT_EVENT_BYTES
    }
}

fn measure_process_billing_cycle(size: u32) -> Row {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|l| l.timestamp = 1_000_000);

    let contract = env.register(SubscriptionContract, ());
    let client = SubscriptionContractClient::new(&env, &contract);
    client.initialize(&Address::generate(&env));

    let token = env
        .register_stellar_asset_contract_v2(Address::generate(&env))
        .address();
    client.add_accepted_token(&token);

    let merchant = Address::generate(&env);
    let plan_id = client.create_plan(
        &merchant,
        &String::from_str(&env, "Plan"),
        &token,
        &AMOUNT,
        &MONTHLY,
        &None,
        &0,
    );

    let mut ids: Vec<u64> = vec![&env];
    for _ in 0..size {
        let customer = Address::generate(&env);
        let sub_id = client.subscribe(&customer, &plan_id);
        StellarAssetClient::new(&env, &token).mint(&customer, &AMOUNT);
        let expiry = env.ledger().sequence() + 1_000_000;
        soroban_sdk::token::TokenClient::new(&env, &token)
            .approve(&customer, &contract, &AMOUNT, &expiry);
        ids.push_back(sub_id);
    }

    env.cost_estimate().budget().reset_unlimited();
    client.process_billing_cycle(&ids);
    let r = env.cost_estimate().resources();

    Row {
        size,
        instructions: r.instructions,
        mem_bytes: r.mem_bytes,
        read_entries: r.disk_read_entries + r.memory_read_entries,
        write_entries: r.write_entries,
        write_bytes: r.write_bytes,
        event_bytes: r.contract_events_size_bytes,
    }
}

/// Regenerate the `process_billing_cycle` rows of docs/reference/resource-costs.md:
///
///     cargo test -p subscription resource_costs -- --nocapture
#[test]
fn resource_costs_process_billing_cycle() {
    let sizes = [1u32, 5, 10, 16, 17, 20, 40, 80];
    let mut max_safe = 0;

    std::println!("| batch size | instructions | memory bytes | read entries | write entries | write bytes | event bytes | within limits |");
    std::println!("|---|---|---|---|---|---|---|---|");
    for size in sizes {
        let row = measure_process_billing_cycle(size);
        let ok = row.within_limits();
        if ok {
            max_safe = size;
        }
        std::println!(
            "| {} | {} | {} | {} | {} | {} | {} | {} |",
            row.size,
            row.instructions,
            row.mem_bytes,
            row.read_entries,
            row.write_entries,
            row.write_bytes,
            row.event_bytes,
            if ok { "yes" } else { "no" }
        );
    }
    std::println!("largest measured batch within limits: {}", max_safe);

    assert!(max_safe >= 1, "a single charge must fit in one transaction");
}
