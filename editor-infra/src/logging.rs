use tracing::{Level, Subscriber};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

pub fn init_logging() {
    let fmt_layer = fmt::layer()
        .with_target(false)
        .with_level(true)
        .with_thread_ids(false)
        .with_thread_names(false);

    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    Registry::default()
        .with(filter_layer)
        .with(fmt_layer)
        .init();

    tracing::info!("Logging initialized");
}

pub fn init_logging_with_level(level: Level) {
    let fmt_layer = fmt::layer()
        .with_target(false)
        .with_level(true)
        .with_thread_ids(false)
        .with_thread_names(false);

    let filter_layer = EnvFilter::new(level.to_string());

    Registry::default()
        .with(filter_layer)
        .with(fmt_layer)
        .init();

    tracing::info!("Logging initialized with level: {:?}", level);
}