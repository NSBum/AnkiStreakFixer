mod utils;
mod date;

use rusqlite::{params, Connection, Result};
use chrono::{Local, NaiveDate, NaiveTime, TimeZone};
use clap::{Arg, ArgMatches, Command};
use std::env;
use unicase::UniCase;
use std::path::PathBuf;
use date::{parse_date, validate_dates};
use utils::{log, replace_deck_delimiter};

const APP_NAME: &str = env!("CARGO_PKG_NAME");
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

const GREEN: &str = "\x1b[32m";
const RESET: &str = "\x1b[0m";

/// Registers a custom collation named `unicase` to enable Unicode-aware case-insensitive comparisons
/// in SQLite.
///
/// # Why is this needed?
/// SQLite's default `NOCASE` collation only supports ASCII case-insensitivity. This means that
/// comparisons like "Ä" vs. "ä" or "ß" vs. "ss" will not work as expected. For applications dealing
/// with Unicode data, such as deck names in Anki that might include international characters,
/// this limitation can result in inaccurate or incomplete query results.
///
/// To address this, we define a `unicase` collation that uses Rust's `unicase` crate to perform
/// Unicode-aware case-insensitive comparisons.
///
/// # How does it work?
/// - SQLite provides a mechanism for defining custom collations via the `create_collation` method
///   in the `rusqlite` crate.
/// - The `unicase` crate simplifies case-insensitive comparisons by normalizing strings before
///   comparing them.
/// - When this collation is applied in SQLite queries, it ensures that the string comparison respects
///   Unicode case-folding rules.
///
/// # Arguments
/// - `conn`: A reference to the SQLite `Connection` object where the collation should be registered.
///
/// # Returns
/// - `Ok(())` if the collation is registered successfully.
/// - An error if SQLite fails to register the collation.
///
/// # Usage
/// This function is typically called during database initialization, after opening a connection.
/// The `unicase` collation can then be used in queries:
///
/// ```sql
/// SELECT name
/// FROM decks
/// WHERE name COLLATE unicase LIKE '%example%'
/// ORDER BY name COLLATE unicase;
/// ```
///
/// # Example
/// ```rust
/// let conn = Connection::open("example.db")?;
/// register_unicase_collation(&conn)?;
/// ```
fn register_unicase_collation(conn: &Connection) -> Result<()> {
    conn.create_collation("unicase", |s1: &str, s2: &str| {
        let s1_key = UniCase::new(s1);
        let s2_key = UniCase::new(s2);
        s1_key.cmp(&s2_key)
    })?;
    Ok(())
}

/// Opens a SQLite database and registers the `unicase` collation for Unicode case-insensitivity.
///
/// # Why is this needed?
/// When working with SQLite databases that contain non-ASCII text, such as Unicode deck names in Anki,
/// the default case-insensitive collation (`NOCASE`) is insufficient because it does not handle Unicode
/// characters correctly. Without registering a custom collation, queries involving case-insensitive
/// matches may fail or produce incomplete results.
///
/// This function abstracts the process of opening a SQLite connection and ensuring that the custom
/// `unicase` collation is available for all queries that require Unicode case-insensitivity.
///
/// # How does it work?
/// - This function opens a SQLite database using the given file path.
/// - After opening the connection, it registers the `unicase` collation by calling `register_unicase_collation`.
/// - This ensures that any subsequent queries can use the `unicase` collation.
///
/// # Arguments
/// - `db_path`: The file path to the SQLite database. This should be a valid path to an existing database file.
///
/// # Returns
/// - `Ok(Connection)` if the database is opened and the collation is registered successfully.
/// - An error if the database cannot be opened or the collation fails to register.
///
/// # Usage
/// Use this function instead of directly calling `Connection::open` to ensure that the custom collation is
/// registered automatically.
///
/// # Example
/// ```rust
/// let conn = open_database_with_collation("example.db")?;
/// let query = "SELECT name FROM decks WHERE name COLLATE unicase LIKE '%example%' ORDER BY name COLLATE unicase;";
/// let mut stmt = conn.prepare(query)?;
/// ```
fn open_database_with_collation(db_path: &str) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    register_unicase_collation(&conn)?;
    Ok(conn)
}

