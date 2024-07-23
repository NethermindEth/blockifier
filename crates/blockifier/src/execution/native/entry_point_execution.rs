use std::io::{self, Write};
use std::time::SystemTime;

use cairo_lang_sierra::ids::{ConcreteLibfuncId, ConcreteTypeId, UserTypeId};
use cairo_lang_sierra::program::{
    ConcreteLibfuncLongId, ConcreteTypeLongId, GenericArg, Program as SierraProgram,
};
use cairo_lang_starknet_classes::contract_class::ContractEntryPoints;
use cairo_native::cache::ProgramCache;
use cairo_vm::vm::runners::cairo_runner::ExecutionResources;
use itertools::Itertools;
use starknet_api::core::ClassHash;

use super::syscall_handler::NativeSyscallHandler;
use super::utils::{
    get_native_executor, get_sierra_entry_function_id, match_entrypoint, run_native_executor,
};
use crate::execution::call_info::CallInfo;
use crate::execution::contract_class::SierraContractClassV1;
use crate::execution::entry_point::{
    CallEntryPoint, EntryPointExecutionContext, EntryPointExecutionResult,
};
use crate::execution::errors::EntryPointExecutionError;
use crate::state::state_api::State;

pub fn execute_entry_point_call(
    call: CallEntryPoint,
    contract_class: SierraContractClassV1,
    state: &mut dyn State,
    resources: &mut ExecutionResources,
    context: &mut EntryPointExecutionContext,
    program_cache: &mut ProgramCache<'_, ClassHash>,
) -> EntryPointExecutionResult<CallInfo> {
    println!(
        "Starting execute_entry_point_call in native blockifier for class hash {}",
        call.class_hash.clone().unwrap()
    );
    let call_clone = call.clone();
    let now = SystemTime::now();
    let sierra_program: &SierraProgram = &contract_class.sierra_program;
    let contract_entrypoints: &ContractEntryPoints = &contract_class.entry_points_by_type;

    let matching_entrypoint =
        match_entrypoint(call.entry_point_type, call.entry_point_selector, contract_entrypoints)?;

    match now.elapsed() {
        Ok(elapsed) => {
            println!(
                "Getting/creating cache at {}s",
                (elapsed.as_micros() as f64) / (1000000 as f64)
            );
        }
        Err(e) => println!("Error timing {e}"),
    }
    match now.elapsed() {
        Ok(elapsed) => {
            println!("Got/created cache at {}s", (elapsed.as_micros() as f64) / (1000000 as f64));
        }
        Err(e) => println!("Error timing {e}"),
    }

    let code_class_hash: ClassHash =
        call.class_hash.ok_or(EntryPointExecutionError::NativeExecutionError {
            info: String::from("Class hash was not found"),
        })?;

    let native_executor = get_native_executor(code_class_hash, sierra_program, program_cache);

    match now.elapsed() {
        Ok(elapsed) => {
            println!("Got executor after {}s", (elapsed.as_micros() as f64) / (1000000 as f64));
        }
        Err(e) => println!("Error timing {e}"),
    }

    let syscall_handler: NativeSyscallHandler<'_, '_> = NativeSyscallHandler::new(
        state,
        call.caller_address,
        call.storage_address,
        call.entry_point_selector,
        resources,
        context,
        program_cache,
    );

    match now.elapsed() {
        Ok(elapsed) => {
            println!(
                "Got syscall handler after {}s",
                (elapsed.as_micros() as f64) / (1000000 as f64)
            );
        }
        Err(e) => println!("Error timing {e}"),
    }

    let sierra_entry_function_id =
        get_sierra_entry_function_id(matching_entrypoint, sierra_program);

    match now.elapsed() {
        Ok(elapsed) => {
            println!("Setup finished after {}s", (elapsed.as_micros() as f64) / (1000000 as f64));
        }
        Err(e) => println!("Error timing {e}"),
    }
    println!("Running function {sierra_entry_function_id:?}");
    // if sierra_entry_function_id.id == 24 {
    //     println!("Types:");
    //     print_all_types(&sierra_program);
    //     println!("Program:");
    //     print_sierra_program(&sierra_program);
    //     println!("Program end");
    //     io::stdout().flush().expect("Failed to flush stdout");
    // }
    let result =
        run_native_executor(native_executor, sierra_entry_function_id, call, syscall_handler);
    match now.elapsed() {
        Ok(elapsed) => {
            println!(
                "Native execution finished after {}s for class hash {}",
                (elapsed.as_micros() as f64) / (1000000 as f64),
                call_clone.class_hash.unwrap()
            );
        }
        Err(e) => println!("Error timing {e}"),
    }
    result
}

#[allow(dead_code)]
fn print_sierra_program(program: &SierraProgram) {
    for (idx, statement) in program.statements.iter().enumerate() {
        if let Some(function) = program.funcs.iter().find(|f| f.entry_point.0 == idx) {
            println!("Sierra Function {}", function.id.id);
        }
        print!("{idx}: ");
        match statement {
            cairo_lang_sierra::program::GenStatement::Invocation(inv) => {
                print!(
                    "{}",
                    process_libfunc_long_id(program, get_libfunc_long_id(program, &inv.libfunc_id))
                );
            }
            cairo_lang_sierra::program::GenStatement::Return(_) => {},
        }
        println!("{statement}");
    }
}

