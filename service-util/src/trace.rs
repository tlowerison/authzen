use hyper::header::HeaderName;
use hyper::Request;
use opentelemetry::propagation::TextMapPropagator;
use opentelemetry::sdk::{
    propagation::TraceContextPropagator,
    trace::{self, Sampler},
};
use opentelemetry::trace::TraceContextExt;
use std::collections::HashMap;
use tracing::{info, Span};
use tracing_error::ErrorLayer;
use tracing_log::LogTracer;
use tracing_opentelemetry::{OpenTelemetryLayer, OpenTelemetrySpanExt};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};
use tracing_tree::HierarchicalLayer;

#[macro_export]
macro_rules! instrument_field {
    ($ident:ident) => {
        tracing::Span::current().record(stringify!($ident), &&*format!("{:?}", $ident));
    };
    ($name:literal, $expr:expr) => {
        tracing::Span::current().record($name, &&*format!("{:?}", $expr));
    };
}

pub static TRACEPARENT: HeaderName = HeaderName::from_static("traceparent");
pub static TRACESTATE: HeaderName = HeaderName::from_static("tracestate");

pub fn install_tracing(should_enable_telemetry: bool) -> Result<(), anyhow::Error> {
    LogTracer::init()?;
    info!("log tracing registry initialized");

    if should_enable_telemetry {
        install_jaeger_enabled_tracing()?;
    } else {
        let registry = Registry::default()
            .with(EnvFilter::from_default_env())
            .with(
                HierarchicalLayer::new(2)
                    .with_bracketed_fields(true)
                    .with_indent_lines(true)
                    .with_targets(true),
            )
            .with(ErrorLayer::default());

        tracing::subscriber::set_global_default(registry).expect("failed to set tracing subscriber");
    }

    info!("set global default tracing subscriber");

    Ok(())
}

pub fn teardown_tracing() {
    opentelemetry::global::shutdown_tracer_provider();
}

// suggestion: add the environment variable OTEL_PROPAGATORS=tracecontext
// propagate your trace ids if using service_util::TraceId in your server stack
fn install_jaeger_enabled_tracing() -> Result<(), anyhow::Error> {
    let tracer = opentelemetry_jaeger::new_agent_pipeline()
        .with_endpoint("localhost:6831")
        .with_max_packet_size(9_216)
        .with_auto_split_batch(true)
        .with_instrumentation_library_tags(false)
        .with_trace_config(
            trace::config()
                .with_sampler(Sampler::AlwaysOn)
                .with_max_events_per_span(512)
                .with_max_attributes_per_event(16),
        )
        .install_batch(opentelemetry::runtime::Tokio)?;

    info!("jaeger tracing registry initialized");

    let registry = Registry::default()
        .with(EnvFilter::from_default_env())
        .with(
            HierarchicalLayer::new(2)
                .with_bracketed_fields(true)
                .with_indent_lines(true)
                .with_targets(true),
        )
        .with(OpenTelemetryLayer::new(tracer))
        .with(ErrorLayer::default());

    tracing::subscriber::set_global_default(registry).expect("failed to set subscriber");

    opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new());

    Ok(())
}

pub fn set_trace_parent(req: &Request<hyper::Body>, span: Span) -> Span {
    let propagator = TraceContextPropagator::new();
    if let Some(traceparent) = req.headers().get(&TRACEPARENT).and_then(|x| x.to_str().ok()) {
        // Propagator::extract only works with HashMap<String, String>
        let mut headers = match req.headers().get(&TRACESTATE).and_then(|x| x.to_str().ok()) {
            Some(tracestate) => {
                let mut headers = HashMap::with_capacity(2);
                headers.insert("tracestate".to_string(), tracestate.to_string());
                headers
            }
            None => HashMap::with_capacity(1),
        };
        headers.insert("traceparent".to_string(), traceparent.to_string());

        let context = propagator.extract(&headers);
        span.set_parent(context);
    }
    span
}

pub fn traceparent() -> Option<String> {
    let context = Span::current().context();
    let span_ref = context.span();
    let span_context = span_ref.span_context();
    if !span_context.is_valid() {
        return None;
    }
    let trace_id = span_context.trace_id();
    let span_id = span_context.span_id();
    let flags = span_context.trace_flags().to_u8();
    Some(format!("00-{trace_id}-{span_id}-{flags:02x}"))
}
