use clap::{Parser, Subcommand, ValueEnum};
use std::net::SocketAddr;
use tokio_modbus::{
    client::{Reader, Writer},
    slave::{Slave, SlaveContext},
};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(value_parser)]
    address: String,

    #[clap(subcommand)]
    command: Option<Subcommands>,

    #[clap(value_parser)]
    watch: Option<bool>,
}

#[derive(Subcommand)]
enum Subcommands {
    ReadRegister {
        #[clap(short, long, action)]
        address: u16,
        #[clap(short, long, action)]
        kind: RegisterKind,
        #[clap(short, long, action)]
        watch: Option<bool>,
        #[clap(short, long, action)]
        unit_id: Option<u8>,
        #[clap(short, long, action)]
        count: Option<u16>,
        #[clap(short, long, action)]
        presentation: Option<ReadPresentationKind>,
    },

    WriteRegister {
        #[clap(short, long, action)]
        address: u16,
        #[clap(short, long, action)]
        value: u16,
        #[clap(short, long, action)]
        unit_id: Option<u8>,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum RegisterKind {
    Holding,
    Input,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum ReadPresentationKind {
    Hex,
    Dec,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let cli = Args::parse();
    let addr = match cli.address.parse::<SocketAddr>() {
        Ok(addr) => addr,
        Err(err) => {
            log::error!("Unable to parse address {}: {}", cli.address, err);
            std::process::exit(-1);
        }
    };

    let command = if let Some(command) = cli.command {
        command
    } else {
        log::warn!("No subcommand specified.");
        std::process::exit(-1);
    };

    match command {
        Subcommands::ReadRegister {
            address,
            kind,
            watch,
            unit_id,
            count,
            presentation,
        } => {
            // Set defaults
            let count = if let Some(cnt) = count { cnt } else { 1 };
            let unit_id = if let Some(uid) = unit_id { uid } else { 1 };
            let watch = if let Some(w) = watch { w } else { false };
            let presentation = if let Some(p) = presentation {
                p
            } else {
                ReadPresentationKind::Dec
            };

            loop {
                let result = match read_modbus(&addr, address, count, kind, unit_id).await {
                    Ok(result) => result,
                    Err(error) => {
                        log::error!("Received error. Aborting: {error}");
                        std::process::exit(-1);
                    }
                };

                let formatted_result = match presentation {
                    ReadPresentationKind::Dec => {
                        // no formatting
                        let result: Vec<String> =
                            result.iter().map(|number| format!("{}", number)).collect();
                        format!("{:?}", result)
                    }
                    ReadPresentationKind::Hex => {
                        let result: Vec<String> = result
                            .iter()
                            .map(|number| format!("{:#x}", number))
                            .collect();
                        format!("{:?}", result)
                    }
                };

                println!("{formatted_result}");

                if !watch {
                    break;
                }
            }
        }
        Subcommands::WriteRegister {
            address,
            value,
            unit_id,
        } => {
            // defaults
            let unit_id = if let Some(uid) = unit_id { uid } else { 1 };
            if let Err(err) = write_modbus(&addr, address, value, unit_id).await {
                log::error!("Unable to write modbus address: {err}");
                std::process::exit(-1);
            }
        }
    }
}

async fn read_modbus(
    socket_addr: &SocketAddr,
    address: u16,
    count: u16,
    kind: RegisterKind,
    unit_id: u8,
) -> Result<Vec<u16>, Box<dyn std::error::Error>> {
    let mut context = tokio_modbus::client::tcp::connect(*socket_addr).await?;
    context.set_slave(Slave(unit_id));
    let result = match kind {
        RegisterKind::Holding => context.read_holding_registers(address, count).await?,
        RegisterKind::Input => context.read_input_registers(address, count).await?,
    };
    Ok(result)
}

async fn write_modbus(
    socket_addr: &SocketAddr,
    address: u16,
    value: u16,
    unit_id: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut context = tokio_modbus::client::tcp::connect(*socket_addr).await?;
    context.set_slave(Slave(unit_id));
    context.write_single_register(address, value).await?;
    Ok(())
}