enum AppMode {
    Deck(String), // Contains the deck name
    All,          // All decks
}

struct AppConfig {
    verbose: bool,
    mode: AppMode
}

#[derive(Debug)]
struct AnkiCollection {
    collection_name: String,
}

impl AnkiCollection {
    fn new(collection_name: &str) -> Self {
        Self {
            collection_name: collection_name.to_string(),
        }
    }

    fn collection_path(&self) -> PathBuf {
        let base_path = match env::consts::OS {
            "macos" => "~/Library/Application Support/Anki2/",
            "windows" => "C:\\Users\\%USERNAME%\\AppData\\Roaming\\Anki2\\",
            "linux" => "~/.local/share/Anki2/",
            _ => panic!("Unsupported OS"),
        };

        let expanded_base = shellexpand::tilde(base_path);
        PathBuf::from(expanded_base.to_string()).join(&self.collection_name).join("collection.anki2")
    }
}

struct AnkiProcessor<'a> {
    simulate: bool,
    db_path: PathBuf,
    limit: i64,
    from_date: Option<NaiveDate>,
    to_date: Option<NaiveDate>,
    config: &'a AppConfig,
}

impl<'a> AnkiProcessor<'a> {
    fn new(
        collection_name: &str,
        simulate: bool,
        limit: i64,
        from_date: Option<NaiveDate>,
        to_date: Option<NaiveDate>,
        config: &'a AppConfig,
    ) -> Self {
        let collection = AnkiCollection::new(collection_name);
        Self {
            //deck_name: deck_name.to_string(),
            simulate,
            db_path: collection.collection_path(),
            limit,
            from_date,
            to_date,
            config,
        }
    }

    fn process(&self) -> Result<()> {
        log(self.config.verbose, "Starting processing...");
        if self.simulate {
            println!(
                "Running {} v{} - {}Simulation mode{}",
                APP_NAME, APP_VERSION, GREEN, RESET
            );
        } else {
            println!("Running {} v{}", APP_NAME, APP_VERSION);
        }


        let rollover_hours = self.get_rollover_hours()?;
        let today = Local::now().date_naive();

        // Use from_date if provided, otherwise use today
        let base_date = self.from_date.unwrap_or_else(|| today);
        let rid_string = self.generate_rid_string(base_date, rollover_hours);

        let note_ids = self.fetch_reviewed_notes()?;

        if note_ids.is_empty() {
            let msg = match &self.config.mode {
                AppMode::All => format!("No notes found in any deck for {}", base_date),
                AppMode::Deck(deck_name) => format!(
                    "No notes found in the deck '{}' for {}",
                    deck_name, base_date
                ),
            };

            println!("{}", msg);
        } else {
            self.process_notes(note_ids, &rid_string)?;
        }

        log(self.config.verbose, "Processing completed.");
        Ok(())
    }

