/*
* This crate provides the structures needed for routing the debug information handled by the tracing crate
* through the purple debug functions.
*
* adapted from https://burgers.io/custom-logging-in-rust-using-tracing
*/

struct PrintlnVisitor {
    buffer: String,
}

impl PrintlnVisitor {
    fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    fn append_to_buffer(&mut self, content: &str) {
        self.buffer.push_str(content);
    }
}

impl tracing::field::Visit for PrintlnVisitor {
    fn record_f64(&mut self, _field: &tracing::field::Field, _value: f64) {
        // self.append_to_buffer(&format!("f64 field={} value={}", field.name(), value));
    }

    fn record_i64(&mut self, _field: &tracing::field::Field, _value: i64) {
        // self.append_to_buffer(&format!("i64 field={} value={}", field.name(), value));
    }

    fn record_u64(&mut self, _field: &tracing::field::Field, _value: u64) {
        // self.append_to_buffer(&format!("u64 field={} value={}", field.name(), value));
    }

    fn record_bool(&mut self, _field: &tracing::field::Field, _value: bool) {
        // self.append_to_buffer(&format!("bool field={} value={}", field.name(), value));
    }

    fn record_str(&mut self, _field: &tracing::field::Field, _value: &str) {
        // self.append_to_buffer(&format!("str field={} value={}", field.name(), value));
    }

    fn record_error(
        &mut self,
        _field: &tracing::field::Field,
        _value: &(dyn std::error::Error + 'static),
    ) {
        // self.append_to_buffer(&format!("error field={} value={}", field.name(), value));
    }

    fn record_debug(&mut self, _field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        // self.append_to_buffer(&format!("debug field={} value={:?}", field.name(), value));
        // NOTE: presage seems to use debug events only. I am not interested in anything else right now
        self.append_to_buffer(&format!("{:?}", value));
    }
}

pub struct PurpleDebugLayer;
impl<S> tracing_subscriber::Layer<S> for PurpleDebugLayer where S: tracing::Subscriber, {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        // disregard all tracing events except those originating from presage
        // TODO: allow more fine-grained control via RUST_LOG environment
        // TODO: include libsignal Level(Trace) events by default, too?
        if event.metadata().target().starts_with("presage") {
            let purple_debug_level = match *event.metadata().level() {
                tracing_core::metadata::Level::TRACE => crate::bridge_structs::PURPLE_DEBUG_MISC,
                tracing_core::metadata::Level::DEBUG => crate::bridge_structs::PURPLE_DEBUG_INFO,
                tracing_core::metadata::Level::INFO => crate::bridge_structs::PURPLE_DEBUG_INFO,
                tracing_core::metadata::Level::WARN => crate::bridge_structs::PURPLE_DEBUG_WARNING,
                tracing_core::metadata::Level::ERROR => crate::bridge_structs::PURPLE_DEBUG_ERROR,
            };
            // println!("Got event!");
            // println!("  level={:?}", event.metadata().level());
            // println!("  target={:?}", event.metadata().target());
            // println!("  name={:?}", event.metadata().name());
            // for field in event.fields() {
            //     println!("  field={}", field.name());
            // }
            let mut visitor = PrintlnVisitor::new();
            event.record(&mut visitor);
            let target = event.metadata().target();
            let message = visitor.buffer;
            crate::bridge::purple_debug(
                std::ptr::null_mut(),
                purple_debug_level,
                format!("{target} {message}\n"),
            );
        }
    }
}
