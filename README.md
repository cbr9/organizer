**NOTE**: This version is unmaintained, as all development efforts are put into the 2.0 version.

# organizer
This is a productivity tool designed to keep your filesystem in order.
For example, you can use this to keep your downloads folder clean. You can even manage several folders, so you can keep not only your downloads folder clean, but also your home directory.

## Requirements
- Rust
- Cargo

## Installation
To install, run the following commands:
```$xslt
git clone git@github.com:cbr9/organizer.git
cd organizer
cargo install --path .
```

or
```$xslt
git clone https://github.com/cbr9/organizer.git
cd organizer
cargo install --path .
```

## Configuration 
This tool requires a YAML configuration file. You can find an example configuration at ```example/config.yaml```.
In this file you must declare rules that will define the program's behaviour as follows:

```$xslt
rules:
    extension:
        - new_folder: <string>
        - patterns:
            - regex: <string>
              new_folder: <string>
            - ... more patterns

    ... more extensions
```

If you're not familiar with YAML, here we're defining an object with the name `rules`, which itself holds objects, each of which defines an extension.
Within it we define a `new_folder` field, which is required, and a `patterns` field, which is optional.
Files with the given `extension` will be moved to `new_folder`.
The `patterns` field defines an array of objects, each of which must have `regex` and a `new_folder` field. 
Filenames matching the specified regular expression will be moved to its associated `new_folder`.

Note: only the `rules` object is required, together with some rules within it. You can define YAML variables outside `rules`.

## Usage
Run
```$xslt
organizer --watch /folder/to/watch/ --config config.yaml --delay 2 --daemon
```
As stated above, you can pass several paths to `--watch`.
The `delay` argument is optional. By default, `organizer` will wait for three seconds before it moves any file.
This is to allow browsers to automatically open files after they've been downloaded (specifying a delay of 0 will break this behaviour, since the browser won't find the given file). The `daemon` option starts the process in the background.

I plan to change this behaviour so that files are not moved while they're open in another process. Feel free to open a PR if you want to help.

### Recommendation
You can set `organizer` to start on system launch. 
Memory footprint is about 300K (tested on Linux with kernel 5.7.7), so it has a very negligible impact.
