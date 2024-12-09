use std::env;

use rusqlite::{params, Connection, Transaction};

pub static TIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

pub fn get_database() -> Connection {
	let database = Connection::open(env::var("PR_STATS_DATABASE").unwrap_or_else(|_err| "./prs.db".to_owned()))
		.expect("failed to open database");
	database
		.execute(
			"
		CREATE TABLE IF NOT EXISTS pulls(
            repo_id INTEGER NOT NULL,
			id INTEGER NOT NULL,
            created STRING NOT NULL,
            closed STRING,
            merged INTEGER NOT NULL,
			merger_id INTEGER,
            PRIMARY KEY (repo_id, id)
		)",
			[],
		)
		.unwrap();
	database
		.execute(
			"
		CREATE TABLE IF NOT EXISTS repos(
			id INTEGER PRIMARY KEY,
            owner STRING NOT NULL,
            repo STRING NOT NULL
		)",
			[],
		)
		.unwrap();
	database
		.execute(
			"
		CREATE TABLE IF NOT EXISTS users(
			id INTEGER PRIMARY KEY,
            name TEXT NOT NULL
		) STRICT",
			[],
		)
		.unwrap();
	database
}

pub fn get_repo(db: &Transaction, owner: &str, repo: &str) -> u64 {
	if let Ok(id) = db.query_row(
		"SELECT id FROM repos WHERE owner = ?1 AND repo = ?2",
		params![owner, repo],
		|row| Ok(row.get::<_, u64>(0)?),
	) {
		return id;
	}
	db.execute("INSERT INTO repos (owner, repo) VALUES (?1, ?2)", params![owner, repo])
		.unwrap();
	get_repo(db, owner, repo)
}
