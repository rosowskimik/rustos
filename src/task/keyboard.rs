use core::{
    pin::Pin,
    task::{Context, Poll},
};

use crossbeam_queue::ArrayQueue;
use futures_util::{task::AtomicWaker, Stream, StreamExt};
use pc_keyboard::{layouts, DecodedKey, Keyboard, ScancodeSet1};
use spin::Once;

use crate::{print, println};

static SCANCODE_QUEUE: Once<ArrayQueue<u8>> = Once::new();
static WAKER: AtomicWaker = AtomicWaker::new();

/// Called by the keyboard interrupt handler.
///
/// NOTE: Must not block or allocate.
pub(crate) fn add_scancode(scancode: u8) {
    if let Some(queue) = SCANCODE_QUEUE.get() {
        if queue.push(scancode).is_err() {
            println!(
                "WARNING: scancode queue full; dropping keyboard input (scancode: {})",
                scancode
            );
        } else {
            WAKER.wake();
        }
    } else {
        println!(
            "WARNING: scancode queue uninitialized; dropping keyboard input (scancode: {})",
            scancode
        );
    }
}

pub async fn print_keypresses() {
    let mut scancodes = ScancodeStream::new();
    let mut keyboard = Keyboard::new(
        layouts::Us104Key,
        ScancodeSet1,
        pc_keyboard::HandleControl::Ignore,
    );

    while let Some(scancode) = scancodes.next().await {
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            if let Some(key) = keyboard.process_keyevent(key_event) {
                match key {
                    DecodedKey::Unicode(c) => print!("{}", c),
                    DecodedKey::RawKey(key) => print!("{:?}", key),
                }
            }
        }
    }
}

pub struct ScancodeStream {
    _sealed: (),
}

impl ScancodeStream {
    /// Creates a new [`ScancodeStream`].
    ///
    /// # Panics
    ///
    /// Panics if called more than once.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        if SCANCODE_QUEUE.is_completed() {
            panic!("ScancodeStream::new called more than once");
        }

        SCANCODE_QUEUE.call_once(|| ArrayQueue::new(100));
        Self { _sealed: () }
    }
}

impl Stream for ScancodeStream {
    type Item = u8;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let queue = SCANCODE_QUEUE.get().expect("scancode queue uninitialized");

        if let Some(scancode) = queue.pop() {
            return Poll::Ready(Some(scancode));
        }

        WAKER.register(cx.waker());
        match queue.pop() {
            Some(scancode) => {
                WAKER.take();
                Poll::Ready(Some(scancode))
            }
            None => Poll::Pending,
        }
    }
}
