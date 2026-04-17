use std::thread;
use std::time::Duration;

fn main() {
    println!();
    println!("================================");
    println!("  FLASHPOINT  v0.1.0");
    println!("  stage1 / boot-rom stub");
    println!("================================");
    println!();
    println!("[STAGE1] platform  : ESP32");
    println!("[STAGE1] features  : display_tft | input_touch");
    println!("[STAGE1] SD card   : not present");
    println!("[STAGE1] internal  : boot-rom stub (embedded)");
    println!("[STAGE1] validating header...");
    thread::sleep(Duration::from_millis(200));
    println!("[STAGE1] header OK");
    println!("[STAGE1] jumping to boot-rom...");
    println!();
    thread::sleep(Duration::from_millis(100));
    println!("================================");
    println!("  FLASHPOINT  OK");
    println!("================================");
    println!();
    println!("system ready.");
    println!("(select button reboots)");

    loop {
        thread::sleep(Duration::from_secs(60));
    }
}