    fn get_rollover_hours(&self) -> Result<i64> {
        log(self.config.verbose, "Querying rollover hours.");
        let query = "SELECT val FROM config WHERE key = 'rollover';";

        let conn = Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare(query)?;

        // Retrieve the value as a BLOB
        let raw_val: Vec<u8> = stmt.query_row([], |row| row.get(0))?;

        // Interpret the BLOB as a UTF-8 encoded string of digits
        let rollover_str = String::from_utf8(raw_val)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        log(self.config.verbose, &format!("Rollover string: {}", rollover_str));

        // Parse the string as an integer
        rollover_str
            .parse::<i64>()
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Failed to parse rollover value: {}", e),
                ),
            )))
    }


    fn generate_rid_string(&self, date: NaiveDate, rollover_hours: i64) -> String {
        let rollover_time = NaiveTime::from_hms_opt(rollover_hours as u32, 0, 0)
            .expect("Invalid rollover hour");

        // Combine the date and rollover time
        let naive_rollover_datetime = date.and_time(rollover_time);

        // Convert to local timezone using the system's timezone offset
        let local_rollover_datetime = chrono::Local
            .from_local_datetime(&naive_rollover_datetime)
            .single()
            .expect("Ambiguous or invalid local datetime");

        // Calculate start and end times
        let start_time = local_rollover_datetime.timestamp_millis();
        let end_time = start_time + 86_400_000; // Add 24 hours in milliseconds

        format!("rid:{}:{}", start_time, end_time)
    }

    /// Fetches matching deck names where the name contains the provided deck name.
    /// Ensures that the parent deck is processed if it matches or has children.
    fn fetch_matching_decks(&self) -> Result<Vec<String>> {
        // Ensure this is only called in AppMode::Deck
        let deck_name = match &self.config.mode {
            AppMode::Deck(name) => name,
            AppMode::All => {
                return Err(rusqlite::Error::InvalidQuery); // Protect against misuse
            }
        };

        log(
            self.config.verbose,
            &format!("Fetching matching deck names for '{}'", deck_name),
        );

        // SQL query to fetch decks that match or are children of the provided name
        let query = "
        SELECT name
        FROM decks
        WHERE name COLLATE unicase = ?1
        OR name COLLATE unicase LIKE ?2 || '::%'
        ORDER BY name COLLATE unicase;
    ";

        // Open the database and register the `unicase` collation
        let conn = open_database_with_collation(self.db_path.to_str().unwrap())?;
        let mut stmt = conn.prepare(query)?;

        let matching_decks = stmt
            .query_map(
                params![deck_name, deck_name],
                |row| row.get::<_, String>(0),
            )?
            .collect::<Result<Vec<String>, _>>()?;

        if matching_decks.is_empty() {
            log(
                self.config.verbose,
                &format!("No decks found matching or under '{}'", deck_name),
            );
            return Err(rusqlite::Error::InvalidQuery);
        }

        log(
            self.config.verbose,
            &match matching_decks.len() {
                1 => format!("Single matching deck found: '{}'", matching_decks[0]),
                _ => format!(
                    "Parent deck '{}' contains the following child decks:\n{}",
                    deck_name,
                    matching_decks
                        .iter()
                        .map(|d| replace_deck_delimiter(d))
                        .collect::<Vec<_>>()
                        .join("\n")
                ),
            },
        );

        Ok(matching_decks)
    }

    fn fetch_reviewed_notes(&self) -> Result<Vec<i64>> {
        log(self.config.verbose, "Fetching reviewed notes...");

        let conn = open_database_with_collation(self.db_path.to_str().unwrap())?;

        // Ensure we have a valid `from_date` to work with
        let from_date = match self.from_date {
            Some(date) => date,
            None => {
                return Err(rusqlite::Error::InvalidQuery); // `--from` date is required
            }
        };

        log(
            self.config.verbose,
            &format!("Fetching notes reviewed on: {}", from_date),
        );

        // Convert `from_date` to a timestamp range
        let from_timestamp_start = from_date
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp();
        let from_timestamp_end = from_timestamp_start + 86_400; // Add 24 hours to get the next day

        // Query logic based on mode
        let query = match &self.config.mode {
            AppMode::All => {
                log(self.config.verbose, "Mode: All decks");
                // Return a query that doesn't limit by deck
                "
            SELECT DISTINCT notes.id
            FROM cards
            JOIN notes ON cards.nid = notes.id
            JOIN revlog ON cards.id = revlog.cid
            WHERE revlog.id / 1000 BETWEEN ?1 AND ?2
            ORDER BY notes.id;
            "
            }
            AppMode::Deck(_) => {
                // Fetch the parent deck and its hierarchy
                let matching_decks = self.fetch_matching_decks()?;
                let parent_deck = &matching_decks[0]; // Assume first is parent

                log(
                    self.config.verbose,
                    &format!(
                        "Processing parent deck '{}'{}",
                        parent_deck,
                        if matching_decks.len() > 1 {
                            format!(
                                " with children:\n{}",
                                matching_decks[1..]
                                    .iter()
                                    .map(|d| replace_deck_delimiter(d))
                                    .collect::<Vec<_>>()
                                    .join("\n")
                            )
                        } else {
                            "".to_string()
                        }
                    ),
                );

                "
            SELECT DISTINCT notes.id
            FROM cards
            JOIN notes ON cards.nid = notes.id
            JOIN decks ON cards.did = decks.id
            JOIN revlog ON cards.id = revlog.cid
            WHERE decks.name COLLATE unicase = ?3
            AND revlog.id / 1000 BETWEEN ?1 AND ?2
            ORDER BY notes.id;
            "
            }
        };

        // Prepare and execute the query
        let mut stmt = conn.prepare(query)?;

        let notes = match &self.config.mode {
            AppMode::All => stmt
                .query_map(params![from_timestamp_start, from_timestamp_end], |row| row.get(0))?
                .collect::<Result<Vec<i64>, _>>()?,
            AppMode::Deck(_) => {
                let matching_decks = self.fetch_matching_decks()?;
                let parent_deck = &matching_decks[0]; // Use parent deck
                stmt.query_map(
                    params![from_timestamp_start, from_timestamp_end, parent_deck],
                    |row| row.get(0),
                )?
                    .collect::<Result<Vec<i64>, _>>()?
            }
        };

        // Apply limit if specified
        let limited_notes = if self.limit > 0 {
            notes.into_iter().take(self.limit as usize).collect()
        } else {
            notes
        };

        Ok(limited_notes)
    }

    fn process_notes(&self, notes: Vec<i64>, rid_string: &str) -> Result<()> {
        log(
            self.config.verbose,
            &format!("Processing {} notes...", notes.len()),
        );

        let start_time: i64 = rid_string.split(':').nth(1).unwrap().parse().unwrap();
        let end_time: i64 = rid_string.split(':').nth(2).unwrap().parse().unwrap();

        // Calculate the actual ID offset using your utility functions
        let id_offset = if let (Some(from), Some(to)) = (self.from_date, self.to_date) {
            let days_difference = date::days_between(to, from);
            date::calculate_id_offset(days_difference)
        } else {
            date::calculate_id_offset(1) // Default 1-day offset if dates are not provided
        };

        let conn = Connection::open(&self.db_path)?;

        // Prepare queries
        let update_revlog_query = "
        UPDATE revlog
        SET id = id - ?
        WHERE id IN (
            SELECT r.id
            FROM revlog r
            INNER JOIN cards c ON r.cid = c.id
            INNER JOIN notes n ON n.id = c.nid
            WHERE n.id = ?
            AND r.id >= ?
            AND r.id < ?
        )
        RETURNING cid;
    ";

        let update_cards_query = "
            UPDATE cards
            SET mod = ?, usn = -1
            WHERE id = ?;
        ";

        let mut affected_cards = Vec::new();
        let current_time = chrono::Utc::now().timestamp();

        for note_id in &notes {
            let mut stmt = conn.prepare(update_revlog_query)?;

            // Collect affected card IDs for the current note
            let note_cards = stmt
                .query_map(params![id_offset, note_id, start_time, end_time], |row| {
                    row.get::<_, i64>(0) // Extract the card ID
                })?
                .collect::<Result<Vec<i64>, _>>()?;

            // Clone note_cards before extending
            affected_cards.extend(note_cards.clone());

            if self.simulate {
                println!(
                    "Simulating update for note {} (from {} to {}), moving back {} days.",
                    note_id,
                    start_time,
                    end_time,
                    id_offset / 86_400_000 // Convert offset back to days for display
                );
            } else {
                // Update the cards table for affected cards
                for cid in &note_cards {
                    conn.execute(update_cards_query, params![current_time, cid])?;
                }
                println!("Note date updated successfully for {}.", note_id);

                log(self.config.verbose, "Will trigger full database sync criterion.");
                let force_sync_query = "
                    UPDATE col SET scm = scm + 1;
                ";
                conn.execute(force_sync_query, [])?;
            }
        }

        log(
            self.config.verbose,
            &format!("Marked {} cards as needing sync.", affected_cards.len()),
        );

        Ok(())
    }

}

