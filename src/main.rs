use std::{fs::File, env::args};

use libc::CLOCK_BOOTTIME;

static TEST_RUN_BATCH_SIZE: usize = 1000;
static WARMUP_RUN_SIZE: usize = 20;
static WARMUP_ROUND_LENGTH: usize = 1_2000; // time per loop in nanoseconds
const KMSG_PATH: *const libc::c_char = "/dev/kmsg\0".as_ptr() as *const libc::c_char;
const WARMUP_MSG: *const libc::c_void = "WARMUP\n".as_ptr() as *const libc::c_void;
const WARMUP_MSG_LEN: usize = 7;
const MSG_BEFORE_10: *const libc::c_void = "10 SECONDS UNTIL 10 MILLION\n".as_ptr() as *const libc::c_void;
const BEFORE_10_LEN: usize = 28;
const MSG_BEFORE_9: *const libc::c_void = "9 SECONDS UNTIL 10 MILLION\n".as_ptr() as *const libc::c_void;
const COUNTER_MSG_LEN: usize = 27;
const MSG_BEFORE_8: *const libc::c_void = "8 SECONDS UNTIL 10 MILLION\n".as_ptr() as *const libc::c_void;
const MSG_BEFORE_7: *const libc::c_void = "7 SECONDS UNTIL 10 MILLION\n".as_ptr() as *const libc::c_void;
const MSG_BEFORE_6: *const libc::c_void = "6 SECONDS UNTIL 10 MILLION\n".as_ptr() as *const libc::c_void;
const MSG_BEFORE_5: *const libc::c_void = "5 SECONDS UNTIL 10 MILLION\n".as_ptr() as *const libc::c_void;
const MSG_BEFORE_4: *const libc::c_void = "4 SECONDS UNTIL 10 MILLION\n".as_ptr() as *const libc::c_void;
const MSG_BEFORE_3: *const libc::c_void = "3 SECONDS UNTIL 10 MILLION\n".as_ptr() as *const libc::c_void;
const MSG_BEFORE_2: *const libc::c_void = "2 SECONDS UNTIL 10 MILLION\n".as_ptr() as *const libc::c_void;
const MSG_BEFORE_1: *const libc::c_void = "1 SECOND UNTIL 10 MILLION\n".as_ptr() as *const libc::c_void;
const BEFORE_1_LEN: usize = 26;
const TEN_MIL_MSG: *const libc::c_void = "🎉 🎉 🎉 I HAVE REACHED EXACTLY 10 MILLION SECONDS OF UPTIME! 🎉 🎉 🎉\n".as_ptr() as *const libc::c_void;
const TEN_MIL_MSG_LEN: usize = 83;
const ONE_SECOND_IN_NSEC: u128 = 1000 * 1000 * 1000;

