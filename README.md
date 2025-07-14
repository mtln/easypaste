# easypaste

A cross-platform clipboard automation tool written in Rust that allows you to sequentially paste delimited text segments from a file using global hotkeys.

## Why is this useful?
- If you want to do a presentation with live coding or typing, you can use this tool to paste the prepared code or text snippets one by one. No need of error-prone, slow retyping. No need to copy and paste manually.
- If you want to record a tutorial or online course, you achieve a better viewer experience. And you don't waste your time editing and speeding up or or cutting the video after recording to get a good pace if you are a slow or unprecise typer.

Furthermore, the tool is [open-source](https://github.com/mtln/easypaste) and runs offline on your computer, so you can be sure that your data is not being sent to any server.  
And by the way, it’s free! If you find it useful, you can [donate](https://donate.stripe.com/8x28wObdhgoV8aVaQW6J202).


## Features

- **Cross-platform**: Works on macOS and Windows,
- **Global hotkeys**: Configurable hotkey combinations (default: Ctrl+Shift+B)
- **Configurable delimiters**: Use any character or string as a delimiter (default: `%%%`)
- **Single file load**: Loads the input file once at startup
- **Flexible configuration**: Command-line arguments and configuration file support
- **Sequential pasting**: Automatically moves to the next segment after each paste
- **Segment preview**: Shows preview of the next segment before pasting
- **Internal notes**: Support for inline notes after delimiters that are displayed but not pasted
- **Optional pasting**: Can disable automatic pasting to only load segments to clipboard. This is useful if you want to paste the segments manually or if you don't want to grant additional system privileges for the tool to work (Mac).
- **Small binary size**: The tool is less than 2MB.


## Installation

## Binary Release Downloads
* Windows: [easypaste.exe](https://github.com/mtln/easypaste/releases/latest/download/easypaste.exe)
* Mac: [ceasypaste-installer.pkg](https://github.com/mtln/easypaste/releases/latest/download/easypaste-installer.pkg)

### Prerequisites for building your own binary from source

Make sure you have Rust installed. If not, install it from [rustup.rs](https://rustup.rs/).

```bash
cargo build --release
```

## Usage

### Basic Usage

```bash
# Run with a text file using default delimiter (%%%)
easypaste --file example_input.txt

# Use a custom delimiter
easypaste --file mytext.txt --delimiter "ç"

# Disable automatic pasting (only load to clipboard)
easypaste --file mytext.txt --no-paste

# Use a configuration file
easypaste --config config.toml

```

### Command Line Arguments

- `--file, -f <FILE>`: Path to the text file containing delimited content (required)
- `--delimiter, -d <DELIMITER>`: Delimiter character/string (default: "%%%")
- `--config, -c <CONFIG>`: Path to configuration file (optional)
- `--no-paste`: Disable automatic pasting of clipboard contents after loading segment

### Configuration File

Create a `config.toml` file to customize behavior:

```toml
# The delimiter character or string
delimiter = "%%%"

# Path to the input file
file_path = "example_input.txt"

# Hotkey modifiers (CMD/WIN/META, CTRL/CONTROL, ALT/OPTION, SHIFT)
hotkey_modifiers = ["CTRL", "SHIFT"]  # Use ["CTRL", "SHIFT"] on Windows/Linux

# The trigger key (A-Z, 0-9, SPACE, ENTER/RETURN)
hotkey_key = "B"

# Enable/disable automatic pasting (optional, defaults to true)
paste = true
```

### Input File Format

Create a text file with segments separated by your chosen delimiter. You can also include internal notes after delimiters on the same line:

```
First text segment with line break
%%%this is an internal note
Second segment with
multiple lines%%%
function example() {
    console.log("Code snippet");
}%%%this is another internal note
echo "Command example"%%%
Last segment
```

Internal notes (text after delimiter on the same line) are displayed in the console but not included in the pasted content.

## How It Works

1. **Load**: The tool loads your input file and shows a preview of the first segment
2. **Trigger**: Press your configured hotkey (default: Ctrl+Shift+B) to activate the next segment
3. **Copy & Paste**: The segment is copied to clipboard and automatically pasted (unless `--no-paste` is used)
4. **Preview**: A preview of the next segment is shown in the console
5. **Repeat**: Continue pressing the hotkey to move through all segments
6. **Complete**: The tool automatically exits when all segments are processed

## Supported Operating Systems

* **macOS**: Full support with 100ms paste delay
* **Windows**: Full support with 2 second paste delay for reliability

## Limitations

- **Windows delay**: There is a 2 second delay when pasting on Windows. Without this delay, pasting doesn't seem to work reliably. In manual paste mode (--no-paste), there is no delay.

## Disclaimer

easypaste comes with no warranty. If you need to grant additional system privileges for the tool to work, revoke them again if you don't use the tool.
