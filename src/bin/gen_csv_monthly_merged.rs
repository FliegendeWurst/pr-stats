use chrono::{Months, NaiveDateTime, TimeDelta};
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

	let rows = rows.chunk_by(|x| x.3[0..4].to_owned());
	let rows_grouped_by_month = rows.into_iter().collect::<Vec<_>>();

	println!("timestamp,time_to_merge");

	for (year_month, group) in rows_grouped_by_month {
		let group = group.collect::<Vec<_>>();
		let timestamp =
			chrono::NaiveDateTime::parse_from_str(&format!("{year_month}-01-01 00:00:00"), TIME_FORMAT).unwrap();
		let unix = timestamp.signed_duration_since(NaiveDateTime::UNIX_EPOCH).num_seconds();
		let mut timestamp_end = timestamp.clone();
		timestamp_end = timestamp_end.checked_add_months(Months::new(12)).unwrap();
		let unix_end = timestamp_end
			.signed_duration_since(NaiveDateTime::UNIX_EPOCH)
			.num_seconds()
			- 1;
		let unix_delta = unix_end - unix;

		let mut merged_times = group
			.iter()
			.filter(|x| x.5 == 1)
			.map(|x| {
				let opened = chrono::NaiveDateTime::parse_from_str(&x.3, TIME_FORMAT).unwrap();
				let closed = chrono::NaiveDateTime::parse_from_str(x.4.as_ref().unwrap(), TIME_FORMAT).unwrap();
				closed - opened
			})
			//.map(|x| {
			//    x.min(TimeDelta::days(60))
			//})
			.collect::<Vec<_>>();
		merged_times.sort_unstable();
		merged_times.reverse();

		let total = merged_times.len();
		for (i, merge_time) in merged_times.into_iter().enumerate() {
			let time = timestamp + TimeDelta::seconds((unix_delta as usize * i / total) as i64);
			let this_unix = time.signed_duration_since(NaiveDateTime::UNIX_EPOCH).num_seconds();
			let days = merge_time.num_days();
			println!("{this_unix},{days}");
		}
	}
}