fn main() {
	/* =====================================
	 * declare and initialize most variables
	 * =====================================
	 */
	let dmesg_logged_deltas: Vec<u128>;
	let average_delta: u128;
	let mut over_avg_count: u128 = 0;
	let mut kmsg_file: libc::c_int;

	let time_to_print = args().collect::<Vec<_>>()[1].parse::<u128>().unwrap() * 1_000_000_000; // time passed on the command line in seconds

	unsafe {
		/* ==============================
		 * initialize all unsafe elements
		 * ==============================
		 */
		kmsg_file = libc::open(KMSG_PATH, libc::O_WRONLY);
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
		libc::close(kmsg_file);
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
	 * complete with evil code repetition
	 * ==============================================================================
	 */
	let clock_time_to_trigger = time_to_print - average_delta - 160 * 1000;
	let clock_time_to_trigger_warmup = clock_time_to_trigger - (WARMUP_ROUND_LENGTH * WARMUP_RUN_SIZE) as u128;

	let clock_time_to_trigger_ctd_1 = clock_time_to_trigger - 1 * (ONE_SECOND_IN_NSEC + 20 * 1000);
	let clock_time_to_trigger_ctd_1_warmup = clock_time_to_trigger_ctd_1 - (WARMUP_ROUND_LENGTH * WARMUP_RUN_SIZE) as u128;

	let clock_time_to_trigger_ctd_2 = clock_time_to_trigger - 2 * (ONE_SECOND_IN_NSEC + 20 * 1000);
	let clock_time_to_trigger_ctd_2_warmup = clock_time_to_trigger_ctd_2 - (WARMUP_ROUND_LENGTH * WARMUP_RUN_SIZE) as u128;

	let clock_time_to_trigger_ctd_3 = clock_time_to_trigger - 3 * (ONE_SECOND_IN_NSEC + 20 * 1000);
	let clock_time_to_trigger_ctd_3_warmup = clock_time_to_trigger_ctd_3 - (WARMUP_ROUND_LENGTH * WARMUP_RUN_SIZE) as u128;

	let clock_time_to_trigger_ctd_4 = clock_time_to_trigger - 4 * (ONE_SECOND_IN_NSEC + 20 * 1000);
	let clock_time_to_trigger_ctd_4_warmup = clock_time_to_trigger_ctd_4 - (WARMUP_ROUND_LENGTH * WARMUP_RUN_SIZE) as u128;

	let clock_time_to_trigger_ctd_5 = clock_time_to_trigger - 5 * (ONE_SECOND_IN_NSEC + 20 * 1000);
	let clock_time_to_trigger_ctd_5_warmup = clock_time_to_trigger_ctd_5 - (WARMUP_ROUND_LENGTH * WARMUP_RUN_SIZE) as u128;
	
	let clock_time_to_trigger_ctd_6 = clock_time_to_trigger - 6 * (ONE_SECOND_IN_NSEC + 20 * 1000);
	let clock_time_to_trigger_ctd_6_warmup = clock_time_to_trigger_ctd_6 - (WARMUP_ROUND_LENGTH * WARMUP_RUN_SIZE) as u128;

	let clock_time_to_trigger_ctd_7 = clock_time_to_trigger - 7 * (ONE_SECOND_IN_NSEC + 20 * 1000);
	let clock_time_to_trigger_ctd_7_warmup = clock_time_to_trigger_ctd_7 - (WARMUP_ROUND_LENGTH * WARMUP_RUN_SIZE) as u128;

	let clock_time_to_trigger_ctd_8 = clock_time_to_trigger - 8 * (ONE_SECOND_IN_NSEC + 20 * 1000);
	let clock_time_to_trigger_ctd_8_warmup = clock_time_to_trigger_ctd_8 - (WARMUP_ROUND_LENGTH * WARMUP_RUN_SIZE) as u128;

	let clock_time_to_trigger_ctd_9 = clock_time_to_trigger - 9 * (ONE_SECOND_IN_NSEC + 20 * 1000);
	let clock_time_to_trigger_ctd_9_warmup = clock_time_to_trigger_ctd_9 - (WARMUP_ROUND_LENGTH * WARMUP_RUN_SIZE) as u128;

	let clock_time_to_trigger_ctd_10 = clock_time_to_trigger - 10 * (ONE_SECOND_IN_NSEC + 20 * 1000);
	let clock_time_to_trigger_ctd_10_warmup = clock_time_to_trigger_ctd_10 - (WARMUP_ROUND_LENGTH * WARMUP_RUN_SIZE) as u128;
	
	/* =====================================================================
	 * busy wait loop, constantly polling clock to check if time is in range
	 * =====================================================================
	 */
	unsafe {
		kmsg_file = libc::open(KMSG_PATH, libc::O_WRONLY);
		let mut time = std::mem::MaybeUninit::zeroed();
		let time = time.assume_init_mut();

		libc::clock_gettime(CLOCK_BOOTTIME, time);
		
		/* ================= COUNTDOWN 10 ================== */
		{
			let mut time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128; // Convert from separate times to one unit
			while time_nanosecs < clock_time_to_trigger_ctd_10_warmup {
				// each loop should take ~20 ns
				libc::clock_gettime(CLOCK_BOOTTIME, time);
				time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128; // Convert from separate times to one unit
			}
			/* ====================================================
				* do warmup prints to make performance more consistent
				* ====================================================
				*/
			let mut counter = 0;
			while counter < WARMUP_RUN_SIZE + 1 {
				libc::write(kmsg_file, WARMUP_MSG, WARMUP_MSG_LEN);
				counter += 1;
				libc::clock_gettime(CLOCK_BOOTTIME, time);
				time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128;
				if time_nanosecs >= clock_time_to_trigger_ctd_10 {
					break;
				}
			}

			while time_nanosecs < clock_time_to_trigger_ctd_10 { // make sure we're as close as possible
				// each loop should take ~20 ns
				libc::clock_gettime(CLOCK_BOOTTIME, time);
				time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128; // Convert from separate times to one unit
			}

			/* ==================================================
				* it's printing time
				* and then he printed all over the bad guys (jitter)
				* ==================================================
				*/
			libc::write(kmsg_file, MSG_BEFORE_10, BEFORE_10_LEN);
		}

		/* ================= COUNTDOWN 9 =================== */
		{
			let mut time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128; // Convert from separate times to one unit
			while time_nanosecs < clock_time_to_trigger_ctd_9_warmup {
				// each loop should take ~20 ns
				libc::clock_gettime(CLOCK_BOOTTIME, time);
				time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128; // Convert from separate times to one unit
			}
			/* ====================================================
				* do warmup prints to make performance more consistent
				* ====================================================
				*/
			let mut counter = 0;
			while counter < WARMUP_RUN_SIZE + 1 {
				libc::write(kmsg_file, WARMUP_MSG, WARMUP_MSG_LEN);
				counter += 1;
				libc::clock_gettime(CLOCK_BOOTTIME, time);
				time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128;
				if time_nanosecs >= clock_time_to_trigger_ctd_9 {
					break;
				}
			}

			while time_nanosecs < clock_time_to_trigger_ctd_9 { // make sure we're as close as possible
				// each loop should take ~20 ns
				libc::clock_gettime(CLOCK_BOOTTIME, time);
				time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128; // Convert from separate times to one unit
			}

			/* ==================================================
				* it's printing time
				* and then he printed all over the bad guys (jitter)
				* ==================================================
				*/
			libc::write(kmsg_file, MSG_BEFORE_9, COUNTER_MSG_LEN);
		}

		/* ================= COUNTDOWN 8 =================== */
		{
			let mut time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128; // Convert from separate times to one unit
			while time_nanosecs < clock_time_to_trigger_ctd_8_warmup {
				// each loop should take ~20 ns
				libc::clock_gettime(CLOCK_BOOTTIME, time);
				time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128; // Convert from separate times to one unit
			}
			/* ====================================================
			* do warmup prints to make performance more consistent
			* ====================================================
			*/
			let mut counter = 0;
			while counter < WARMUP_RUN_SIZE + 1 {
				libc::write(kmsg_file, WARMUP_MSG, WARMUP_MSG_LEN);
				counter += 1;
				libc::clock_gettime(CLOCK_BOOTTIME, time);
				time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128;
				if time_nanosecs >= clock_time_to_trigger_ctd_8 {
					break;
				}
			}

			while time_nanosecs < clock_time_to_trigger_ctd_8 { // make sure we're as close as possible
				// each loop should take ~20 ns
				libc::clock_gettime(CLOCK_BOOTTIME, time);
				time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128; // Convert from separate times to one unit
			}

			/* ==================================================
		 	* it's printing time
		 	* and then he printed all over the bad guys (jitter)
		 	* ==================================================
		 	*/
			libc::write(kmsg_file, MSG_BEFORE_8, COUNTER_MSG_LEN);
		}

		/* ================= COUNTDOWN 7 =================== */
		{
			let mut time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128; // Convert from separate times to one unit
			while time_nanosecs < clock_time_to_trigger_ctd_7_warmup {
				// each loop should take ~20 ns
				libc::clock_gettime(CLOCK_BOOTTIME, time);
				time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128; // Convert from separate times to one unit
			}
			/* ====================================================
			* do warmup prints to make performance more consistent
			* ====================================================
			*/
			let mut counter = 0;
			while counter < WARMUP_RUN_SIZE + 1 {
				libc::write(kmsg_file, WARMUP_MSG, WARMUP_MSG_LEN);
				counter += 1;
				libc::clock_gettime(CLOCK_BOOTTIME, time);
				time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128;
				if time_nanosecs >= clock_time_to_trigger_ctd_7 {
					break;
				}
			}

			while time_nanosecs < clock_time_to_trigger_ctd_7 { // make sure we're as close as possible
				// each loop should take ~20 ns
				libc::clock_gettime(CLOCK_BOOTTIME, time);
				time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128; // Convert from separate times to one unit
			}

			/* ==================================================
			* it's printing time
			* and then he printed all over the bad guys (jitter)
			* ==================================================
			*/
			libc::write(kmsg_file, MSG_BEFORE_7, COUNTER_MSG_LEN);
		}

		/* ================= COUNTDOWN 6 =================== */
		{
			let mut time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128; // Convert from separate times to one unit
			while time_nanosecs < clock_time_to_trigger_ctd_6_warmup {
				// each loop should take ~20 ns
				libc::clock_gettime(CLOCK_BOOTTIME, time);
				time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128; // Convert from separate times to one unit
			}
			/* ====================================================
			* do warmup prints to make performance more consistent
			* ====================================================
			*/
			let mut counter = 0;
			while counter < WARMUP_RUN_SIZE + 1 {
				libc::write(kmsg_file, WARMUP_MSG, WARMUP_MSG_LEN);
				counter += 1;
				libc::clock_gettime(CLOCK_BOOTTIME, time);
				time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128;
				if time_nanosecs >= clock_time_to_trigger_ctd_6 {
					break;
				}
			}

			while time_nanosecs < clock_time_to_trigger_ctd_6 { // make sure we're as close as possible
				// each loop should take ~20 ns
				libc::clock_gettime(CLOCK_BOOTTIME, time);
				time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128; // Convert from separate times to one unit
			}

			/* ==================================================
			* it's printing time
			* and then he printed all over the bad guys (jitter)
			* ==================================================
			*/
			libc::write(kmsg_file, MSG_BEFORE_6, COUNTER_MSG_LEN);
		}

		/* ================= COUNTDOWN 5 =================== */
		{
			let mut time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128; // Convert from separate times to one unit
			while time_nanosecs < clock_time_to_trigger_ctd_5_warmup {
				// each loop should take ~20 ns
				libc::clock_gettime(CLOCK_BOOTTIME, time);
				time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128; // Convert from separate times to one unit
			}
			/* ====================================================
			* do warmup prints to make performance more consistent
			* ====================================================
			*/
			let mut counter = 0;
			while counter < WARMUP_RUN_SIZE + 1 {
				libc::write(kmsg_file, WARMUP_MSG, WARMUP_MSG_LEN);
				counter += 1;
				libc::clock_gettime(CLOCK_BOOTTIME, time);
				time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128;
				if time_nanosecs >= clock_time_to_trigger_ctd_5 {
					break;
				}
			}

			while time_nanosecs < clock_time_to_trigger_ctd_5 { // make sure we're as close as possible
				// each loop should take ~20 ns
				libc::clock_gettime(CLOCK_BOOTTIME, time);
				time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128; // Convert from separate times to one unit
			}

			/* ==================================================
			* it's printing time
			* and then he printed all over the bad guys (jitter)
			* ==================================================
			*/
			libc::write(kmsg_file, MSG_BEFORE_5, COUNTER_MSG_LEN);
		}

		/* ================= COUNTDOWN 4 =================== */
		{
			let mut time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128; // Convert from separate times to one unit
			while time_nanosecs < clock_time_to_trigger_ctd_4_warmup {
				// each loop should take ~20 ns
				libc::clock_gettime(CLOCK_BOOTTIME, time);
				time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128; // Convert from separate times to one unit
			}
			/* ====================================================
			* do warmup prints to make performance more consistent
			* ====================================================
			*/
			let mut counter = 0;
			while counter < WARMUP_RUN_SIZE + 1 {
				libc::write(kmsg_file, WARMUP_MSG, WARMUP_MSG_LEN);
				counter += 1;
				libc::clock_gettime(CLOCK_BOOTTIME, time);
				time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128;
				if time_nanosecs >= clock_time_to_trigger_ctd_7 {
					break;
				}
			}

			while time_nanosecs < clock_time_to_trigger_ctd_4 { // make sure we're as close as possible
				// each loop should take ~20 ns
				libc::clock_gettime(CLOCK_BOOTTIME, time);
				time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128; // Convert from separate times to one unit
			}

			/* ==================================================
			* it's printing time
			* and then he printed all over the bad guys (jitter)
			* ==================================================
			*/
			libc::write(kmsg_file, MSG_BEFORE_4, COUNTER_MSG_LEN);
		}

		/* ================= COUNTDOWN 3 =================== */
		{
			let mut time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128; // Convert from separate times to one unit
			while time_nanosecs < clock_time_to_trigger_ctd_3_warmup {
				// each loop should take ~20 ns
				libc::clock_gettime(CLOCK_BOOTTIME, time);
				time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128; // Convert from separate times to one unit
			}
			/* ====================================================
			* do warmup prints to make performance more consistent
			* ====================================================
			*/
			let mut counter = 0;
			while counter < WARMUP_RUN_SIZE + 1 {
				libc::write(kmsg_file, WARMUP_MSG, WARMUP_MSG_LEN);
				counter += 1;
				libc::clock_gettime(CLOCK_BOOTTIME, time);
				time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128;
				if time_nanosecs >= clock_time_to_trigger_ctd_3 {
					break;
				}
			}

			while time_nanosecs < clock_time_to_trigger_ctd_3 { // make sure we're as close as possible
				// each loop should take ~20 ns
				libc::clock_gettime(CLOCK_BOOTTIME, time);
				time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128; // Convert from separate times to one unit
			}

			/* ==================================================
			* it's printing time
			* and then he printed all over the bad guys (jitter)
			* ==================================================
			*/
			libc::write(kmsg_file, MSG_BEFORE_3, COUNTER_MSG_LEN);
		}

		/* ================= COUNTDOWN 2 =================== */
		{
			let mut time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128; // Convert from separate times to one unit
			while time_nanosecs < clock_time_to_trigger_ctd_2_warmup {
				// each loop should take ~20 ns
				libc::clock_gettime(CLOCK_BOOTTIME, time);
				time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128; // Convert from separate times to one unit
			}
			/* ====================================================
			* do warmup prints to make performance more consistent
			* ====================================================
			*/
			let mut counter = 0;
			while counter < WARMUP_RUN_SIZE + 1 {
				libc::write(kmsg_file, WARMUP_MSG, WARMUP_MSG_LEN);
				counter += 1;
				libc::clock_gettime(CLOCK_BOOTTIME, time);
				time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128;
				if time_nanosecs >= clock_time_to_trigger_ctd_2 {
					break;
				}
			}

			while time_nanosecs < clock_time_to_trigger_ctd_2 { // make sure we're as close as possible
				// each loop should take ~20 ns
				libc::clock_gettime(CLOCK_BOOTTIME, time);
				time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128; // Convert from separate times to one unit
			}

			/* ==================================================
			* it's printing time
			* and then he printed all over the bad guys (jitter)
			* ==================================================
			*/
			libc::write(kmsg_file, MSG_BEFORE_2, COUNTER_MSG_LEN);
		}

		/* ================= COUNTDOWN 1 =================== */
		{
			let mut time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128; // Convert from separate times to one unit
			while time_nanosecs < clock_time_to_trigger_ctd_1_warmup {
				// each loop should take ~20 ns
				libc::clock_gettime(CLOCK_BOOTTIME, time);
				time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128; // Convert from separate times to one unit
			}
			/* ====================================================
			* do warmup prints to make performance more consistent
			* ====================================================
			*/
			let mut counter = 0;
			while counter < WARMUP_RUN_SIZE + 1 {
				libc::write(kmsg_file, WARMUP_MSG, WARMUP_MSG_LEN);
				counter += 1;
				libc::clock_gettime(CLOCK_BOOTTIME, time);
				time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128;
				if time_nanosecs >= clock_time_to_trigger_ctd_1 {
					break;
				}
			}

			while time_nanosecs < clock_time_to_trigger_ctd_1 { // make sure we're as close as possible
				// each loop should take ~20 ns
				libc::clock_gettime(CLOCK_BOOTTIME, time);
				time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128; // Convert from separate times to one unit
			}

			/* ==================================================
			* it's printing time
			* and then he printed all over the bad guys (jitter)
			* ==================================================
			*/
			libc::write(kmsg_file, MSG_BEFORE_1, BEFORE_1_LEN);
		}

		/* ================= MAIN MESSAGE ================== */
		{
			let mut time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128; // Convert from separate times to one unit
			while time_nanosecs < clock_time_to_trigger_warmup {
				// each loop should take ~20 ns
				libc::clock_gettime(CLOCK_BOOTTIME, time);
				time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128; // Convert from separate times to one unit
			}
			/* ====================================================
			* do warmup prints to make performance more consistent
			* ====================================================
			*/
			let mut counter = 0;
			while counter < WARMUP_RUN_SIZE + 1 {
				libc::write(kmsg_file, WARMUP_MSG, WARMUP_MSG_LEN);
				counter += 1;
				libc::clock_gettime(CLOCK_BOOTTIME, time);
				time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128;
				if time_nanosecs >= clock_time_to_trigger {
					break;
				}
			}

			while time_nanosecs < clock_time_to_trigger { // make sure we're as close as possible
				// each loop should take ~20 ns
				libc::clock_gettime(CLOCK_BOOTTIME, time);
				time_nanosecs = time.tv_sec as u128 * 1_000_000_000 + time.tv_nsec as u128; // Convert from separate times to one unit
			}

			/* ==================================================
			* it's printing time
			* and then he printed all over the bad guys (jitter)
			* ==================================================
			*/
			libc::write(kmsg_file, TEN_MIL_MSG, TEN_MIL_MSG_LEN);
		}

		libc::clock_gettime(CLOCK_BOOTTIME, time);
		println!("Finished at {} s {} ns", time.tv_sec, time.tv_nsec);

		/* ===============
		 * close resources
		 * ===============
		 */
		libc::close(kmsg_file);
	}
}
