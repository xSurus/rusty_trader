A command line tool for following US Senators trades using Alpaca written in rust.

## Installation
Clone this repository to your local machine.
## Usage 

### 1. Declare your alpaca API keys
Declare your API keys as follows:

> *export APCA_API_KEY_ID=YOUR_KEY_ID*

> *export APCA_API_SECRET_KEY=YOUR_SECRET_KEY*

> *export APCA_API_BASE_URL=https://api.alpaca.markets*

For instructions on how to find these keys, refer to [this guide](https://alpaca.markets/learn/connect-to-alpaca-api/).

### 2. Use the subcommands to follow the orders placed by US chamber members:
Replicate US house member's trades: 

    rusty_trader -f order -c house

Replicate US house member's trades: 

    rusty_trader -f order -c senate

Show the performance of your portfolio:
    
    rusty_trader -f alpaca_positions


If you need help, try `rusty_trader -h` for usage information.

## Automation
To automatically replicate the trades placed by US senators, you might create a crontab that looks something like the following:
> *APCA_API_KEY_ID=YOUR_KEY_ID*

> *APCA_API_SECRET_KEY=YOUR_SECRET_KEY*

> *APCA_API_BASE_URL=https://api.alpaca.markets*

> *0 19 * * * ./Documents/CodingProjects/rust_trader/trader/target/debug/trader -f order -c house*
