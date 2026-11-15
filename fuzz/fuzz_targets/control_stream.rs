#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: &[u8]| tilling_fuzz_cases::control_stream::check(input));
