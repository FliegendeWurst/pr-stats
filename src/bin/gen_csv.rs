use chrono::NaiveDateTime;
use pr_stats::{get_database, get_repo, TIME_FORMAT};
use rusqlite::params;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Action {
	Open,
	Close,
	Merge,
}
use Action::*;

fn main() {
	let mut db = get_database();
	let tx = db.transaction().unwrap();

	let owner = "NixOS";
	let repo = "nixpkgs";

	let repo_id = get_repo(&tx, owner, repo);
	let mut stmt = tx.prepare("SELECT * FROM pulls,repos WHERE repo_id = ?1").unwrap();

	let rows = stmt
		.query_map(params![repo_id], |row| {
			Ok((
				owner,
				repo,
				row.get::<_, u64>(1).unwrap(),
				row.get::<_, String>(2).unwrap(),
				row.get::<_, Option<String>>(3).unwrap(),
				row.get::<_, u64>(4).unwrap(),
			))
		})
		.unwrap()
		.map(Result::unwrap);

	println!("timestamp,open,merged,closed");
	let mut open = 0;
	let mut closed = 0;
	let mut merged = 0;

	let mut events = rows
		.map(|x| {
			let mut e = vec![];
			e.push((x.3, Open));
			if x.5 == 1 {
				e.push((x.4.unwrap(), Merge));
			} else if let Some(t) = x.4 {
				e.push((t, Close));
			}
			e
		})
		.flatten()
		.collect::<Vec<_>>();
	events.sort_unstable();

	for (time, event) in events {
		let timestamp = chrono::NaiveDateTime::parse_from_str(&time, TIME_FORMAT).unwrap();
		let unix = timestamp.signed_duration_since(NaiveDateTime::UNIX_EPOCH).num_seconds();
		match event {
			Open => open += 1,
			Close => {
				closed += 1;
				open -= 1;
			},
			Merge => {
				merged += 1;
				open -= 1;
			},
		}
		println!("{},{},{},{}", unix, open, merged, closed);
	}
}
