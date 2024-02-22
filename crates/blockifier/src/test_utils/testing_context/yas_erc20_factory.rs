use starknet_api::class_hash;
use starknet_api::core::ClassHash;
use starknet_api::hash::StarkHash;
use starknet_types_core::felt::Felt;

use crate::execution::contract_class::SierraContractClassV1;
use crate::state::cached_state::CachedState;
use crate::test_utils::dict_state_reader::DictStateReader;
use crate::test_utils::testing_context::{create_custom_deploy_test_state, StateFactory};
use crate::test_utils::{ERC20_FULL_CONTRACT_PATH, TEST_ERC20_FULL_CONTRACT_CLASS_HASH};

pub struct YASERC20Factory {
    args: Vec<Felt>,
    #[allow(dead_code)]
    state: CachedState<DictStateReader>,
}

impl YASERC20Factory {
    pub fn new(args: Vec<Felt>) -> Self {
        YASERC20Factory { args, state: create_custom_deploy_test_state(vec![], vec![]) }
    }
}

impl StateFactory for YASERC20Factory {
    fn get_state(&self) -> CachedState<DictStateReader> {
        create_custom_deploy_test_state(
            vec![],
            vec![(
                class_hash!(TEST_ERC20_FULL_CONTRACT_CLASS_HASH),
                SierraContractClassV1::from_file(ERC20_FULL_CONTRACT_PATH).into(),
            )],
        )
    }

    fn args(&self) -> Vec<Felt> {
        self.args.clone()
    }

    fn class_hash(&self) -> &'static str {
        todo!()
    }
}
