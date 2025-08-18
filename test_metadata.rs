use std::any::type_name;

// Simple test to verify our type name matching logic
fn main() {
    println!(
        "TotalOrder: {}",
        type_name::<hydro_lang::stream::TotalOrder>()
    );
    println!("NoOrder: {}", type_name::<hydro_lang::stream::NoOrder>());
    println!(
        "ExactlyOnce: {}",
        type_name::<hydro_lang::stream::ExactlyOnce>()
    );
    println!(
        "AtLeastOnce: {}",
        type_name::<hydro_lang::stream::AtLeastOnce>()
    );

    // Test our matching logic
    let ordering_type = type_name::<hydro_lang::stream::TotalOrder>();
    let is_total_order = ordering_type.contains("TotalOrder");
    println!("TotalOrder matches: {}", is_total_order);

    let retries_type = type_name::<hydro_lang::stream::ExactlyOnce>();
    let is_exactly_once = retries_type.contains("ExactlyOnce");
    println!("ExactlyOnce matches: {}", is_exactly_once);
}
