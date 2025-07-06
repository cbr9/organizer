# organize

TODO:
- [ ] implement custom variables
  - [x] regex based (using named capture groups)
  - [x] simple (string)
  - [ ] script
- [x] integrate external templating library
- [x] file content filter
- [ ] download action; struct Download { to: PathBuf, url: String?, if_exists: ConflictOption, confirm: bool}
- [x] write action to write files based on some content
- [x] compressed file extraction action
- [x] refactor
- [x] refactor logger so all logs are put into one file. debug logs should not be written to stout unless a --verbose option is provided. each run should have its own log file in a folder named with the time the program was run
- [x] create a resource struct that represents a file and holds a context for that specific file, so operations can be parallelized without affecting the Tera context; get rid of global context. Take the list of rule variables in constructor
- [ ] look for configs at git repo root
- [x] Add comprehensive test suite
- [x] Implement robust data validation

---

# Planned Features
- A watch command
  - A corresponding file watcher synchronization service that keeps the known_files cache in sync with the fs
- Undo capabilities
- backups
- Comprehensive testing using `rstest`, `proptest`, `mockall`, and `insta`.
- Data validation with `garde`.

---

## Testing

This project will use a combination of testing libraries to ensure code quality and correctness.

### Unit and Integration Testing with `rstest`

[rstest](https://crates.io/crates/rstest) will be the primary framework for writing unit and integration tests. Its fixture-based approach allows for writing clean, readable, and maintainable tests by decoupling test setup from the test logic.

### Property-Based Testing with `proptest`

For more robust testing of functions, especially those with complex inputs, [proptest](https://crates.io/crates/proptest) will be used. Property-based testing helps in finding edge cases by generating a wide range of inputs and asserting that certain properties hold true for all of them.

### Mocking with `mockall`

To isolate components and test them independently, [mockall](https://crates.io/crates/mockall) will be used for creating mock objects. This is particularly useful for mocking external dependencies and services, allowing for more controlled and predictable tests.

### Snapshot Testing with `insta`

[Insta](https://crates.io/crates/insta) will be used for snapshot testing. This is ideal for testing complex data structures and the output of the templating engine. Snapshots are stored in files and are reviewed and approved manually. Any changes in the output will cause the test to fail, preventing unintended regressions.

## Validation

To ensure the integrity of data throughout the application, especially when parsing configuration files and user input, `garde` will be used.

### Data Validation with `garde`

[Garde](https://crates.io/crates/garde) is a validation library that allows for defining validation rules directly on structs and enums using derive macros. This will help in catching invalid data early and providing clear error messages. It supports a wide range of validation rules, including length, range, and custom validators.