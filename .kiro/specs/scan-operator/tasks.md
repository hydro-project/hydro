# Implementation Plan

- [X] 1. Implement the DFIR scan operator
  - Create a new operator in DFIR that processes input elements and emits intermediate results
  - Implement the scan operator with support for persistence arguments ('tick', 'static')
  - Support early termination when the function returns None
  - _Requirements: 1.1, 1.2, 1.3, 3.1, 3.2_

- [x] 2. Add tests for the DFIR scan operator
  - [x] 2.1 Create basic scan operator tests
    - Test scan with 'tick' persistence
    - Test scan with 'static' persistence
    - _Requirements: 4.2_

  - [x] 2.2 Create edge case tests
    - Test scan with empty input
    - Test scan with different accumulator and output types
    - Test scan with early termination (returning None)
    - _Requirements: 4.2_

- [ ] 3. Add the scan node type to Hydro IR
  - Add the Scan variant to the HydroNode enum in hydro_lang/src/ir.rs
  - Update relevant methods to handle the new node type
  - _Requirements: 3.2, 3.3_

- [ ] 4. Implement the scan method in the Stream trait
  - [ ] 4.1 Add the scan method to Stream for TotalOrder and ExactlyOnce streams
    - Implement the scan method following Rust's standard library scan pattern
    - Ensure proper type constraints are applied (function returns Option<U> to allow for early termination)
    - _Requirements: 1.1, 2.1, 2.2, 3.3_

  - [ ] 4.2 Add documentation and examples
    - Add comprehensive documentation with examples
    - Explain the difference between scan and fold
    - _Requirements: 4.1_

- [ ] 5. Add integration tests for the scan operator
  - Create tests that verify the end-to-end functionality
  - Test with different accumulator and output types
  - Test with different persistence options
  - _Requirements: 4.2, 4.3_

- [ ] 6. Update error messages for incorrect usage
  - Ensure clear error messages when scan is used with NoOrder streams
  - Ensure clear error messages when scan is used with AtLeastOnce streams
  - _Requirements: 4.3_