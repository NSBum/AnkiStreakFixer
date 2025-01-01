use rusqlite::{params, Connection, Result};
use chrono::{Local, NaiveDate, NaiveTime, TimeZone};
use clap::{Arg, ArgMatches, Command};
use std::env;
use std::path::PathBuf;

const APP_NAME: &str = env!("CARGO_PKG_NAME");
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

const GREEN: &str = "\x1b[32m";
const RESET: &str = "\x1b[0m";

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

struct AnkiProcessor {
    deck_name: String,
    simulate: bool,
    db_path: PathBuf,
}

impl AnkiProcessor {
    fn new(deck_name: &str, collection_name: &str, simulate: bool) -> Self {
        let collection = AnkiCollection::new(collection_name);
        Self {
            deck_name: deck_name.to_string(),
            simulate,
            db_path: collection.collection_path(),
        }
    }

    fn process(&self) -> Result<()> {
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
        let rid_string = self.generate_rid_string(today, rollover_hours);
        let note_ids = self.fetch_reviewed_notes()?;

        if note_ids.is_empty() {
            println!("No notes found for today in deck '{}'.", self.deck_name);
        } else {
            self.process_notes(note_ids, &rid_string)?;
        }

        Ok(())
    }

    fn get_rollover_hours(&self) -> Result<i64> {
        let query = "SELECT val FROM config WHERE key = 'rollover';";

        let conn = Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare(query)?;

        // Retrieve the value as a BLOB
        let raw_val: Vec<u8> = stmt.query_row([], |row| row.get(0))?;

        // Handle single-byte or multi-byte cases
        if raw_val.len() == 1 {
            // Interpret the single byte as an ASCII digit
            let rollover_char = raw_val[0] as char;
            rollover_char
                .to_digit(10)
                .map(|d| d as i64)
                .ok_or_else(|| rusqlite::Error::ToSqlConversionFailure(Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Invalid single-byte rollover value",
                    ),
                )))
        } else {
            Err(rusqlite::Error::ToSqlConversionFailure(Box::new(
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Unexpected BLOB size for rollover value: {}", raw_val.len()),
                ),
            )))
        }
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





    fn fetch_reviewed_notes(&self) -> Result<Vec<i64>> {
        let query = "
            SELECT DISTINCT notes.id
            FROM cards
            JOIN notes ON cards.nid = notes.id
            JOIN decks ON cards.did = decks.id
            JOIN revlog ON cards.id = revlog.cid
            WHERE decks.name COLLATE NOCASE = ?1
            AND date(revlog.id / 1000, 'unixepoch', 'localtime') = date('now', 'localtime')
            ORDER BY notes.id;
        ";

        let conn = Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare(query)?;

        let notes = stmt
            .query_map(params![self.deck_name], |row| row.get(0))?
            .collect::<Result<Vec<i64>, _>>()?;

        Ok(notes)
    }

    fn process_notes(&self, notes: Vec<i64>, rid_string: &str) -> Result<()> {
        let start_time: i64 = rid_string.split(':').nth(1).unwrap().parse().unwrap();
        let end_time: i64 = rid_string.split(':').nth(2).unwrap().parse().unwrap();

        let update_query = "
            UPDATE revlog
            SET id = id - 86400000
            WHERE id IN (
                SELECT r.id
                FROM revlog r
                INNER JOIN cards c ON r.cid = c.id
                INNER JOIN notes n ON n.id = c.nid
                WHERE n.id = ?1
                AND r.id >= ?2
                AND r.id < ?3
            );
        ";

        let conn = Connection::open(&self.db_path)?;
        for note_id in notes {
            if self.simulate {
                println!("Simulating update for note {} ({} to {}).", note_id, start_time, end_time);
            } else {
                conn.execute(update_query, params![note_id, start_time, end_time])?;
                println!("Note date updated successfully for {}.", note_id);
            }
        }

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
                .required(true)
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
        .get_matches()
}

fn main() -> Result<()> {
    let matches = get_clap_matches();

    let deck_name = matches.get_one::<String>("deck_name").unwrap().as_str();
    let collection_name = matches.get_one::<String>("collection").unwrap().as_str();
    let simulate = matches.get_flag("simulate");

    let processor = AnkiProcessor::new(deck_name, collection_name, simulate);
    processor.process()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_generate_rid_string() {
        let processor = AnkiProcessor::new("test_deck", "test_collection", true);
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
