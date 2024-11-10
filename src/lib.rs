use std::env;

use rusqlite::Connection;

pub static TIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

pub fn get_database() -> Connection {
	let database = Connection::open(env::var("PR_STATS_DATABASE").expect("no PR_STATS_DATABASE configured"))
		.expect("failed to open database");
	database
		.execute(
			"
		CREATE TABLE IF NOT EXISTS pulls(
            owner STRING NOT NULL,
            repo STRING NOT NULL,
			id INTEGER NOT NULL,
            created STRING NOT NULL,
            closed STRING,
            merged INTEGER NOT NULL,
            PRIMARY KEY (owner, repo, id)
		)",
			[],
		)
		.unwrap();
	database
}
