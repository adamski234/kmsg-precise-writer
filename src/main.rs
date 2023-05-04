use libc::{timespec, CLOCK_BOOTTIME};

static VEC_SIZE: usize = 1000;

fn main() {
	let mut a = Vec::<u64>::with_capacity(VEC_SIZE);
	let mut accessor = a.as_mut_ptr(); 
	unsafe {
		let mut counter = 0;
		let mut t = std::mem::MaybeUninit::zeroed();
		let t = t.assume_init_mut();
		while counter != VEC_SIZE {
			libc::clock_gettime(CLOCK_BOOTTIME, t); // On release this takes ~17 nanoseconds between loops
			*accessor = t.tv_nsec as u64;
			accessor = accessor.add(1);
			counter += 1;
		}
		a.set_len(VEC_SIZE);
	}
	for b in a {
		println!("{}", b);
	}
}
