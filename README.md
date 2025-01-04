# Anki Streak Fixer

[Introduction](#introduction) | [Features](#features) | [Requirements](#requirements) | [Installation](#installation) | [Usage](#usage) | [Simulate Mode](#simulate-mode) | [Configuration](#configuration) | [Contributing](#contributing) | [License](#license)

![Anki Streak Fixer Logo](./img/logo.png)

## Introduction
Anki Streak Fixer is a Rust-based utility to manage and modify streak data in your Anki decks. Essentially, it moves reviews in a particular deck in a specified collection from _today_ to _yesterday_ (or as of version 0.3.0 from any day to any other day.) This fixes the condition that leads to a missed streak. Yes, this is "cheating", but it causes me no moral dilemma. 👹 But if you have strong opinions about such measures, the project may not be for you. 😇

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

You can install from the source. building the application locally on your machine or you can download the binary for your architecture and install it that way. If you choose the latter, you are on your own in terms of dealing with any complaints that your OS has about unsigned binaries from unknown developers. I'm not interested in jumping through whatever hoops are necessary to get around these warnings.

### Install from source (recommended for reasonable technically-adept users)

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

### Install from a prebuilt binary

Go to the **Releases** section of this repository and download the appropriate binary for your architecture.
## Usage
Run the application with the required arguments:

```bash
cargo run -- "<DECK_NAME>" -c "<COLLECTION_NAME>" [-s]
```

### Positional Arguments
- `<DECK_NAME>`: The name of the Anki deck to process.

Since decks may be organized hierarchically, it presents a problem when specifying which deck to manipulate. If you have decks arranged like this, for example:

![Deck hierarchy](/img/deck_hierarchy.png)

then you should choose one of the subdeck names.

If the deck is not specified, then AnkiStreakFixer will attempt to move reviews from _all_ decks in the collection. This is useful if you like to keep track of your streaks in particular decks and subdecks.

### Options
- `-c`, `--collection <COLLECTION>`: The name of the Anki collection.
- `-s`, `--simulate`: Enable simulation mode to preview changes without modifying the database.
- `l`, `--limit`: Limit the number of cards moved
- `v`, `--verbose`: Use verbose logging
- `f`, `--from`: Optionally specify a _from_ date. Reviews will be moved from this date.
- `t`, `--to`: Optionally specify a _to_ date. Reviews will be moved to this date.

The date formats can be e.g. _20250103_ or _2025-01-03_. For the `--from` date, it can also be _today_.

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

If you are not able to contribute code to the project that's fine; you can open an issue wherein you describe what features you would like to see, what bugs you may be encountering etc. Please be as specific as possible and include more details than you think are necessary.

## License
This project is licensed under the MIT License. See the `LICENSE` file for details.


