use std::process::exit;

use lazy_static::lazy_static;
use num_decimal::Num;
use regex::Regex;

use apca;
use apca::Client;
use apca::Error;
use apca::api::v2::account;
use apca::api::v2::positions;
use apca::api::v2::order;
use apca::api::v2;
use apca::data::v2::bars;

use chrono;

#[derive(Debug)]
struct Order {
    o_type: String,
    ticker: String,
    amount: i64,
}

pub async fn check_trade(client : Client, chamber : &str) -> (){
    let trades = get_trades(chamber).await.unwrap();
    if  trades.status() == 200 {
        let orders: Vec<Order> = match chamber {
            "house" => {
                get_trades_house(trades).await.unwrap()
            }
            "senate" => {
                get_trades_senate(trades).await.unwrap()
            }
            _ => {
                eprintln!("Invalid chamber");
                exit(1);
            }
        };
        place_orders(orders, client).await;
    } else {
        // exit program
        println!("Request failed: no transactions reported today");
        exit(1);
    }
}

pub async fn print_positions(client: Client, cash: Num) -> () {
    let positions = get_positions(&client).await;
    let mut sum = 0;
    println!(
        "{0: >10} | {1: >12} | {2: >10} | {3: >10} | {4: >10}",
        "Name", "Amount", "Relative gain", "Absolute gain", "Market value"
    );
    println!("=========================================================================");
    for position in positions.unwrap() {
        let value = position.market_value.unwrap();
        sum += value.to_i64().unwrap();
        println!(
            "{0: >10} | {1: >12.2} | {2: >12.2}% | {3: >12.2}$ | {4: >10.2}$",
            position.symbol,
            position.quantity,
            position.unrealized_gain_total_percent.unwrap(),
            position.unrealized_gain_total.unwrap(),
            value
        );
    }
    println!("=========================================================================");
    println!("Your total balance is: {:.2}", sum + cash.to_i64().unwrap());
}

async fn get_trades(chamber : &str) -> Result<reqwest::Response, reqwest::Error> {
    // get date in US format from 4 days ago, as trades take some time to be transcribed
    let now = chrono::offset::Local::now()- chrono::Duration::days(4);
    let mut date = now.format("%m-%d-%Y").to_string();
    date = date.replace("-", "_");
    // date = "11_21_2022".to_string();
    // request senate trades for date
    let request_url = format!("http://{chamber}-stock-watcher-data.s3-us-west-2.amazonaws.com/data/transaction_report_for_{date}.json");
    let response = reqwest::get(&request_url).await;
    // if the request was successful, parse the response
    return response;
}

async fn get_trades_house (trades: reqwest::Response) -> Result<Vec<Order>, Error> {
    let mut orders = Vec::<Order>::new();
    let json = trades.json::<serde_json::Value>().await.unwrap();
    for line in json.as_array().unwrap() {
        let transactions = line.as_object().unwrap().get("transactions").unwrap().as_array().unwrap();
        for transaction in transactions {
            let current_transaction = transaction.as_object().unwrap();
            let description = current_transaction.get("description").unwrap().as_str().unwrap();
            let tick = current_transaction.get("ticker").unwrap().as_str().unwrap();
            let amount = extract_amount(current_transaction.get("amount").unwrap().as_str().unwrap()).unwrap();
            if description.contains("Bond") || description.contains("Option") || description.contains("Note") || tick == "--" {
                continue;
            }
            orders.push(Order {
                o_type: current_transaction.get("transaction_type").unwrap().as_str().unwrap().to_string(),
                ticker: tick.to_string(),
                amount: amount,
            });
        }
    }
    Ok(orders)
}
async fn get_trades_senate (trades: reqwest::Response) -> Result<Vec<Order>, Error> {
    let mut orders = Vec::<Order>::new();
    let json = trades.json::<serde_json::Value>().await.unwrap();
    for line in json.as_array().unwrap() {
        let transactions = line.as_object().unwrap().get("transactions").unwrap().as_array().unwrap();
        for transaction in transactions {
            let current_transaction = transaction.as_object().unwrap();
            if current_transaction.get("asset_type").unwrap() == "Stock" && current_transaction.get("ticker").unwrap() != "--" {
                let tick = extract_ticker(current_transaction.get("ticker").unwrap().as_str().unwrap()).unwrap();
                let amount = extract_amount(current_transaction.get("amount").unwrap().as_str().unwrap()).unwrap();
                orders.push(Order {
                    // shouldn't use ticker, can be -- falsely ??
                    o_type: current_transaction.get("type").unwrap().as_str().unwrap().to_string(),
                    ticker: tick.to_string(),
                    amount: amount,
                });
            }
        }
    }
    Ok(orders)
}

