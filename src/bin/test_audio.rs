use cpal::traits::{DeviceTrait, HostTrait};

fn main() {
    let host = cpal::default_host();
    match host.default_input_device() {
        Some(device) => {
            println!("Default input device: {}", device.name().unwrap_or_else(|_| "unknown".to_string()));
            match device.default_input_config() {
                Ok(config) => println!("Supported config: {:?}", config),
                Err(e) => println!("Detailed Error: {:?}", e),
            }
        },
        None => {
            println!("No default input device found.");
            println!("Available input devices:");
            if let Ok(devices) = host.input_devices() {
                for dev in devices {
                    println!(" - {}", dev.name().unwrap_or_else(|_| "unknown".to_string()));
                }
            }
        }
    }
}