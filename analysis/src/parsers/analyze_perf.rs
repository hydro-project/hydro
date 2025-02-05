use std::collections::HashMap;

use regex::Regex;

pub fn analyze_perf(file_path: &str) {
    let file = std::fs::read_to_string(file_path).unwrap();
    let mut usages = HashMap::new();
    let operator_regex = Regex::new(r"(\d+.\d+)%.*\[\.\].*op_(.*)__loc").unwrap();

    for line in file.lines() {
        for cap in operator_regex.captures_iter(line) {
            let cpu_usage = cap[1].parse::<f64>().unwrap();
            let operator = &cap[2];
            let prev_usage = usages.insert(operator.to_string(), cpu_usage);
            // If the same operator had a different CPU usage in the file before, something's wrong
            debug_assert!(prev_usage.is_none(), "Operator {} had CPU usage {:?}, then {:?}", operator, prev_usage, cpu_usage);
        }
    }

    println!("CPU Usages: {:?}", usages);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_perf() {
        analyze_perf("leader.perf.out");
    }
}