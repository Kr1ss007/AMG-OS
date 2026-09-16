//! Astrophage Shell: Process 2 Diagnostic Reporter
//!
//! Subscribes to Process 1's continuous Astrophage event buffer across e-bus.
//! Prepares pre-formatted, privacy-sanitized GitHub Issue diagnostic reports.

use amgos_protocol::ebus::{DesktopRequest, EventBusClient, SystemEvent, DEFAULT_EBUS_SOCKET_PATH};
use astrophage_shell::DiagnosticReportBuilder;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let description = if args.len() > 1 {
        args[1..].join(" ")
    } else {
        "Hardware Diagnostic Report".to_string()
    };

    let mut builder =
        DiagnosticReportBuilder::new("Intel Core i5-13420H / NVIDIA RTX 3050 Laptop GPU");
    builder.set_user_description(&description);

    // Query Process 1 for rolling Astrophage diagnostic buffer over e-bus
    if let Ok(client) = EventBusClient::connect(DEFAULT_EBUS_SOCKET_PATH) {
        let req = DesktopRequest::GetAstrophageBuffer { max_entries: 50 };
        let _ = client.send_request(&req);

        // Await snapshot response
        if let Ok(SystemEvent::AstrophageBufferSnapshot { entries }) = client.read_event() {
            builder.set_events(entries);
        }
    }

    let payload = builder.build_github_payload();
    let formatted = payload.to_markdown();

    // In production Wayland desktop session, this opens the review surface.
    // In CLI / stdout context, it prints the scrubbed report markdown.
    println!("{formatted}");

    Ok(())
}
