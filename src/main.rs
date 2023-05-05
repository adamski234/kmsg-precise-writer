use std::fs::File;

use libc::CLOCK_BOOTTIME;

static TEST_RUN_BATCH_SIZE: usize = 1000;
const KMGS_PATH: *const libc::c_char = "/dev/kmsg\0".as_ptr() as *const libc::c_char;
//static TIME_TO_TRIGGER_AT: u128 = 10_000_000 * 1_000_000; // At which point the message should be written, in microseconds. Currently set to exactly 10 million seconds
static TIME_TO_TRIGGER_AT: u128 = 26500_000000;

fn main() {
	/* =====================================
	 * declare and initialize most variables
	 * =====================================
	 */
	let dmesg_logged_deltas: Vec<u128>;
	let average_delta: u128;
	let mut over_avg_count: u128 = 0;
	let kmsg_file: libc::c_int;

	
	unsafe {
		/* ==============================
		 * initialize all unsafe elements
		 * ==============================
		 */
		kmsg_file = libc::open(KMGS_PATH, libc::O_RDWR);
		let mut counter = 0;
		let mut t = std::mem::MaybeUninit::zeroed();
		let t = t.assume_init_mut();

		/* ================================================================
		 * find time offset between time measured and time in dmesg message
		 * using an average of multiple writes
		 * ================================================================
		 */
		while counter < TEST_RUN_BATCH_SIZE + 1 { 	// Each write takes about 1200 ns on an i5-4460. This is slow enough to result in subsequent writes being on different timestamps in dmesg
												// Difference between debug and release versions is tiny - about 100 ns
												// Each write takes more than 10 microseconds on a Raspberry Pi 3B+. That's slow enough to require scheduling the write early
												// Difference between debug and release is far larger there - about 2 microseconds
			libc::clock_gettime(CLOCK_BOOTTIME, t);
			let message = format!("{} {}", t.tv_sec, t.tv_nsec);
			libc::write(kmsg_file, message.as_ptr() as *const libc::c_void, message.len());
			counter += 1;
		}
	}


	/* ==========================================
	 * read entries from /dev/kmsg to parse times
	 * calculate average delay
	 * dmesg time - logged time
	 * ==========================================
	 */
	let mut dmesg_buffer = String::new();
	nonblock::NonBlockingReader::from_fd(File::open("/dev/kmsg").unwrap()).unwrap().read_available_to_string(&mut dmesg_buffer).unwrap();
	dmesg_logged_deltas = dmesg_buffer.split('\n').rev().skip(1).take(TEST_RUN_BATCH_SIZE).map(|input| {
		// for each line
		let parts: Vec<&str> = input.split(';').collect();
		let dmesg_timestamp_microsecs: u128 = parts[0].split(',').collect::<Vec<&str>>()[2].parse().unwrap();
		let dmesg_timestamp_nsecs = dmesg_timestamp_microsecs * 1000; // Convert to nanoseconds from microseconds
		let written_stamp_parts: Vec<&str> = parts[1].split(' ').collect(); // expects it in the form of `sec nsec`, space necessary
		let mut written_stamp_buffer = String::from(written_stamp_parts[0]);
		written_stamp_buffer.push_str(written_stamp_parts[1]); // Concatenates nanoseconds to the end of seconds
		let written_stamp_nsecs: u128 = written_stamp_buffer.parse().unwrap();
		return dmesg_timestamp_nsecs - written_stamp_nsecs; // Dmesg entry is always later than logged timestamp
	}).collect();

	average_delta = dmesg_logged_deltas.iter().sum::<u128>() / TEST_RUN_BATCH_SIZE as u128;

	for delta in &dmesg_logged_deltas {
		if *delta > average_delta {
			over_avg_count += 1;
		}
	}

	println!("Average time delta: {} ns, times exceeded: {}", average_delta, over_avg_count);
	eprintln!("{:#?}", dmesg_logged_deltas);

	/* ==============================================================================
	 * calculate time returned by CLOCK_BOOTTIME that corresponds to the desired time
	 * ==============================================================================
	 */
	let clock_time_to_trigger = TIME_TO_TRIGGER_AT * 1000 - average_delta; // multiply by 1000 to get nanoseconds from microseconds
	
	/* =====================================================================
	 * busy wait loop, constantly polling clock to check if time is in range
	 * =====================================================================
	 */
	unsafe {
		let message = format!("Message should be at exactly {}", TIME_TO_TRIGGER_AT);
		let mut time = std::mem::MaybeUninit::zeroed();
		let time = time.assume_init_mut();
		libc::clock_gettime(CLOCK_BOOTTIME, time);
		let mut time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128; // Convert from separate times to one unit
		while time_nanosecs < clock_time_to_trigger {
			libc::clock_gettime(CLOCK_BOOTTIME, time);
			time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128; // Convert from separate times to one unit
		}
		libc::write(kmsg_file, message.as_ptr() as *const libc::c_void, message.len());
		libc::clock_gettime(CLOCK_BOOTTIME, time);
		println!("Finished at {} s {} ns", time.tv_sec, time.tv_nsec);
	}

	unsafe {
		/* ===============
		 * close resources
		 * ===============
		 */
		libc::close(kmsg_file);
	}
}
