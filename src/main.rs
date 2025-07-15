use anyhow::{Context, Result};
use arboard::Clipboard;
use clap::Parser;
use enigo::{Enigo, Key, Direction::{Press, Release, Click}, Settings, Keyboard};
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};
use tracing::{error, info, warn};
use winit::event_loop::{ControlFlow, EventLoop};

#[derive(Parser, Debug)]
#[command(name = "easypaste")]
#[command(about = "A cross-platform clipboard automation tool")]
struct Args {
    /// Path to the text file containing delimited content
    #[arg(short, long)]
    file: PathBuf,

    /// Delimiter character (default: %%%)
    #[arg(short, long, default_value = "%%%")]
    delimiter: String,

    /// Configuration file path
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Disable automatic pasting of clipboard contents after loading segment
    #[arg(long)]
    no_paste: bool,

    /// Enable verbose logging (info level)
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    delimiter: String,
    file_path: PathBuf,
    hotkey_modifiers: Vec<String>,
    hotkey_key: String,
    paste: Option<bool>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            delimiter: "%%%".to_string(),
            file_path: PathBuf::from("input.txt"),
            hotkey_modifiers: vec!["CTRL".to_string(), "SHIFT".to_string()],
            hotkey_key: "B".to_string(),
            paste: Some(true),
        }
    }
}

const DONATE_LINK: &str = "https://donate.stripe.com/8x28wObdhgoV8aVaQW6J202";

fn show_donation_prompt() {
    print!("\nDo you like the tool and want to buy me a coffee? [y/N]: ");
    io::stdout().flush().unwrap();
    
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        if input.trim().to_lowercase() == "y" {
            if let Err(e) = webbrowser::open(DONATE_LINK) {
                error!("Failed to open browser: {}", e);
            }
        }
    }
}

struct TextManager {
    content: Arc<Mutex<String>>,
    position: Arc<AtomicUsize>,
    delimiter: String,
}

impl TextManager {
    fn new(file_path: PathBuf, delimiter: String) -> Result<Self> {
        let content = fs::read_to_string(&file_path)
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

        Ok(Self {
            content: Arc::new(Mutex::new(content)),
            position: Arc::new(AtomicUsize::new(0)),
            delimiter,
        })
    }

    fn get_next_segment(&self) -> Option<(String, Option<String>)> {
        let content = self.content.lock().unwrap();
        let current_pos = self.position.load(Ordering::Relaxed);

        if current_pos >= content.len() {
            // Don't reset, just return None to indicate we're done
            if content.is_empty() {
                return None;
            }
            return None;
        }

        let remaining = &content[current_pos..];
        
        match remaining.find(&self.delimiter) {
            Some(delimiter_pos) => {
                let segment = remaining[..delimiter_pos].to_string();
                
                // Check if there's an internal note on the same line after the delimiter
                let after_delimiter = &remaining[delimiter_pos + self.delimiter.len()..];
                let internal_note = if let Some(newline_pos) = after_delimiter.find('\n') {
                    if newline_pos > 0 {
                        Some(after_delimiter[..newline_pos].trim().to_string())
                    } else {
                        None
                    }
                } else if !after_delimiter.is_empty() {
                    Some(after_delimiter.trim().to_string())
                } else {
                    None
                };
                
                // Move position past the delimiter, any internal note, and the newline
                let after_note = &remaining[delimiter_pos + self.delimiter.len()..];
                if let Some(newline_pos) = after_note.find('\n') {
                    self.position.store(current_pos + delimiter_pos + self.delimiter.len() + newline_pos + 1, Ordering::Relaxed);
                } else {
                    self.position.store(current_pos + delimiter_pos + self.delimiter.len() + after_note.len(), Ordering::Relaxed);
                }
                
                Some((segment, internal_note))
            }
            None => {
                // No more delimiters, return rest of content
                if !remaining.is_empty() {
                    self.position.store(content.len(), Ordering::Relaxed);
                    Some((remaining.to_string(), None))
                } else {
                    None
                }
            }
        }
    }

    fn preview_next_segment(&self) -> Option<(String, Option<String>)> {
        let content = self.content.lock().unwrap();
        let current_pos = self.position.load(Ordering::Relaxed);

        if current_pos >= content.len() {
            // If we're at the end, there are no more segments
            return None;
        } else {
            let remaining = &content[current_pos..];
            match remaining.find(&self.delimiter) {
                Some(delimiter_pos) => {
                    let segment = remaining[..delimiter_pos].to_string();
                    
                    // Check for internal note
                    let after_delimiter = &remaining[delimiter_pos + self.delimiter.len()..];
                    let internal_note = if let Some(newline_pos) = after_delimiter.find('\n') {
                        if newline_pos > 0 {
                            Some(after_delimiter[..newline_pos].trim().to_string())
                        } else {
                            None
                        }
                    } else if !after_delimiter.is_empty() {
                        Some(after_delimiter.trim().to_string())
                    } else {
                        None
                    };
                    
                    Some((segment, internal_note))
                },
                None => {
                    if !remaining.is_empty() {
                        Some((remaining.to_string(), None))
                    } else {
                        None
                    }
                }
            }
        }
    }
}

