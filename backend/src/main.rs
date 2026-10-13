use clap::{Parser, Subcommand};
use gr_logistics::{cli, config::Config, logging};

#[derive(Parser)]
#[command(name = "gr-logistics", about = "GR Logistics backend")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run the HTTP API (the default when no subcommand is given).
    Serve,
    /// Apply pending SQL migrations from `migrations/`.
    Migrate,
    /// Create a user, or reset an existing user's password/role. There is no public
    /// registration — this is the only way accounts get provisioned.
    CreateUser {
        #[arg(long)]
        email: String,
        #[arg(long)]
        password: String,
        #[arg(long, default_value = "admin")]
        role: String,
    },
    /// Exit 0 if this instance's /api/health is OK (container healthcheck; needs only PORT).
    Healthcheck,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Debug builds read .env.development; release builds get their env from the container.
    #[cfg(debug_assertions)]
    dotenvy::from_filename(".env.development").ok();

    let command = Cli::parse().command.unwrap_or(Command::Serve);
    if let Command::Healthcheck = command {
        return cli::healthcheck::run().await;
    }

    let config = Config::from_env()?;
    logging::init(&config);

    match command {
        Command::Serve => gr_logistics::serve(config).await,
        Command::Migrate => cli::migrate::run(&config).await,
        Command::CreateUser {
            email,
            password,
            role,
        } => cli::create_user::run(&config, &email, &password, &role).await,
        Command::Healthcheck => unreachable!("handled before config is loaded"),
    }
}
