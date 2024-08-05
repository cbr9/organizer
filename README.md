![Travis (.org)](https://img.shields.io/travis/cbr9/alfred)
![Codecov](https://img.shields.io/codecov/c/github/cbr9/alfred)

TODO:
- [ ] implement custom variables
  - [x] regex based (using named capture groups)
  - [x] simple (string)
  - [ ] script 
- [x] integrate external templating library
- [ ] file content filter
- [x] compressed file extraction action
- [x] refactor
- [ ] refactor logger so all logs are put into one file. debug logs should not be written to stout unless a --verbose option is provided. each run should have its own log file in a folder named with the time the program was run
- [ ] create a resource struct that represents a file and holds a context for that specific file, so operations can be parallelized without affecting the Tera context; get rid of global context. Take the list of rule variables in constructor
- [ ] TESTS!!
