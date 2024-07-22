use std::collections::HashMap;
use std::str::FromStr;

use cairo_vm::vm::runners::cairo_runner::ExecutionResources;
use starknet_api::transaction::TransactionHash;
use starknet_core::types::MaybePendingTransactionReceipt::Receipt;
use starknet_core::types::TransactionReceipt;
use starknet_crypto::FieldElement;
use starknet_providers::jsonrpc::HttpTransport;
use starknet_providers::{JsonRpcClient, Provider, Url};
use tokio::runtime::Runtime;

pub fn get_execution_resources(tx_hash: TransactionHash) -> ExecutionResources {
    println!("Getting execution resources for {}", tx_hash.to_string());

    let rpc_url = std::env::var("RPC_URL").unwrap_or("http://localhost:6060".to_string());
    let rpc_client = JsonRpcClient::new(HttpTransport::new(Url::from_str(&rpc_url).unwrap()));

    let tx_hash_fe = FieldElement::from(tx_hash.0);
    let rt = Runtime::new().unwrap();
    let tx_receipt =
        rt.block_on(rpc_client.get_transaction_receipt(tx_hash_fe)).unwrap_or_else(|err| {
            panic!("Error occured: {:?}", err);
        });

    let execution_resources = if let Receipt(receipt) = tx_receipt {
        match receipt {
            TransactionReceipt::Invoke(v) => v.execution_resources,
            TransactionReceipt::L1Handler(v) => v.execution_resources,
            TransactionReceipt::Declare(v) => v.execution_resources,
            TransactionReceipt::Deploy(v) => v.execution_resources,
            TransactionReceipt::DeployAccount(v) => v.execution_resources,
        }
    } else {
        panic!("Transaction is pending");
    };

    let mut builtin_instance_counter = HashMap::new();

    builtin_instance_counter.insert(
        "range_check_builtin".to_string(),
        execution_resources.range_check_builtin_applications.unwrap_or_default() as usize,
    );
    builtin_instance_counter.insert(
        "pedersen_builtin".to_string(),
        execution_resources.pedersen_builtin_applications.unwrap_or_default() as usize,
    );
    builtin_instance_counter.insert(
        "poseidon_builtin".to_string(),
        execution_resources.poseidon_builtin_applications.unwrap_or_default() as usize,
    );
    builtin_instance_counter.insert(
        "ec_op_builtin".to_string(),
        execution_resources.ec_op_builtin_applications.unwrap_or_default() as usize,
    );
    builtin_instance_counter.insert(
        "ecdsa_builtin".to_string(),
        execution_resources.ecdsa_builtin_applications.unwrap_or_default() as usize,
    );
    builtin_instance_counter.insert(
        "bitwise_builtin".to_string(),
        execution_resources.bitwise_builtin_applications.unwrap_or_default() as usize,
    );
    builtin_instance_counter.insert(
        "keccak_builtin".to_string(),
        execution_resources.keccak_builtin_applications.unwrap_or_default() as usize,
    );
    builtin_instance_counter.insert(
        "segment_arena_builtin".to_string(),
        execution_resources.segment_arena_builtin.unwrap_or_default() as usize,
    );

    ExecutionResources {
        n_steps: execution_resources.steps as usize,
        n_memory_holes: execution_resources.memory_holes.unwrap_or_default() as usize,
        builtin_instance_counter,
    }
}
