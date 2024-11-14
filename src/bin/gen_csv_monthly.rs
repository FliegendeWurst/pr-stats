use chrono::{Months, NaiveDateTime};
use itertools::Itertools;
use pr_stats::{get_database, get_repo, TIME_FORMAT};
use rusqlite::params;

fn main() {
	let mut db = get_database();
	let tx = db.transaction().unwrap();

	let owner = "NixOS";
	let repo = "nixpkgs";

	let repo_id = get_repo(&tx, owner, repo);

	let mut stmt = tx
		.prepare("SELECT * FROM pulls WHERE repo_id = ?1 ORDER BY created ASC")
		.unwrap();

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

	let rows = rows.chunk_by(|x| x.3[0..7].to_owned());
	let rows_grouped_by_month = rows.into_iter().collect::<Vec<_>>();

	println!("timestamp,open,merged,closed");

	for (year_month, group) in rows_grouped_by_month {
		let group = group.collect::<Vec<_>>();
		let mut timestamp =
			chrono::NaiveDateTime::parse_from_str(&format!("{year_month}-01 00:00:00"), TIME_FORMAT).unwrap();
		let unix = timestamp.signed_duration_since(NaiveDateTime::UNIX_EPOCH).num_seconds();
		let open = group.iter().filter(|x| x.4.is_none()).count();
		let merged = group.iter().filter(|x| x.5 == 1).count();
		let closed = group.iter().filter(|x| x.4.is_some() && x.5 == 0).count();
		let total = (open + merged + closed) as f32;
		println!(
			"{},{},{},{}",
			unix,
			open as f32 / total,
			merged as f32 / total,
			closed as f32 / total
		);
		timestamp = timestamp.checked_add_months(Months::new(1)).unwrap();
		let unix = timestamp.signed_duration_since(NaiveDateTime::UNIX_EPOCH).num_seconds() - 1;
		println!(
			"{},{},{},{}",
			unix,
			open as f32 / total,
			merged as f32 / total,
			closed as f32 / total
		);
	}
}
