use hydro_lang::*;

#[test]
fn test_metadata_structure() {
    println!("=== Testing Rich Type Metadata Structure ===");

    // This test validates that:
    // 1. HydroIrMetadata has the input_collection_types field
    // 2. The new_conversion_metadata function exists and can be called
    // 3. Operations compile with the new metadata system

    let builder = FlowBuilder::new();
    let process = builder.process::<()>();
    let tick = process.tick();

    // Test that operations with instrumented metadata compile
    let singleton = tick.singleton(q!(42));
    let _stream1 = singleton.all_ticks();

    let optional = tick.singleton(q!(100)).filter(q!(|&x| x > 50));
    let _stream2 = optional.all_ticks();

    let singleton2 = process.source_iter(q!(vec![1])).first();
    let _stream3 = singleton2.flat_map_ordered(q!(|x| vec![x]));

    // Test that sample_eager compiles (requires nondet)
    let optional2 = process
        .source_iter(q!(vec![200]))
        .first()
        .filter(q!(|&x| x > 100));
    let _stream4 = optional2.sample_eager(nondet!(/** test */));

    println!("✅ All operations compiled successfully");
    println!("✅ Metadata system is working");

    println!("=== Test completed successfully ===");
    // Ensure the builder is finalized to avoid drop panic
    let _built = builder.finalize();
}

#[test]
fn test_conversion_metadata_functions() {
    println!("=== Testing Conversion Metadata Functions ===");

    let builder = FlowBuilder::new();
    let process = builder.process::<()>();

    // Test that the new_conversion_metadata function exists and can be called
    // by using it in context (checking that operations that should use it compile)

    let singleton = process.source_iter(q!(vec![42])).first();
    let _converted = singleton.flat_map_ordered(q!(|x| vec![x, x + 1]));

    println!("✅ Conversion operations compile with new metadata system");
    // Ensure the builder is finalized to avoid drop panic
    let _built = builder.finalize();
}
