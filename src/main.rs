mod ctfd;

use std::collections::HashMap;

use ctfd::{CTFdClient, ChallengeSolver};
use serenity::http::Http;
use serenity::model::webhook::Webhook;

use clap::Parser;
use sqlite::State;

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
    #[arg(short, long)]
    first_blood_only: bool,

    /// Whether to announce existing solves
    #[arg(short, long)]
    dont_skip_existing_solves: bool,

    /// Refresh interval in seconds
    #[arg(short, long, default_value = "5")]
    refresh_interval_seconds: u64,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let http = Http::new("");
    let webhook = Webhook::from_url(&http, &args.webhook_url)
        .await
        .expect("Supply a webhook url");

    let ctfd_client = CTFdClient::new(args.ctfd_url, args.ctfd_api_key);

    // A hashmap of challenge id to their solvers
    let mut announced_solves: HashMap<i64, Vec<ChallengeSolver>> = HashMap::new();

    let db_conn = sqlite::open("ctfd_discord.sqlite3").unwrap();

    db_conn
        .execute("CREATE TABLE IF NOT EXISTS announced_solves (id INTEGER PRIMARY KEY AUTOINCREMENT, challenge_id INTEGER, solver_id INTEGER);",
        )
        .unwrap();

    // Populate the announced solves hashmap with the existing solves
    let mut statement = db_conn
        .prepare("SELECT challenge_id, solver_id FROM announced_solves;")
        .unwrap();

    while let Ok(State::Row) = statement.next() {
        announced_solves
            .entry(statement.read(0).unwrap())
            .or_insert_with(Vec::new)
            .push(ChallengeSolver {
                account_id: statement.read(1).unwrap(),
                name: "".to_string(),
            });
    }

    // Skips announcing existing solves by default
    if !args.dont_skip_existing_solves {
        // Get a list of all challenges
        let challenges = ctfd_client.get_challenges().await.unwrap();

        for challenge in challenges {
            // Get a list of all solvers for the challenge
            let solvers = challenge.get_solves(&ctfd_client).await.unwrap();

            for solver in solvers {
                // Add the solve to the list of announced solves
                announced_solves
                    .entry(challenge.id)
                    .or_insert_with(Vec::new)
                    .push(solver);
            }
        }
    }

    loop {
        // Get a list of all challenges
        let challenges = ctfd_client.get_challenges().await.unwrap();

        for challenge in challenges {
            // Get a list of all solvers for the challenge
            let solvers = challenge.get_solves(&ctfd_client).await.unwrap();

            for solver in solvers {
                if args.first_blood_only && announced_solves.contains_key(&challenge.id) {
                    continue;
                }

                // Check if the solve is new
                if !announced_solves
                    .get(&challenge.id)
                    .unwrap_or(&Vec::new())
                    .contains(&solver)
                {
                    // Send a message to the webhook
                    webhook
                        .execute(&http, false, |w| {
                            // If this is the first solve
                            if !announced_solves.contains_key(&challenge.id) {
                                w.content(format!(
                                    "First blood for {} goes to {}! :knife::drop_of_blood:",
                                    challenge.name, solver.name
                                ))
                            } else {
                                w.content(format!(
                                    "{} just solved {}! :tada:",
                                    solver.name, challenge.name
                                ))
                            }
                        })
                        .await
                        .expect("Could not execute webhook.");

                    // Add the solve to the database
                    let mut statement = db_conn
                        .prepare(
                            "INSERT INTO announced_solves (challenge_id, solver_id) VALUES (?, ?);",
                        )
                        .unwrap();

                    statement.bind(1, challenge.id).unwrap();
                    statement.bind(2, solver.account_id).unwrap();
                    statement.next().unwrap();

                    // Add the solve to the list of announced solves
                    announced_solves
                        .entry(challenge.id)
                        .or_insert_with(Vec::new)
                        .push(solver);
                }
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(
            args.refresh_interval_seconds,
        ))
        .await;
    }
}
