use core::iter;
use embassy_sync::{blocking_mutex::raw::RawMutex, channel::Receiver};

pub const DEFAULT_CHANNEL_CAPACITY: usize = 16;

/// Convenience macro to create a channel with paired senders and receivers.
#[macro_export]
macro_rules! channel {
    ($message:ty) => {
        static CHANNEL: embassy_sync::channel::Channel<
            esp_sync::RawMutex,
            $message,
            { $crate::concurrency::DEFAULT_CHANNEL_CAPACITY },
        > = embassy_sync::channel::Channel::new();

        #[inline]
        pub fn sender() -> embassy_sync::channel::Sender<
            'static,
            esp_sync::RawMutex,
            $message,
            { $crate::concurrency::DEFAULT_CHANNEL_CAPACITY },
        > {
            CHANNEL.sender()
        }

        #[inline]
        pub fn receiver() -> embassy_sync::channel::Receiver<
            'static,
            esp_sync::RawMutex,
            $message,
            { $crate::concurrency::DEFAULT_CHANNEL_CAPACITY },
        > {
            CHANNEL.receiver()
        }
    };

    ($message:ty, $capacity:expr) => {
        static CHANNEL: embassy_sync::channel::Channel<
            esp_sync::RawMutex,
            $message,
            { $capacity },
        > = embassy_sync::channel::Channel::new();

        #[inline]
        pub fn sender()
        -> embassy_sync::channel::Sender<'static, esp_sync::RawMutex, $message, { $capacity }> {
            CHANNEL.sender()
        }

        #[inline]
        pub fn receiver()
        -> embassy_sync::channel::Receiver<'static, esp_sync::RawMutex, $message, { $capacity }> {
            CHANNEL.receiver()
        }
    };
}

/// Receive all the contents of a channel, awaiting the first value.
pub async fn receive_all<'ch, M: RawMutex, T, const N: usize>(
    receiver: &Receiver<'ch, M, T, N>,
) -> impl Iterator<Item = T> {
    let first_message = receiver.receive().await;
    iter::once(first_message).chain(try_receive_all(&receiver))
}

/// Attempt to receive all the contents of a channel if there are any.
pub fn try_receive_all<'ch, M: RawMutex, T, const N: usize>(
    receiver: &Receiver<'ch, M, T, N>,
) -> impl Iterator<Item = T> {
    iter::from_fn(move || receiver.try_receive().ok())
}