fn get_libfunc_long_id<'a>(
    program: &'a SierraProgram,
    id: &'a ConcreteLibfuncId,
) -> &'a ConcreteLibfuncLongId {
    // println!("Getting libfunc long id");
    &program.libfunc_declarations[id.id as usize].long_id
}

fn get_type_long_id<'a>(
    program: &'a SierraProgram,
    id: &'a ConcreteTypeId,
) -> &'a ConcreteTypeLongId {
    // println!("Getting type long id {}/{}", id.id, program.type_declarations.len());
    io::stdout().flush().expect("Failed to flush stdout");
    if id.id as usize >= program.type_declarations.len() {
        // println!("About to panic");
        // io::stdout().flush().expect("Failed to flush stdout");
        panic!("Attempted to get type with id {}, which is out of bounds", id.id);
    }
    let res = &program.type_declarations[id.id as usize].long_id;
    // println!("Got res");
    // io::stdout().flush().expect("Failed to flush stdout");
    res
}

// fn get_userfunc_long_id<'a>(program: &'a SierraProgram, id: &'a FunctionId) -> &'a
// ConcreteTypeLongId {     &program.funcs[id.id as usize].signature
// }

fn get_usertype_type_id<'a>(
    program: &'a SierraProgram,
    id: &'a UserTypeId,
) -> &'a ConcreteTypeLongId {
    // println!("Getting usertype id");
    let type_id = program.type_declarations.iter().find(|t| {
        // println!("About to get type long id");
        // io::stdout().flush().expect("Failed to flush stdout");
        let generic_args = &get_type_long_id(&program, &t.id).generic_args;
        // println!("1");
        // io::stdout().flush().expect("Failed to flush stdout");
        if !generic_args.is_empty() {
            // println!("2");
            // io::stdout().flush().expect("Failed to flush stdout");
            match &generic_args[0] {
                GenericArg::UserType(u) => {
                    // println!("Found user type : {u:?} :{id:?}");
                    // println!("Found user type");
                    // io::stdout().flush().expect("Failed to flush stdout");
                    u.id == id.id
                }
                _ => {
                    // println!("4");
                    // io::stdout().flush().expect("Failed to flush stdout");
                    false
                }
            }
        } else {
            // println!("5");
            // io::stdout().flush().expect("Failed to flush stdout");
            false
        }
    });
    // println!("-");
    // io::stdout().flush().expect("Failed to flush stdout");
    // println!("{}", type_id.is_some());
    // io::stdout().flush().expect("Failed to flush stdout");
    // println!("{:?}", type_id);
    // io::stdout().flush().expect("Failed to flush stdout");
    // if type_id.is_none() {
        // println!("No type found for user type");
    // } else {
        // println!("Type found for user type");
    // }
    // io::stdout().flush().expect("Failed to flush stdout");
    // println!("Creating message");
    // io::stdout().flush().expect("Failed to flush stdout");
    // let expect_msg = format!("Program should have a type declaration for user type {}", id);
    let expect_msg = format!("Program should have a type declaration for a user type");
    // println!("Message created");
    // io::stdout().flush().expect("Failed to flush stdout");
    let type_id = type_id.expect(&expect_msg);
    // println!("Unwrapped successfully");
    // io::stdout().flush().expect("Failed to flush stdout");
    get_type_long_id(&program, &type_id.id)
}

fn process_type_long_id(program: &SierraProgram, id: &ConcreteTypeLongId) -> String {
    // println!("Processing type id");
    if id.generic_args.is_empty() {
        format!("{}", id.generic_id.0)
    } else {
        format!(
            "{}<{}>",
            id.generic_id.0,
            id.generic_args
                .iter()
                .filter(|a| match a {
                    GenericArg::UserType(_) => false,
                    _ => true,
                })
                .map(|arg| process_generic_arg(program, arg))
                .join(", ")
        )
    }
}

fn process_libfunc_long_id(program: &SierraProgram, id: &ConcreteLibfuncLongId) -> String {
    // println!("Processing libfunc id");
    if id.generic_args.is_empty() {
        format!("{}", id.generic_id.0)
    } else {
        format!(
            "{}<{}>",
            id.generic_id.0,
            id.generic_args.iter().map(|arg| process_generic_arg(program, arg)).join(", ")
        )
    }
}

fn process_generic_arg(program: &SierraProgram, arg: &GenericArg) -> String {
    // println!("Processing generic arg");
    match arg {
        GenericArg::UserType(u) => process_type_long_id(program, get_usertype_type_id(program, &u)),
        GenericArg::Type(t) => process_type_long_id(program, get_type_long_id(program, &t)),
        GenericArg::Value(v) => format!("{}", v),
        GenericArg::UserFunc(u) => format!("{:?}", program.funcs[u.id as usize]),
        GenericArg::Libfunc(l) => {
            process_libfunc_long_id(program, get_libfunc_long_id(program, &l))
        }
    }
}

#[allow(dead_code)]
fn print_all_types(program: &SierraProgram) {
    println!("Program has {} types", program.type_declarations.len());
    program.type_declarations.iter().for_each(|t| {
        let id = t.id.id;
        let long_id = get_type_long_id(program, &t.id);
        println!("{:?}", long_id);
        println!("{id}: ");
        println!("{}", process_type_long_id(program, long_id));
        println!("-");
    });
}
