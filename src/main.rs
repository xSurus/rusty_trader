use apca;
use apca::ApiInfo;
use apca::Client;
use apca::api::v2::account;


use clap::Parser;

// mod portfolio;
mod alpaca;



#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Which function to execute
    /// [account, positions, orders, order]
    #[arg(short, long)]
    func: String,

    /// Who's investments you want to track
    /// [house, senate]
    #[arg(short, long, default_value = "senate")]
    chamber: String,
}

#[tokio::main]
async fn main() {
    // set up connection to alpaca API
    let args = Cli::parse();

    let client = Client::new(ApiInfo::from_env().unwrap());
    let account = client.issue::<account::Get>(&()).await.unwrap();
    match args.func.as_str() {
        "account" => {
            println!("Account information: {:#?}", account);
        }
        "alpaca_positions" => {
            alpaca::print_positions(client, account.cash).await;
        }
        "portfolio_positions" => {
            println!("Not implemented yet");
        }
        "add_portfolio_position" => {
            println!("Not implemented yet");
        }
        "order" => {
            alpaca::check_trade(client, args.chamber.as_str()).await;
        }
        _ => {
            println!("Invalid function");
        }
    }
}
