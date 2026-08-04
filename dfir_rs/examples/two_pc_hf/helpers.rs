pub fn parse_out<T: std::str::FromStr>(line: String) -> Option<T> {
    line.trim().parse::<T>().ok()
}

use rand::RngExt;
pub fn decide(odds: u8) -> bool {
    let mut rng = rand::rng();
    rng.random_range(0..100) <= odds
}
