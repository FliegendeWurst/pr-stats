use std::{
	collections::HashMap,
	env,
	error::Error,
	io::{self, Write},
	time::Duration,
};

use octocrab::{
	params::{pulls::Sort, Direction},
	Octocrab,
};
use pr_stats::{get_database, get_repo, TIME_FORMAT};
use rusqlite::{params, Transaction};
use tokio::time;

#[tokio::main]
async fn main() {
	let args = env::args().skip(1).collect::<Vec<_>>();
	real_main(&args).await.unwrap();
}

async fn real_main(args: &[String]) -> Result<(), Box<dyn Error>> {
	let gh = octocrab::OctocrabBuilder::default()
		.personal_token(env::var("GITHUB_PAT").expect("no GITHUB_PAT configured"))
		.build()?;

	let mut database = get_database();
	let tx = database.transaction()?;

	let owner = &args[0];
	let repo = &args[1];

	// get repo
	let repo_id = get_repo(&tx, owner, repo);

	if args.len() == 4 && args[2] == "mergers" {
		fetch_mergers(gh, &tx, owner, repo, repo_id, args[3].parse().unwrap()).await?;
		tx.commit()?;
		return Ok(());
	}

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

macro_rules! extract_row {
	($($t:ty)*) => {
		|_row| {
			let mut _i = 0usize;
			Ok(($(_row.get::<_, $t>({ _i += 1; _i - 1 })?),*))
		}
	};
}

async fn fetch_mergers(
	gh: Octocrab,
	tx: &Transaction<'_>,
	owner: &str,
	repo: &str,
	repo_id: u64,
	limit: u64,
) -> Result<(), Box<dyn Error>> {
	// get PRs with this info missing
	// (rate limit per hour: 5000)
	let mut pulls = vec![];
	let mut last_update = tx.prepare(&format!(
		"SELECT id FROM pulls
		WHERE merger_id IS NULL
		AND merged = 1
		AND repo_id = ?1
		LIMIT {limit}"
	))?;
	for (i, id) in last_update.query_map(params![repo_id], extract_row!(i64))?.enumerate() {
		let id = id?;
		let res: serde_json::Value = gh
			.graphql(&serde_json::json!({
				"query": format!("{{
				repository(owner: \"{owner}\", name: \"{repo}\") {{
				  pullRequest(number: {id}) {{
					mergedBy {{
					  login
					}}
				}}
			}}
		}}")
			}))
			.await?;
		print!("\r{i}/?");
		io::stdout().flush()?;
		let user = res
			.pointer("/data/repository/pullRequest/mergedBy/login")
			.map(|x| x.as_str().unwrap().to_owned());
		let user = user.unwrap_or_else(|| {
			println!("\rWARN: no merger user for merged PR {id}, using ghost");
			"ghost".to_owned()
		});
		pulls.push((id, user));
		// 3600 calls / hour <= 5000 rate limit / hour!
		time::sleep(Duration::from_secs(1)).await;
	}
	drop(last_update);
	let mut get_mergers = tx.prepare("SELECT id, name FROM users")?;
	let mut mergers: HashMap<String, i64> = get_mergers
		.query_map([], extract_row!(i64 String))?
		.map(Result::unwrap)
		.map(|x| (x.1, x.0))
		.collect();
	drop(get_mergers);
	let mut add_merger = tx.prepare("INSERT INTO users (name) VALUES (?1) RETURNING id")?;
	for (_pr, user) in &pulls {
		if mergers.contains_key(user) {
			continue;
		}
		let res = add_merger
			.query_map(params![user], extract_row!(i64))?
			.map(Result::unwrap)
			.next()
			.unwrap();
		mergers.insert(user.clone(), res);
	}
	drop(add_merger);
	let mut update_pr = tx.prepare(
		"UPDATE pulls
		SET merger_id = ?1
		WHERE id = ?2 AND repo_id = ?3",
	)?;
	for (pr, user) in pulls {
		let merger_id = mergers[&user];
		update_pr.execute(params![merger_id, pr, repo_id])?;
	}
	let ratelimit = gh.ratelimit().get().await?;
	let g = ratelimit.resources.graphql.unwrap();
	println!("\rratelimit: {} used, {} remaining", g.used, g.remaining);
	Ok(())
}
