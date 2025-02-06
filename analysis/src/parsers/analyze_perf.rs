use std::collections::HashMap;

use regex::Regex;

pub fn analyze_perf(file_path: &str) {
    let file = std::fs::read_to_string(file_path).unwrap();

    let mut total_samples = 0f64;
    let mut samples_per_operator = HashMap::new();
    let operator_regex = Regex::new(r"op_(.*?)__loc").unwrap();

    for line in file.lines() {
        let n_samples_index = line.rfind(' ').unwrap() + 1;
        let n_samples = &line[n_samples_index..].parse::<f64>().unwrap();

        for cap in operator_regex.captures_iter(line) {
            let operator = &cap[1];
            let prev_samples = samples_per_operator.entry(operator.to_string()).or_insert(0f64);
            *prev_samples += n_samples;
        }

        total_samples += n_samples;
    }

    samples_per_operator.iter().for_each(|(k, v)| println!("Operator {:?}: {:.2}%", k, v / total_samples * 100f64));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_perf() {
        analyze_perf("cluster1.data.folded");
    }
}