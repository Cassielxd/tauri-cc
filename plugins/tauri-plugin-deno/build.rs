const COMMANDS: &[&str] = &["send_to_deno", "create_deno_channel", "listen_on", "unlisten_from", "close_deno_channel"];

fn main() {
  tauri_plugin::Builder::new(COMMANDS).android_path("android").ios_path("ios").build();
}
