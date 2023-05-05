use libc::CLOCK_BOOTTIME;

static VEC_SIZE: usize = 1000;
const KMGS_PATH: *const libc::c_char = "/dev/kmsg\0".as_ptr() as *const libc::c_char;

fn main() {
	let mut nanosec_times = Vec::<libc::c_long>::with_capacity(VEC_SIZE);
	let mut sec_times = Vec::<libc::time_t>::with_capacity(VEC_SIZE);
	let mut nanosec_pointer = nanosec_times.as_mut_ptr(); 
	let mut sec_pointer = sec_times.as_mut_ptr();
	unsafe {
		let kmsg_file: libc::c_int = libc::open(KMGS_PATH, libc::O_WRONLY);
		let mut counter = 0;
		let mut t = std::mem::MaybeUninit::zeroed();
		let t = t.assume_init_mut();
		while counter != VEC_SIZE { // Each write takes about 1200 ns on an i5-4460. This is slow enough to result in subsequent writes being on different timestamps in dmesg
									// Difference between debug and release versions is tiny - about 100 ns
									// Each write takes more than 10 microseconds on a Raspberry Pi 3B+. That's slow enough to require scheduling the write early
									// Difference between debug and release is far larger there - about 2 microseconds
			libc::clock_gettime(CLOCK_BOOTTIME, t);
			let message = format!("Write started at {} s + {} ns", t.tv_sec, t.tv_nsec);
			libc::write(kmsg_file, message.as_ptr() as *const libc::c_void, message.len());
			*nanosec_pointer = t.tv_nsec;
			*sec_pointer = t.tv_sec;
			nanosec_pointer = nanosec_pointer.add(1);
			sec_pointer = sec_pointer.add(1);
			counter += 1;
		}
		libc::close(kmsg_file);	
		nanosec_times.set_len(VEC_SIZE);
		sec_times.set_len(VEC_SIZE);
	}
	let mut full_times = Vec::<i128>::with_capacity(VEC_SIZE);
	for pair in nanosec_times.iter().zip(sec_times) {
		full_times.push((pair.1 as i128) << 64 | *pair.0 as i128);
	}
	let mut diffs = Vec::<i128>::with_capacity(VEC_SIZE - 1);
	let mut counter = 0;
	for index in 0..VEC_SIZE - 1 {
		diffs.push(full_times[index + 1] - full_times[index]);
		counter += full_times[index + 1] - full_times[index];
	}
	let avg = counter / (VEC_SIZE as i128 - 1);
	let mut over_avg_count = 0;
	for diff in &diffs {
		if diff > &avg {
			over_avg_count += 1;
		}
	}
	println!("{:#?}", diffs);
	println!("Average time per loop: {} ns, times exceeded: {}", avg, over_avg_count);
}
