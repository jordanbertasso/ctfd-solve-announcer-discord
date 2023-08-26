mod ctfd;

use std::collections::HashMap;

use ctfd::{CTFdClient, ChallengeSolver};
use serenity::http::Http;
use serenity::model::webhook::Webhook;

use clap::Parser;
use rusqlite::Connection;

/// A Discord webhook bot to announce CTFd solves
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Discord Webhook URL
    #[arg(short, long)]
    webhook_url: String,

    /// CTFd URL
    #[arg(long, short = 'c')]
    ctfd_url: String,

    /// CTFd API Key
    #[arg(long, short = 'a')]
    ctfd_api_key: String,

    /// Whether to only announce first bloods
    #[arg(long, default_value = "true")]
    announce_first_blood_only: bool,

    /// Whether to skip announcing existing solves
    #[arg(short, long, default_value = "true")]
    skip_announcing_existing_solves: bool,

    /// Refresh interval in seconds
    #[arg(short, long, default_value = "5")]
    refresh_interval_seconds: u64,
}

async fn populate_announced_solves(
    ctfd_client: &CTFdClient,
    announced_solves: &mut HashMap<i64, Vec<ChallengeSolver>>,
) {
    // Get a list of all challenges
    let challenges = ctfd_client.get_challenges().await.unwrap();

    for challenge in challenges {
        // Get a list of all solvers for the challenge
        let solvers = challenge.get_solves(ctfd_client).await.unwrap();

        for solver in solvers {
            // Add the solve to the list of announced solves
            announced_solves
                .entry(challenge.id)
                .or_insert_with(Vec::new)
                .push(solver);
        }
    }
}

async fn announce_solves(
    http: &Http,
    webhook: &Webhook,
    ctfd_client: &CTFdClient,
    announced_solves: &mut HashMap<i64, Vec<ChallengeSolver>>,
    db_conn: &Connection,
    announce_first_blood_only: bool,
) {
    // Get a list of all challenges
    let challenges = ctfd_client.get_challenges().await.unwrap();

    for challenge in challenges {
        // Get a list of all solvers for the challenge
        let solvers = challenge.get_solves(ctfd_client).await.unwrap();

        for solver in solvers {
            // If we only want to announce first bloods and this challenge has already been solved then skip
            if announce_first_blood_only && announced_solves.contains_key(&challenge.id) {
                continue;
            }

            // Check if the solve is new
            if !announced_solves
                .get(&challenge.id)
                .unwrap_or(&Vec::new())
                .contains(&solver)
            {
                println!("Announcing solve for {} by {}", challenge.name, solver.name);

                // Send a message to the webhook
                webhook
                    .execute(&http, false, |w| {
                        // If this is the first solve
                        if !announced_solves.contains_key(&challenge.id) {
                            w.content(format!(
                                "First blood for **{}** goes to **{}**! :knife::drop_of_blood:",
                                challenge.name, solver.name
                            ))
                        } else {
                            w.content(format!("{} just solved {}! :tada:", solver.name, challenge.name))
                        }
                    })
                    .await
                    .expect("Could not execute webhook.");

                // Add the solve to the database
                db_conn
                    .execute(
                        "INSERT INTO announced_solves (challenge_id, solver_id) VALUES (?1, ?2);",
                        (&challenge.id, &solver.account_id),
                    )
                    .unwrap();

                // Add the solve to the list of announced solves
                announced_solves
                    .entry(challenge.id)
                    .or_insert_with(Vec::new)
                    .push(solver);
            }
        }
    }
}

type TeamId = i64;
type TeamPosition = i64;

