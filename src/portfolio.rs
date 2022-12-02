pub struct Position {
    pub symbol: String,
    pub quantity: i32,
    pub price: f64,
    pub value: f64,
}
// add an entry to a json file with stock positions
pub fn add_positions(positions: &mut Vec<Position>) {
}

// remove an entry to a json file with stock positions
pub fn remove_positions(positions: &mut Vec<Position>, symbol: &str) {
    let mut index = 0;
    for position in positions {
        if position.symbol == symbol {
            break;
        }
        index += 1;
    }
}
// print the current portfolio
pub fn print_portfolio(positions: &Vec<Position>) {
    println!("Portfolio:");
    for position in positions {
        println!("{}: {} shares at ${}", position.symbol, position.quantity, position.price);
    }
}