struct EasypasteApp {
    text_manager: Arc<TextManager>,
        clipboard: Arc<Mutex<Clipboard>>,
        hotkey_manager: Arc<Mutex<GlobalHotKeyManager>>,
}

impl EasypasteApp {
    fn new(config: Config) -> Result<Self> {
        let text_manager = Arc::new(TextManager::new(config.file_path, config.delimiter)?);
        let clipboard = Arc::new(Mutex::new(
            Clipboard::new().context("Failed to initialize clipboard")?,
        ));

        let hotkey_manager = GlobalHotKeyManager::new()
            .context("Failed to initialize global hotkey manager")?;

        Ok(Self {
            text_manager,
            clipboard,
            hotkey_manager: Arc::new(Mutex::new(hotkey_manager)),
        })
    }

    fn register_hotkey(&self, config: &Config) -> Result<HotKey> {
        let mut modifiers = Modifiers::empty();
        for modifier in &config.hotkey_modifiers {
            match modifier.to_uppercase().as_str() {
                "CMD" | "WIN" | "META" => modifiers |= Modifiers::SUPER,
                "CTRL" | "CONTROL" => modifiers |= Modifiers::CONTROL,
                "ALT" | "OPTION" => modifiers |= Modifiers::ALT,
                "SHIFT" => modifiers |= Modifiers::SHIFT,
                _ => warn!("Unknown modifier: {}", modifier),
            }
        }

        let key_code = match config.hotkey_key.to_uppercase().as_str() {
            "A" => Code::KeyA, "B" => Code::KeyB, "C" => Code::KeyC, "D" => Code::KeyD,
            "E" => Code::KeyE, "F" => Code::KeyF, "G" => Code::KeyG, "H" => Code::KeyH,
            "I" => Code::KeyI, "J" => Code::KeyJ, "K" => Code::KeyK, "L" => Code::KeyL,
            "M" => Code::KeyM, "N" => Code::KeyN, "O" => Code::KeyO, "P" => Code::KeyP,
            "Q" => Code::KeyQ, "R" => Code::KeyR, "S" => Code::KeyS, "T" => Code::KeyT,
            "U" => Code::KeyU, "V" => Code::KeyV, "W" => Code::KeyW, "X" => Code::KeyX,
            "Y" => Code::KeyY, "Z" => Code::KeyZ,
            "1" => Code::Digit1, "2" => Code::Digit2, "3" => Code::Digit3,
            "4" => Code::Digit4, "5" => Code::Digit5, "6" => Code::Digit6,
            "7" => Code::Digit7, "8" => Code::Digit8, "9" => Code::Digit9, "0" => Code::Digit0,
            "SPACE" => Code::Space,
            "ENTER" | "RETURN" => Code::Enter,
            _ => {
                return Err(anyhow::anyhow!("Unsupported key: {}", config.hotkey_key));
            }
        };

        let hotkey = HotKey::new(Some(modifiers), key_code);
        self.hotkey_manager
            .lock()
            .unwrap()
            .register(hotkey)
            .with_context(|| {
                format!(
                    "Failed to register hotkey: {:?}+{}",
                    modifiers, config.hotkey_key
                )
            })?;

            println!("Registered hotkey: {:?}+{}", modifiers, config.hotkey_key);
        Ok(hotkey)
    }