async fn place_orders(orders: Vec<Order>, client: Client) -> () {
    // set up connection to alpaca API
    let cash = client.issue::<account::Get>(&()).await.unwrap().cash / 3;
    // sum over the amount of the orders if the order is a buy
    let sum = orders.iter().fold(0, |acc, order| {
        if order.o_type == "Purchase" || order.o_type == "purchase" {
            println!("Order: {:#?}", order);
            return acc + order.amount;
        }
        return acc;
    });
    if sum == 0 {
        println!("No orders to place");
        exit(1);
    }
    let adjust = cash / sum;
    for order in orders {
        let request;
        if order.o_type == "Purchase" || order.o_type == "purchase" {
            let to_buy = Num::from(order.amount) * &adjust;
            let last_price = get_current_ticker_value(&client, &order.ticker).await.unwrap();
            if last_price == 0 {
                continue;
            }
            let mut quantity = to_buy / last_price;
            if quantity < Num::from(1) {
                quantity = Num::from(1);
            }
            request = order::OrderReqInit {
                type_: order::Type::Market,
                ..Default::default()
            }.init(order.ticker, order::Side::Buy, order::Amount::quantity(quantity));
        } else if order.o_type == "Sale (Full)"  || order.o_type == "sale_full" {
            // make new num
            let mut currently_held = 0;
            for pos in get_positions(&client).await.unwrap() {
                if pos.symbol == order.ticker {
                    currently_held = pos.quantity.to_i64().unwrap();
                }
            }
            if currently_held == 0 {
                continue;
            }
            request = order::OrderReqInit {
                type_: order::Type::Market,
                ..Default::default()
            }.init(order.ticker, order::Side::Buy, order::Amount::quantity(currently_held));
        } else if order.o_type == "Sale (Partial)" || order.o_type == "sale_partial" {
            let mut currently_held = 0;
            for pos in get_positions(&client).await.unwrap() {
                if pos.symbol == order.ticker {
                    currently_held = pos.quantity.to_i64().unwrap();
                }
            }
            if currently_held == 0 {
                continue;
            }
            request = order::OrderReqInit {
                type_: order::Type::Market,
                ..Default::default()
            }.init(order.ticker, order::Side::Sell, order::Amount::quantity(currently_held / 2));
        } else {
            // exit program
            println!("Request failed: didn't recognise action");
            continue;
        }

        let place_order = client.issue::<order::Post>(&request).await;
        match place_order {
            Ok(order) => {
                println!("Order placed: {:#?}", order);
            },
            Err(e) => {
                // to be fixed, good errors!
                let error = e;
                println!("Request failed: {:#?}", error);

                continue;
            }
        };
    }
}

async fn get_positions(client: &Client) -> Result<Vec<v2::position::Position>, Error> {
    let open_positions = client.issue::<positions::Get>(&()).await.unwrap();
    Ok(open_positions)
}

async fn get_current_ticker_value(client: &Client, ticker: &str) -> Result<i64, Error> {
    // get the value that the ticker is trading at right now
    let now = chrono::offset::Local::now();
    // get the time 20 minutes ago
    let now = now - chrono::Duration::minutes(15);
    let utc_now = now.with_timezone(&chrono::offset::Utc);
    let last_value = bars::BarsReq {
        symbol: ticker.to_string(),
        limit: Some(1),
        start: utc_now - chrono::Duration::days(1),
        end: utc_now,
        timeframe: bars::TimeFrame::OneDay,
        adjustment: Default::default(),
        feed: Default::default(),
        page_token: None,
    };
    let last_value = client.issue::<bars::Get>(&last_value).await.unwrap();
    return match last_value.bars.len() {
        0 => {
            println!("No value found for {}", ticker);
            Ok(0)
        },
        _ => {
            let last_value = last_value.bars.get(0).unwrap().close.clone();
            Ok(last_value.to_i64().unwrap())
        }
    }
}


fn extract_ticker(input: &str) -> Option<&str> {
    // regex expression to extract text everything between > and <
    lazy_static! {
        static ref RE: Regex = Regex::new(
            ">([A-Z]+)<"
        ).unwrap();
    }
    RE.captures(input).and_then(|cap| cap.get(1)).map(|m| m.as_str())
}

fn extract_amount(input: &str) -> Option<i64> {
    // regex expression to extract amount given by $250,001 - $500,000 and then average it
    lazy_static! {
        static ref RE: Regex = Regex::new(
            r"\$(\d+,\d+) - \$(\d+,\d+)"
        ).unwrap();
    }
    let min = RE.captures(input).and_then(|cap| cap.get(1)).map(|m| m.as_str())?;
    let min = min.replace(",", "").parse::<i64>().unwrap();
    let max = RE.captures(input).and_then(|cap| cap.get(2)).map(|m| m.as_str())?;
    let max = max.replace(",", "").parse::<i64>().unwrap();
    return Some((min + max) / 2);
}