# Anki Streak Fixer

[Introduction](#introduction) | [Features](#features) | [Requirements](#requirements) | [Installation](#installation) | [Usage](#usage) | [Simulate Mode](#simulate-mode) | [Configuration](#configuration) | [Contributing](#contributing) | [License](#license)

![Anki Streak Fixer Logo](./img/logo.png)

## Introduction
Anki Streak Fixer is a Rust-based utility to manage and modify streak data in your Anki decks. It allows users to process notes in a specified deck and collection, simulate changes, and ensure consistent data integrity.

This application is particularly useful for advanced Anki users who need to manipulate review logs programmatically.

## Features
- Case-insensitive deck name matching.
- Accurate manipulation of Anki's review logs.
- Cross-platform support for macOS, Windows, and Linux.
- Simulate mode to preview changes without applying them.
- Command-line interface for efficient operation.

## Requirements
- Rust (latest stable version)
- Cargo (for building and running the project)
- SQLite3
- An existing Anki installation

## Installation
To install Anki Streak Fixer, follow these steps:

1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/anki-streak-fixer.git
   cd anki-streak-fixer
   ```

2. Build the application using Cargo:
   ```bash
   cargo build --release
   ```

3. The compiled binary can be found in the `target/release` directory.

## Usage
Run the application with the required arguments:

```bash
cargo run -- "<DECK_NAME>" -c "<COLLECTION_NAME>" [-s]
```

### Positional Arguments
- `<DECK_NAME>`: The name of the Anki deck to process.

### Options
- `-c`, `--collection <COLLECTION>`: The name of the Anki collection.
- `-s`, `--simulate`: Enable simulation mode to preview changes without modifying the database.

### Example
Simulate changes for the deck "Словарный запас" in the collection "Alan - Russian":

```bash
cargo run -- "Словарный запас" -c "Alan - Russian" -s
```

## Simulate Mode
In simulate mode, Anki Streak Fixer:
- Prints the actions it would take, including which notes would be modified.
- Does not modify the database, making it safe for testing.

Simulation mode is recommended when testing changes to ensure accuracy.

## Configuration
The application automatically detects the OS and locates your Anki collection database in the following locations:
- **macOS**: `~/Library/Application Support/Anki2/`
- **Windows**: `C:\Users\%USERNAME%\AppData\Roaming\Anki2\`
- **Linux**: `~/.local/share/Anki2/`

Ensure your collection name matches the folder name within this directory.

## Contributing
Contributions are welcome! To contribute:
1. Fork the repository.
2. Create a feature branch.
3. Submit a pull request with a detailed description of your changes.

For major changes, please open an issue first to discuss what you would like to contribute.

## License
This project is licensed under the MIT License. See the `LICENSE` file for details.


