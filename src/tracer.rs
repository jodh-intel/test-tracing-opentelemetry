// Copyright (c) 2020-2021 Intel Corporation
//
// SPDX-License-Identifier: Apache-2.0
//

use anyhow::Result;
//use opentelemetry::{global, sdk::trace::Config, sdk::trace::TracerProvider};
use opentelemetry::{global, sdk::trace::Config, trace::TracerProvider};
use slog::{o, Logger};
use tracing_opentelemetry::OpenTelemetryLayer;
//use tracing_subscriber::Registry;

const NAME: &str = "my tracer";

pub fn setup_tracing(logger: &Logger) -> Result<()> {
    let logger = logger.new(o!("subsystem" => "vsock-tracer"));

    let exporter = vsock_exporter::Exporter::builder()
        .with_logger(&logger)
        .init();

    let config = Config::default();

    let builder = opentelemetry::sdk::trace::TracerProvider::builder()
        .with_simple_exporter(exporter)
        .with_config(config);

    let provider = builder.build();

    let version = None;
    let tracer = provider.get_tracer(NAME, version);

    let _global_provider = global::set_tracer_provider(provider);

    let layer = OpenTelemetryLayer::new(tracer);

    //let tracer = global::tracer_provider().get_tracer(name);
    //let layer = OpenTelemetryLayer::with_tracer(tracer);

    //------------------------------

    //let tracer = global::trace_provider().get_tracer("example");

    //let opentelemetry = OpenTelemetryLayer::with_tracer(tracer);
    //let subscriber = Registry::default().with(opentelemetry);

    //------------------------------

    //let layer = OpenTelemetryLayer::with_tracer(provider);
    //let subscriber = Registry::default().with(layer);

    //let trace_subscriber = tracer::get_subscriber(NAME);
    tracing::subscriber::set_global_default(trace_subscriber)?;

    Ok(())
}

pub fn end_tracing() {
    global::shutdown_tracer_provider();
}
