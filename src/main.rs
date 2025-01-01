use rusqlite::{params, Connection, Result};
use chrono::{Local, NaiveDate};
use clap::{Arg, ArgMatches, Command};
use std::env;
use std::path::PathBuf;

const APP_NAME: &str = env!("CARGO_PKG_NAME");
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

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
            println!("Running {} v{} - Simulation Mode", APP_NAME, APP_VERSION);
        } else {
            println!("Running {} v{}", APP_NAME, APP_VERSION);
        }

        let today = Local::now().date_naive();
        let rid_string = self.generate_rid_string(today);
        let note_ids = self.fetch_reviewed_notes()?;

        if note_ids.is_empty() {
            println!("No notes found for today in deck '{}'.", self.deck_name);
        } else {
            self.process_notes(note_ids, &rid_string)?;
        }

        Ok(())
    }

    fn generate_rid_string(&self, date: NaiveDate) -> String {
        let local_midnight = date.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp_millis();
        let next_midnight = local_midnight + 86_400_000; // Add 24 hours in milliseconds

        format!("rid:{}:{}", local_midnight, next_midnight)
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
        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let rid_string = processor.generate_rid_string(date);

        assert!(rid_string.starts_with("rid:"));
        let parts: Vec<&str> = rid_string.split(':').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[1], "1704067200000"); // Expected timestamp for midnight 2024-01-01
        assert_eq!(parts[2], "1704153600000"); // Expected timestamp for midnight 2024-01-02
    }

    #[test]
    fn test_collection_path() {
        let collection = AnkiCollection::new("test_collection");
        let path = collection.collection_path();

        assert!(path.to_str().unwrap().contains("test_collection"));
        assert!(path.to_str().unwrap().ends_with("collection.anki2"));
    }
}
