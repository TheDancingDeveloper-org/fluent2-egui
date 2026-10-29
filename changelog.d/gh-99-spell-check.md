### Added

- `Textarea` and `RichTextEditor` take an optional `SpellChecker` the app supplies. The library ships no dictionary. Misspellings are underlined, a secondary click or Shift+F10 opens suggestions (replace or ignore), and the checker runs only when the text changes. `WordListChecker` is the small stand-in the gallery and tests use.
