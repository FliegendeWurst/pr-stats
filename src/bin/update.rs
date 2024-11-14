use std::{env, error::Error};

use octocrab::params::{pulls::Sort, Direction};
use pr_stats::{get_database, get_repo, TIME_FORMAT};
use rusqlite::params;

#[tokio::main]
async fn main() {
	let args = env::args().collect::<Vec<_>>();
	real_main(&args).await.unwrap();
}

async fn real_main(args: &[String]) -> Result<(), Box<dyn Error>> {
	let gh = octocrab::OctocrabBuilder::default()
		.personal_token(env::var("GITHUB_PAT").expect("no GITHUB_PAT configured"))
		.build()?;

	let mut database = get_database();
	let tx = database.transaction()?;

	let owner = &args[1];
	let repo = &args[2];

	// get repo
	let repo_id = get_repo(&tx, owner, repo);

	// find last update
	let last_update = tx.query_row("SELECT MAX(created,CASE WHEN closed IS NULL THEN created ELSE closed END) AS t FROM pulls WHERE repo_id = ?1 ORDER BY t DESC LIMIT 1", params![repo_id], |row| {
		Ok(row.get::<_, String>(0)?)
	}).map(Some).unwrap_or(None);
	println!("last update: {last_update:?}");

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
			let updated_at = pr.updated_at.map(|x| x.format(TIME_FORMAT).to_string());

			if updated_at
				.as_ref()
				.map(|x| last_update.as_ref().map(|y| *x < *y).unwrap_or(false))
				.unwrap_or(false)
			{
				println!("done: PR was updated {updated_at:?}");
				break 'pages; // we are done here!
			}

			let merged = pr.merged_at.is_some();
			println!("processed {}/{}#{}", owner, repo, id);
			let res = tx.execute(
				"INSERT INTO pulls (repo_id,id,created,closed,merged) VALUES (?1,?2,?3,?4,?5) ON CONFLICT DO UPDATE SET closed = ?4, merged = ?5",
				params![repo_id, id, created_at, closed_at, merged],
			);
			if let Err(err) = res {
				println!("error: {:?}", err);
			}
		}
	}
	tx.commit()?;
	Ok(())
}
