use std::{env, error::Error};

use octocrab::params::{pulls::Sort, Direction};
use pr_stats::{get_database, TIME_FORMAT};
use rusqlite::params;

#[tokio::main]
async fn main() {
	real_main().await.unwrap();
}

async fn real_main() -> Result<(), Box<dyn Error>> {
	let gh = octocrab::OctocrabBuilder::default()
		.personal_token(env::var("GITHUB_PAT").expect("no GITHUB_PAT configured"))
		.build()?;

	let mut database = get_database();
	let tx = database.transaction()?;

	// find last update
	let last_update = tx.query_row("SELECT MAX(created,CASE WHEN closed IS NULL THEN created ELSE closed END) AS t FROM pulls ORDER BY t DESC LIMIT 1", params![], |row| {
		Ok(row.get::<_, String>(0)?)
	}).unwrap();
	println!("last update: {last_update}");

	let owner = "NixOS";
	let repo = "nixpkgs";
	'pages: for page in 1u32.. {
		let prs = gh
			.pulls(owner, repo)
			.list()
			.sort(Sort::Updated)
			.direction(Direction::Descending)
			.state(octocrab::params::State::All)
			.per_page(100)
			.page(page)
			.send()
			.await?;
		if prs.items.is_empty() {
			break;
		}
		for pr in prs {
			let id = pr.number;
			let created_at = pr.created_at.unwrap().format(TIME_FORMAT).to_string();
			let closed_at = pr.closed_at.map(|x| x.format(TIME_FORMAT).to_string());

			if created_at < last_update && closed_at.as_ref().map(|x| *x < last_update).unwrap_or(false) && page > 10 {
				println!("done: PR was opened {created_at} / closed {closed_at:?}");
				break 'pages; // we are done here!
			}

			let merged = pr.merged_at.is_some();
			println!("processed {}/{}#{}", owner, repo, id);
			let res = tx.execute(
				"INSERT INTO pulls (owner,repo,id,created,closed,merged) VALUES (?1,?2,?3,?4,?5,?6) ON CONFLICT DO UPDATE SET closed = ?5, merged = ?6",
				params![owner, repo, id, created_at, closed_at, merged],
			);
			if let Err(err) = res {
				println!("error: {:?}", err);
			}
		}
	}
	tx.commit()?;
	Ok(())
}
