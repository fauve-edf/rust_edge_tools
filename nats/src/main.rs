use std::collections::HashMap;

use anyhow::{anyhow, bail, Result};
use async_nats::{Client, ConnectOptions};
use clap::{Parser, Subcommand};
use futures::StreamExt;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    // Address
    #[clap(value_parser)]
    address: String,

    // Authentication
    #[clap(short, long, action)]
    username: Option<String>,
    #[clap(short, long, action)]
    password: Option<String>,
    #[clap(short, long, action)]
    token: Option<String>,
    // meta command
    #[clap(short, long, action)]
    verbose: Option<bool>,

    // Subcommand
    #[clap(subcommand)]
    command: Subcommands,
}

// yeah I know you're not supposed to pluralize enums, but the conflict with "Subcommand" derive is annoying.
#[derive(Subcommand)]
enum Subcommands {
    Subscribe {
        #[clap(short, long, action)]
        subject: String,
        #[clap(short, long, action)]
        watch: Option<bool>,
    },

    Publish {
        #[clap(short, long, action)]
        subject: String,
        // TODO: allow either a file name or a direct string.
        #[clap(short, long, action)]
        message: String,
    },
    ListSubjects {
        #[clap(short, long, action)]
        filter_response: bool,
    },
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let cli = Args::parse();
    let connect_options = match get_connect_options(&cli) {
        Ok(opts) => opts,
        Err(err) => {
            log::error!("Unable to parse options: {err}");
            return;
        }
    };

    let connection = match connect_options.connect(cli.address).await {
        Ok(cnxn) => cnxn,
        Err(err) => {
            log::error!("Unable to connect to remote: {err}");
            return;
        }
    };

    match cli.command {
        Subcommands::Subscribe { subject, watch } => {
            if let Err(err) = subscribe(&connection, subject, watch, cli.verbose).await {
                log::error!("Aborted subscription: {err}");
            }
        }
        Subcommands::Publish { subject, message } => {
            if let Err(err) = publish(&connection, subject, message).await {
                log::error!("Could not publish: {err}");
            }
        }
        Subcommands::ListSubjects { filter_response } => {
            if let Err(err) = list_topics(&connection, filter_response).await {
                log::error!("Error while listing topics: {err}");
            }
        }
    }
}

fn get_connect_options(args: &Args) -> Result<ConnectOptions> {
    let opts = match (
        args.username.as_ref(),
        args.password.as_ref(),
        args.token.as_ref(),
    ) {
        // TODO: add more authentication options.
        (Some(user), Some(password), None) => {
            log::info!("Using username and password to connect to nats.");
            ConnectOptions::with_user_and_password(user.clone(), password.clone())
        }
        (Some(_), None, _) => {
            bail!("Username but no password specified.")
        }
        (None, Some(_), _) => {
            bail!("Password but no username specified")
        }
        (None, None, Some(token)) => {
            log::info!("Using token to connect to nats");
            ConnectOptions::with_token(token.clone())
        }
        (Some(_), Some(_), Some(_)) => {
            bail!("Username and password, token specified. Can't decide which to use.")
        }
        (None, None, None) => {
            log::info!("No authentication specified");
            ConnectOptions::new()
        }
    };

    let opts = opts.event_callback(|event| async move {
        // Not sure what to throw in with this block.
        // TODO: a more reified vision for this block.
        match event {
            async_nats::Event::Disconnect => {
                log::info!("Disconnected nats connection");
            }
            async_nats::Event::Reconnect => log::info!("Nats client reconnected,"),
            async_nats::Event::ClientError(err) => {
                log::error!("Nats client received error : {}", err)
            }
            other => log::warn!("Nats client unused event: {}", other),
        };
    });

    Ok(opts)
}

async fn subscribe(
    connection: &Client,
    subject: String,
    watch: Option<bool>,
    verbose: Option<bool>,
) -> Result<()> {
    let watch = watch.unwrap_or(false);
    let verbose = verbose.unwrap_or(false);

    let mut subscription = connection
        .subscribe(subject)
        .await
        .map_err(|err| anyhow!("Unable to subscribe: {err}"))?;

    for message in subscription.next().await {
        let payload = if let Ok(s) = String::from_utf8(message.payload.to_vec()) {
            s
        } else {
            bail!("Unable to parse message into utf-8. Please petition to authors to display raw bytes.")
        };

        if verbose {
            println!("Description: {:?}", message.description);
            println!("Status: {:?}", message.status);
            println!("Subject: {}", message.subject);
            println!("Payload: {}", payload);
        } else {
            println!("{}", payload);
        }

        if !watch {
            break;
        }
    }
    Ok(())
}

async fn publish(connection: &Client, subject: String, payload: String) -> Result<()> {
    connection
        .publish(subject, payload.into())
        .await
        .map_err(|err| anyhow!("Unable to publish: {:?}", err))
}

async fn list_topics(connection: &Client, filter_response: bool) -> Result<()> {
    let mut seen_subscriptions = HashMap::new();
    let mut subscription = connection
        .subscribe(">".to_string())
        .await
        .map_err(|err| anyhow!("Error subscribing: {err}"))?;

    loop {
        let message = subscription.next().await.unwrap();
        if filter_response && message.subject.starts_with("_INBOX") {
            continue;
        }
        if let None = seen_subscriptions.insert(message.subject.clone(), ()) {
            println!("{}", message.subject);
        }
    }
}
