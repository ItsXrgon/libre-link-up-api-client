//! Example of averaging glucose readings in the background
//!
//! Run with: cargo run --example averaged_reading

use libre_link_up_api_client::LibreLinkUpClient;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = LibreLinkUpClient::simple(
        "your_email@example.com",
        "your_password",
        None, // Global endpoint; redirects to the account's region at login
    )?;

    println!("Polling every 15 seconds, averaging every 5 distinct readings...");

    let handle = client
        .poll_averaged(5, Duration::from_secs(15), |average, samples, _history| {
            println!("\n=== Average Calculated ===");
            println!("Average value: {:.1} mg/dL", average.value_mg_dl);
            println!("Average trend: {:?}", average.trend);
            println!("Based on {} readings:", samples.len());
            for (i, reading) in samples.iter().enumerate() {
                println!(
                    "  {}. {:.1} mg/dL at {}",
                    i + 1,
                    reading.value_mg_dl,
                    reading.timestamp
                );
            }
        })
        .await?;

    // Let it run for a while (this would normally run indefinitely)
    tokio::time::sleep(Duration::from_secs(120)).await;

    // Stop the background task
    handle.abort();

    Ok(())
}
