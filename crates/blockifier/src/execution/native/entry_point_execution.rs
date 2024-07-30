use cairo_lang_starknet_classes::contract_class::ContractEntryPoints;
use cairo_vm::vm::runners::cairo_runner::ExecutionResources;

use super::syscall_handler::NativeSyscallHandler;
use super::utils::{match_entrypoint, run_native_executor};
use crate::execution::call_info::CallInfo;
use crate::execution::contract_class::NativeContractClassV1;
use crate::execution::entry_point::{
    CallEntryPoint, EntryPointExecutionContext, EntryPointExecutionResult,
};
use crate::state::state_api::State;

pub fn execute_entry_point_call(
    call: CallEntryPoint,
    contract_class: NativeContractClassV1,
    state: &mut dyn State,
    resources: &mut ExecutionResources,
    context: &mut EntryPointExecutionContext,
) -> EntryPointExecutionResult<CallInfo> {
    let contract_entrypoints: &ContractEntryPoints = &contract_class.entry_points_by_type;

    // Looks like this could be a lookup with just a hashmap
    // The entry point selector
    // The goal is the find the function id into the contract.
    // Staying too close to the way it has been created?
    // Could also make use of it to have three dictionaries
    let matching_entrypoint =
        match_entrypoint(call.entry_point_type, call.entry_point_selector, contract_entrypoints)?;

    let syscall_handler: NativeSyscallHandler<'_> = NativeSyscallHandler::new(
        state,
        call.caller_address,
        call.storage_address,
        call.entry_point_selector,
        resources,
        context,
    );

    println!("Blockifier-Native: running the Native Executor");
    let result =
        run_native_executor(&contract_class.executor, matching_entrypoint, call, syscall_handler);
    println!("Blockifier-Native: Native Executor finished running");
    result
}
