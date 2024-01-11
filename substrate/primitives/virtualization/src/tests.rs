use crate::{ExecError, Memory, MemoryT, SharedState, Virt, VirtT};

pub fn run(program: &[u8]) {
	struct State {
		counter: u64,
		memory: Memory,
	}

	extern "C" fn syscall_handler(
		state: &mut SharedState<State>,
		syscall_no: u32,
		a0: u32,
		_a1: u32,
		_a2: u32,
		_a3: u32,
		_a4: u32,
		_a5: u32,
	) -> u64 {
		match syscall_no {
			// read_counter
			1 => {
				let buf = state.user.counter.to_le_bytes();
				state.user.memory.write(a0, buf.as_ref()).unwrap();
				syscall_no.into()
			},
			// increment counter
			2 => {
				let mut buf = [0u8; 8];
				state.user.memory.read(a0, buf.as_mut()).unwrap();
				state.user.counter += u64::from_le_bytes(buf);
				u64::from(syscall_no) << 56
			},
			// exit
			3 => {
				state.exit = true;
				0
			},
			_ => panic!("unknown syscall: {:?}", syscall_no),
		}
	}

	// start counter at 0 and use a host trap to exit
	let mut instance = Virt::instantiate(program).unwrap();
	let mut state = SharedState {
		gas_left: 0,
		exit: false,
		user: State { counter: 0, memory: instance.memory() },
	};
	let ret = instance.execute("main_0", syscall_handler, &mut state);
	assert_eq!(ret, Err(ExecError::Trap));
	assert!(state.exit);
	assert_eq!(state.user.counter, 8);

	// start counter at 21 and use a host trap to exit
	let instance = Virt::instantiate(program).unwrap();
	let mut state = SharedState {
		gas_left: 0,
		exit: false,
		user: State { counter: 0, memory: instance.memory() },
	};
	let ret = instance.execute_and_destroy("main_21", syscall_handler, &mut state);
	assert_eq!(ret, Err(ExecError::Trap));
	assert!(state.exit);
	assert_eq!(state.user.counter, 29);

	// start counter at 7 but instruct to return naturally
	let mut instance = Virt::instantiate(program).unwrap();
	let mut state = SharedState {
		gas_left: 0,
		exit: false,
		user: State { counter: 0, memory: instance.memory() },
	};
	let ret = instance.execute("main_7_no_exit", syscall_handler, &mut state);
	assert_eq!(ret, Ok(()));
	assert!(!state.exit);
	assert_eq!(state.user.counter, 15);

	// instruct program to panic
	let instance = Virt::instantiate(program).unwrap();
	let mut state = SharedState {
		gas_left: 0,
		exit: false,
		user: State { counter: 0, memory: instance.memory() },
	};
	let ret = instance.execute_and_destroy("panic_me", syscall_handler, &mut state);
	assert_eq!(ret, Err(ExecError::Trap));
	assert!(!state.exit);
	assert_eq!(state.user.counter, 0);
}

#[cfg(test)]
#[test]
fn tests() {
	sp_tracing::try_init_simple();
	run(sp_virtualization_test_fixture::binary());
}
