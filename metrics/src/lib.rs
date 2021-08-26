#[macro_use]
extern crate lazy_static;

mod id;
mod recorder;
use recorder::Recorder;

mod duration;

mod snapshot;
use snapshot::Snapshot;

pub use id::*;

mod ip;
pub use ip::encode_addr;
pub use ip::init_local_ip;

mod sender;
use sender::Sender;

use once_cell::sync::OnceCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tokio::sync::mpsc::channel;

pub static RECORDER: OnceCell<Recorder> = OnceCell::new();
static INITED: AtomicBool = AtomicBool::new(false);

pub fn init(addr: &str) {
    if addr.len() > 0 {
        match INITED.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => {
                let (tx, rx) = channel::<Snapshot>(256);
                let recorder = Recorder::new(tx);
                RECORDER.set(recorder).ok().expect("recorder init once");

                let send = Sender::new(rx, addr);
                send.start_sending();
            }
            Err(_) => {}
        }
    }
}

pub fn duration(key: &'static str, d: Duration, is_success: bool) {
    if let Some(recorder) = RECORDER.get() {
        recorder.duration(key, d, is_success);
    }
}

#[inline(always)]
pub fn duration_with_service(key: &'static str, d: Duration, metric_id: usize, is_success: bool) {
    if let Some(recorder) = RECORDER.get() {
        recorder.duration_with_service(key, d, metric_id, is_success);
    }
}
#[inline(always)]
pub fn counter_with_service(key: &'static str, c: usize, metric_id: usize) {
    if let Some(recorder) = RECORDER.get() {
        recorder.counter_with_service(key, c, metric_id);
    }
}