    fn run(&self, config: Config) -> Result<()> {
        let hotkey = self.register_hotkey(&config)?;

        println!("Easypaste is running. Press the configured hotkey to paste next segment.");
        println!("File: {}", config.file_path.display());
        println!("Delimiter: '{}'", config.delimiter);
        println!("Auto-paste: {}", config.paste.unwrap_or(true));
        println!("Press Ctrl+C to exit");

        // Show initial preview
        if let Some((segment, note)) = self.text_manager.preview_next_segment() {
            if !segment.is_empty() {
                println!("Next segment preview:");
                println!("{}", segment);
                if let Some(note_text) = note {
                    println!("[Note: {}]", note_text);
                }
                println!("---");
            }
        }

        // Create event loop (required for hotkey system integration)
        let event_loop = EventLoop::new().context("Failed to create event loop")?;
        
        let text_manager = Arc::clone(&self.text_manager);
        let clipboard = Arc::clone(&self.clipboard);
        let should_exit = Arc::new(Mutex::new(false));

        // Set up event loop
        event_loop.run(move |event, elwt| {
            elwt.set_control_flow(ControlFlow::Wait);
            
            match event {
                winit::event::Event::WindowEvent { event, .. } => match event {
                    winit::event::WindowEvent::CloseRequested => {
                        info!("Shutting down...");
                        elwt.exit();
                    }
                    _ => {}
                },
                _ => {}
            }
            
            // Handle global hotkey events
            if let Ok(hotkey_event) = GlobalHotKeyEvent::receiver().try_recv() {
                // Check if we should exit before processing hotkey events
                if *should_exit.lock().unwrap() {
                    return;
                }
                
                // Only react to key press events, not key release events
                if hotkey_event.state == HotKeyState::Pressed {
                    info!("Hotkey triggered: {:?}", hotkey_event);
                    
                    // Get and copy the actual segment
                    if let Some((segment, _)) = text_manager.get_next_segment() {
                        if !segment.is_empty() {
                            if let Ok(mut cb) = clipboard.lock() {
                                if let Err(e) = cb.set_text(&segment) {
                                    error!("Failed to set clipboard: {}", e);
                                } else {
                                    info!("Set clipboard to: {:.50}...", segment);
                                    
                                    // Paste the contents if enabled
                                    if config.paste.unwrap_or(true) {
                                        // We need to drop the clipboard lock before trying to paste
                                        drop(cb);
                                        
                                        // Create a new Enigo instance for pasting
                                        if let Ok(mut enigo) = Enigo::new(&Settings::default()) {
                                            // Small delay to ensure clipboard is set
                                            let sleep_duration = if cfg!(target_os = "macos") {
                                                100
                                            } else if cfg!(target_os = "windows") {
                                                2000
                                            } else {
                                                100 // Default for other platforms
                                            };
                                            thread::sleep(Duration::from_millis(sleep_duration));
                                            
                                            // Use appropriate modifier key based on platform
                                            let modifier_key = if cfg!(target_os = "macos") {
                                                Key::Meta
                                            } else {
                                                Key::Control
                                            };
                                            
                                            if let Err(e) = enigo.key(modifier_key, Press) {
                                                error!("Failed to press modifier key: {}", e);
                                            } else if let Err(e) = enigo.key(Key::Unicode('v'), Click) {
                                                error!("Failed to click V key: {}", e);
                                            } else if let Err(e) = enigo.key(modifier_key, Release) {
                                                error!("Failed to release modifier key: {}", e);
                                            } else {
                                                info!("Pasted clipboard contents");
                                            }
                                        } else {
                                            error!("Failed to initialize Enigo for pasting");
                                        }
                                    }
                                }
                            }
                        }
                        
                        // Check if there are more segments after this one
                        if let Some((segment, note)) = text_manager.preview_next_segment() {
                            if !segment.is_empty() {
                                println!("Next segment preview:");
                                println!("{}", segment);
                                if let Some(note_text) = note {
                                    println!("[Note: {}]", note_text);
                                }
                                println!("---");
                            }
                        } else {
                            // No more segments after this one, quit immediately
                            info!("All segments processed. Exiting...");
                            show_donation_prompt();
                            std::process::exit(0);
                        }
                    } else {
                        // No segments available at all, quit the application
                        info!("No segments found. Exiting...");
                        show_donation_prompt();
                        std::process::exit(0);
                    }
                }
            }
        })?;

        // This code will only run if the event loop exits
        self.hotkey_manager.lock().unwrap().unregister(hotkey)?;
        info!("Unregistered hotkey");

        Ok(())
    }
}

fn load_config(args: &Args) -> Result<Config> {
    let mut config = if let Some(config_path) = &args.config {
        let config_content = fs::read_to_string(config_path)
            .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;
        toml::from_str(&config_content)
            .with_context(|| format!("Failed to parse config file: {}", config_path.display()))?
    } else {
        Config::default()
    };

    // Override with command line arguments
    config.file_path = args.file.clone();
    if args.delimiter != "%%%" {
        config.delimiter = args.delimiter.clone();
    }
    config.paste = Some(!args.no_paste);

    Ok(config)
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    // Configure logging level based on verbose flag
    let log_level = if args.verbose {
        tracing::Level::INFO
    } else {
        tracing::Level::WARN
    };
    
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .init();

    let config = load_config(&args)?;

    if !config.file_path.exists() {
        return Err(anyhow::anyhow!(
            "Input file does not exist: {}",
            config.file_path.display()
        ));
    }

    let app = EasypasteApp::new(config.clone())?;

    app.run(config)
}