/// Announce when any team in the top 10 gets overtaken by any other team
/// Scores are compared to do so
async fn announce_top_10_overtakes(
    http: &Http,
    webhook: &Webhook,
    ctfd_client: &CTFdClient,
    db_conn: &Connection,
)
{
    // Get the previous top 10 teams
    let mut statement = db_conn
        .prepare("SELECT id, position FROM top_10_teams;")
        .unwrap();

    let previous_top_10_iter = statement
        .query_map([], |row| {
            Ok(
                (
                row.get::<_, TeamId>(0).unwrap(),
                row.get::<_, TeamPosition>(1).unwrap(),
                )
            )
        })
        .unwrap();
 
    let mut previous_top_10_teams: HashMap<TeamId, TeamPosition> = HashMap::new();

    for previous_top_10 in previous_top_10_iter {
        let (team_id , position) = previous_top_10.unwrap();

        previous_top_10_teams.insert(team_id, position);
    }

    // Get the current top 10 teams
    let top_10_teams: HashMap<TeamId, TeamPosition> = ctfd_client.get_top_10_teams().await.unwrap();

    // For a given team in the current top 10
    //     Check if they increased their position in the top 10 or have entered the top 10

    //     If they increased their position in the top 10
    //         Announce that they have overtaken the team that was previously at their current position
        
    //     If they have entered the top 10
    //         Announce that they have overtaken the team that was previously at their current position

    for (team_id, position) in top_10_teams.iter() {
        let team = ctfd_client.get_team(*team_id).await.unwrap();
        let previous_team_in_position = previous_top_10_teams.iter().find(|(_, p)| **p == *position);

        // If there was no team at the current position then skip
        if previous_team_in_position.is_none() {
            continue;
        } 

        let previous_team = ctfd_client.get_team(*previous_team_in_position.unwrap().0).await.unwrap();

        if previous_top_10_teams.contains_key(team_id) {
            let previous_position = previous_top_10_teams.get(team_id).unwrap();

            if position < previous_position {
                webhook
                    .execute(&http, false, |w| {
                        w.content(format!("**{}** has overtaken **{}** in the top 10! Landing in position {}", team.name, previous_team.name, position))
                    })
                    .await
                    .expect("Could not execute webhook.");
            }
        } else {
            webhook
                .execute(&http, false, |w| {
                    w.content(format!("**{}** has entered the top 10! Overtaking **{}** for position {}", team.name, previous_team.name, position))
                })
                .await
                .expect("Could not execute webhook.");
        }
    }

    // Update the top 10 teams in the database
    db_conn
        .execute("DELETE FROM top_10_teams;", ())
        .unwrap();

    for (team_id, position) in top_10_teams.iter() {
        db_conn
            .execute(
                "INSERT INTO top_10_teams (id, position) VALUES (?1, ?2);",
                (&team_id, &position),
            )
            .unwrap();
    }
}

#[tokio::main]
async fn main() {
    println!("Starting CTFd Discord Solve Announcer Bot");

    let args = Args::parse();

    let http = Http::new("");
    let webhook = Webhook::from_url(&http, &args.webhook_url)
        .await
        .expect("Supply a webhook url");

    let ctfd_client = CTFdClient::new(args.ctfd_url, args.ctfd_api_key);

    // A hashmap of challenge id to their solvers
    let mut announced_solves: HashMap<i64, Vec<ChallengeSolver>> = HashMap::new();

    let db_conn = Connection::open("ctfd_discord.sqlite3").unwrap();

    db_conn
    .execute("CREATE TABLE IF NOT EXISTS announced_solves (id INTEGER PRIMARY KEY AUTOINCREMENT, challenge_id INTEGER, solver_id INTEGER);", ())
    .unwrap();

    db_conn
    .execute("CREATE TABLE IF NOT EXISTS top_10_teams (id INTEGER PRIMARY KEY AUTOINCREMENT, position INTEGER);", ())
    .unwrap();

    // Populate the announced solves hashmap with the existing solves
    let mut statement = db_conn
        .prepare("SELECT challenge_id, solver_id FROM announced_solves;")
        .unwrap();

    let announced_iter = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0).unwrap(),
                ChallengeSolver {
                    account_id: row.get::<_, i64>(1).unwrap(),
                    name: "".to_string(),
                },
            ))
        })
        .unwrap();

    for announced in announced_iter {
        let (challenge_id, solver) = announced.unwrap();

        announced_solves
            .entry(challenge_id)
            .or_insert_with(Vec::new)
            .push(solver);
    }

    // Skips announcing existing solves by default
    if args.skip_announcing_existing_solves {
        populate_announced_solves(&ctfd_client, &mut announced_solves).await;
    }

    loop {
        announce_solves(&http, &webhook, &ctfd_client, &mut announced_solves, &db_conn, args.announce_first_blood_only).await;
        announce_top_10_overtakes(&http, &webhook, &ctfd_client, &db_conn).await;

        tokio::time::sleep(std::time::Duration::from_secs(args.refresh_interval_seconds)).await;
    }
}
