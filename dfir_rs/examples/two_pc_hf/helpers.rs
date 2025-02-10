pub fn parse_out<T: std::str::FromStr>(line: String) -> Option<T> {
    line.trim().parse::<T>().ok()
}

use rand::Rng;
pub fn decide(odds: u8) -> bool {
    let mut rng = rand::thread_rng();
    rng.gen_range(0..100) <= odds
}
