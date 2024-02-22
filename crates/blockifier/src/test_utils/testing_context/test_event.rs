use starknet_types_core::felt::Felt;

use crate::execution::call_info::OrderedEvent;
use crate::execution::sierra_utils::starkfelt_to_felt;

#[derive(Debug, Clone)]
pub struct TestEvent {
    pub data: Vec<Felt>,
    pub keys: Vec<Felt>,
}

impl From<OrderedEvent> for TestEvent {
    fn from(value: OrderedEvent) -> Self {
        let event_data = value.event.data.0.iter().map(|e| starkfelt_to_felt(*e)).collect();
        let event_keys = value.event.keys.iter().map(|e| starkfelt_to_felt(e.0)).collect();
        Self { data: event_data, keys: event_keys }
    }
}
