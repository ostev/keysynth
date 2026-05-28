pub const DEFAULT_CHANNEL_CAPACITY: usize = 16;

#[macro_export]
macro_rules! receiver {
    ($message:ty) => {
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

    ($message:ty, capacity: $capacity:expr) => {
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
            CHANNEL.receiver()
        }
    };
}
