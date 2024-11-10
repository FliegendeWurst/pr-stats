use chrono::NaiveDateTime;
use pr_stats::{get_database, TIME_FORMAT};
use rusqlite::params;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Action {
    Open,
    Close,
    Merge
}
use Action::*;

fn main() {
	let db = get_database();

	let mut stmt = db
		.prepare("SELECT * FROM pulls WHERE owner = 'NixOS' AND repo = 'nixpkgs'")
		.unwrap();

	let rows = stmt.query_map(params![], |row| {
		Ok((
			row.get::<_, String>(0).unwrap(),
			row.get::<_, String>(1).unwrap(),
			row.get::<_, u64>(2).unwrap(),
			row.get::<_, String>(3).unwrap(),
			row.get::<_, Option<String>>(4).unwrap(),
			row.get::<_, u64>(5).unwrap(),
		))
	}).unwrap().map(Result::unwrap);

    println!("timestamp,open,merged,closed");
    let mut open = 0;
    let mut closed = 0;
    let mut merged = 0;

    let mut events = rows.map(|x| {
        let mut e = vec![];
        e.push((x.3, Open));
        if x.5 == 1 {
            e.push((x.4.unwrap(), Merge));
        } else if let Some(t) = x.4 {
            e.push((t, Close));
        }
        e
    }).flatten().collect::<Vec<_>>();
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
