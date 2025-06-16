[![codecov](https://codecov.io/github/cbr9/organize/graph/badge.svg?token=9K4VVDHUMH)](https://codecov.io/github/cbr9/organize)

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
- [ ] refactor logger so all logs are put into one file. debug logs should not be written to stout unless a --verbose option is provided. each run should have its own log file in a folder named with the time the program was run
- [x] create a resource struct that represents a file and holds a context for that specific file, so operations can be parallelized without affecting the Tera context; get rid of global context. Take the list of rule variables in constructor
- [ ] restructure confirm prompts; actions need to take a list of resources, run all the confirmation prompts, and based on that filter the entries they got, then execute action in parallel on all filtered entries
- [ ] look for configs at git repo root
- [ ] TESTS!!
