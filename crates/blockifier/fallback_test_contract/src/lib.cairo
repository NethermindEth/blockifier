#[starknet::contract]
mod SimpleContract {
    use starknet::syscalls;

    #[storage]
    struct Storage {
        value: u32
    }


    #[external(v0)]
    fn set_value(ref self: ContractState, value: u32) {
        assert!(self.value.read() == 0, "value must be 0");

        self.value.write(value);
    }

    #[external(v0)]
    fn get_value(ref self: ContractState) -> u32 {
        self.value.read()
    }
}