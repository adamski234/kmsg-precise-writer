# kmsg-precise-writer
This is a small Rust (for now) program intended to output a kernel message at a time precise down to a microsecond.

# Usage
I run this as `sudo chrt --rr 99 taskset -c 3 ./target/release/uptime_runner $SECONDS` where `$SECONDS` is the second part of the intended timestamp. I also set `/proc/sys/kernel/sched_rt_runtime_us` to `-1`, as per [RedHat docs](https://access.redhat.com/documentation/en-us/red_hat_enterprise_linux_for_real_time/7/html/tuning_guide/real_time_throttling), to avoid preemption. However, these settings seem to have little to no effect on the actual accuracy of the program.
Don't forget to set `WARMUP_ROUND_LENGTH`. It's the average value calculated that commit [298b5f6](https://github.com/adamski234/kmsg-precise-writer/commit/298b5f61266c837e3a1b4d984a8330f82708c7e1) spits out.

# Precision
On a Raspberry Pi 3B+ this can achieve sub-100 microsecond precision. I am not sure it is possible to get a better time.
