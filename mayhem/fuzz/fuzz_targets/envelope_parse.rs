#![no_main]
use libfuzzer_sys::fuzz_target;

extern crate melib;

use melib::Envelope;

fuzz_target!(|data: &[u8]| {
    // Fuzz melib's RFC5322 e-mail/MIME parser (same entry point as upstream fuzz/).
    let _envelope = Envelope::from_bytes(data, None);
});
