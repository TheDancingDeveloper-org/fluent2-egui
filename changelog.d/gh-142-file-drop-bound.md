### Fixed

- File drops stop reading at the size limit. An openable path is cut off by a bounded reader. A pathless drop, or a path that cannot be opened, is copied through the same reader so only `max + 1` bytes are kept.
