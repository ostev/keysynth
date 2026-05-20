pub const DEFAULT_CHANNEL_CAPACITY: usize = 16;

#[macro_export]
macro_rules! receiver {
    ($message:ident) => {
        static CHANNEL: embassy_sync::channel::Channel<
            esp_sync::RawMutex,
            $message,
            { $crate::concurrency::DEFAULT_CHANNEL_CAPACITY },
        > = embassy_sync::channel::Channel::new();

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

    ($message:ident, capacity: $capacity:expr) => {
        static CHANNEL: embassy_sync::channel::Channel<
            esp_sync::RawMutex,
            $message,
            { $capacity },
        > = embassy_sync::channel::Channel::new();

        #[inline]
        pub fn receiver()
        -> embassy_sync::channel::Receiver<'static, esp_sync::RawMutex, $message, { $capacity }> {
            CHANNEL.receiver()
        }
    };
}

#[macro_export]
macro_rules! sender {
    ($message:ident) => {
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
    };

    ($message:ident, $capacity:expr) => {
        static CHANNEL: embassy_sync::channel::Channel<
            esp_sync::RawMutex,
            $message,
            { $capacity },
        > = embassy_sync::channel::Channel::new();

        #[inline]
        pub fn sender()
        -> embassy_sync::channel::Sender<'static, esp_sync::RawMutex, $message, { $capacity }> {
            CHANNEL.receiver()
        }
    };
}
