#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
	unsafe {
		core::arch::asm!("unimp", options(noreturn));
	}
}

mod sys {
	#[polkavm_derive::polkavm_import]
	extern "C" {
		#[polkavm_import(symbol = 1u32)]
		pub fn read_counter(buf_ptr: *mut u8) -> u32;
		#[polkavm_import(symbol = 2u32)]
		pub fn increment_counter(buf_ptr: *const u8) -> u64;
		#[polkavm_import(symbol = 3u32)]
		pub fn exit();
	}
}

fn read_counter() -> u64 {
	let mut buffer = [42u8; 8];
	let ret = unsafe { sys::read_counter(buffer.as_mut_ptr()) };
	assert_eq!(ret, 1);
	u64::from_le_bytes(buffer)
}

fn increment_counter(inc: u64) {
	let ret = unsafe { sys::increment_counter(inc.to_le_bytes().as_ptr()) };
	assert_eq!(ret, 2 << 56);
}

fn exit() -> ! {
	unsafe {
		sys::exit();
		core::hint::unreachable_unchecked();
	}
}

fn run(initial_counter: u64, explicit_exit: bool) {
	increment_counter(initial_counter);
	assert_eq!(read_counter(), initial_counter);
	increment_counter(7);
	assert_eq!(read_counter(), initial_counter + 7);
	increment_counter(1);
	assert_eq!(read_counter(), initial_counter + 8);
	if explicit_exit {
		exit()
	}
}

#[polkavm_derive::polkavm_export]
extern "C" fn main_0() {
	run(0, true)
}

#[polkavm_derive::polkavm_export]
extern "C" fn main_21() {
	run(21, true)
}

#[polkavm_derive::polkavm_export]
extern "C" fn main_7_no_exit() {
	run(7, false)
}

#[polkavm_derive::polkavm_export]
extern "C" fn panic_me() {
	panic!("panic_me was called")
}