fn get_clap_matches() -> ArgMatches {
    Command::new(APP_NAME)
        .version(APP_VERSION)
        .about("Processes Anki notes based on deck and collection.")
        .arg(
            Arg::new("deck_name")
                .help("Name of the deck to process.")
                //.required(true)
                .index(1),
        )
        .arg(
            Arg::new("collection")
                .help("Name of the Anki collection.")
                .short('c')
                .long("collection")
                .value_name("COLLECTION"),
        )
        .arg(
            Arg::new("simulate")
                .help("Simulate the changes without applying them.")
                .short('s')
                .long("simulate")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("limit")
                .help("Limit the number of cards moved to previous day.")
                .short('l')
                .long("limit")
                .value_name("LIMIT"),
        )
        .arg(
            Arg::new("from")
                .help("Start date (format: YYYY-MM-DD or YYYYMMDD)")
                .long("from")
                .value_name("FROM_DATE")
                .value_parser(|s: &str| parse_date(s)),
        )
        .arg(
            Arg::new("to")
                .help("End date (format: YYYY-MM-DD or YYYYMMDD)")
                .long("to")
                .value_name("TO_DATE")
                .value_parser(|s: &str| parse_date(s)),
        )
        .arg(
            Arg::new("verbose")
                .help("Emit verbose logging")
                .short('v')
                .long("verbose")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches()
}

fn main() -> Result<()> {
    let matches = get_clap_matches();

    // Optional deck name
    let deck_name = matches.get_one::<String>("deck_name").map(|s| s.as_str());
    // Required collection name
    let collection_name = matches.get_one::<String>("collection").unwrap().as_str();

    let simulate = matches.get_flag("simulate");

    let verbose = matches.get_flag("verbose");

    // Set mode based on deck name presence
    let mode = match deck_name {
        Some(name) => AppMode::Deck(name.to_string()),
        None => AppMode::All,
    };

    // Create global config
    let config = AppConfig { verbose, mode };

    log(config.verbose, "Application started.");

    // Allow user to optionally limit the number of cards moved to previous day
    let limit: i64 = matches.get_one::<String>("limit").unwrap_or(&"0".to_string()).parse().unwrap_or(0);

    // User may have specified from/to dates
    let from_date: Option<NaiveDate> = matches.get_one("from").copied();
    let to_date: Option<NaiveDate> = matches.get_one("to").copied();
    // Check that either both dates are provided or neither is provided
    match (from_date, to_date) {
        (Some(_), None) => {
            eprintln!("Error: If --from is specified, --to must also be specified");
            std::process::exit(1);
        },
        (None, Some(_)) => {
            eprintln!("Error: If --to is specified, --from must also be specified");
            std::process::exit(1);
        },
        _ => () // Both Some or both None is fine
    }

    let today = chrono::Local::now().date_naive(); // Use current date
    if let Err(err) = validate_dates(from_date, to_date, today) {
        eprintln!("\x1b[31m[ERROR]\x1b[0m {}", err); // Print the error in red
        std::process::exit(1); // Exit with an error code
    }

    let processor = AnkiProcessor::new(
        collection_name,
        simulate,
        limit,
        from_date,
        to_date,
        &config
    );
    processor.process()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_generate_rid_string() {
        let config = AppConfig{verbose:true, mode:AppMode::All};
        let processor = AnkiProcessor::new("test_collection", true, 1, None, None, &config);
        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let rid_string = processor.generate_rid_string(date, 1);

        assert!(rid_string.starts_with("rid:"));
        let parts: Vec<&str> = rid_string.split(':').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[1], "1735711200000"); // Expected timestamp for 2025-01-01 01:00:00 local time
        assert_eq!(parts[2], "1735797600000");  // Expected timestamp for 2025-01-02 01:00:00 local

        let date2 = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let rid_string2 = processor.generate_rid_string(date2, 1);
        assert_eq!(rid_string2, "rid:1735711200000:1735797600000");
    }

    #[test]
    fn test_collection_path() {
        let collection = AnkiCollection::new("test_collection");
        let path = collection.collection_path();

        assert!(path.to_str().unwrap().contains("test_collection"));
        assert!(path.to_str().unwrap().ends_with("collection.anki2"));
    }
}
