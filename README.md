# docread

## A program to find regular expression matches in .docx and zipped .docx files

### Command line options

```bash
Usage: docread [OPTIONS] --regex <REGEX>

Options:
  -r, --regex <REGEX>
          Regular expression to search for, e.g. 'Hi|[Hh]ello'

  -d, --dir <DIR>
          top-level dir or file name to search for docx or zip files

          [default: .]

  -c, --context <CONTEXT>
          number of context chars to show before/after matches

          [default: 75]

  -q, --quiet
          show file names & match status only

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version

docread -r 'Hi|[Hh]ello' -d $HOME/docs -c 100
   will find all occurrences of 'Hi' or 'Hello' or 'hello' in all .docx and zipped docx files
   in the $HOME/docs directory and its subdirectories, showing 100 chars of context
   on either side of the match


```

### Notes

Todo:

- [x] change file specification method
- [x] allow specification of context length on matches
- [ ] refactor handling of docx and zip into separate files
- [ ] add pdf processing
- [ ] replace glob with walkdir and allow zip, docx, and pdf to be specified independently
