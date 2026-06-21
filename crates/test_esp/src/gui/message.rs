use alloc::borrow::Cow;
use embassy_time::Instant;

#[derive(Clone)]
pub struct Message {
    pub text: Cow<'static, str>,
    pub timestamp: Instant,
}

impl Message {
    pub fn now(text: Cow<'static, str>) -> Message {
        Message {
            text,
            timestamp: Instant::now(),
        }
    }
}
