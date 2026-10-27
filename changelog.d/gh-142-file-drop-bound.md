### Fixed

- File drops stop reading at the size limit. An openable path is cut off by a bounded reader. A pathless drop, or a path that cannot be opened, is copied through the same reader so only `max + 1` bytes are kept.
- Pathless file drops, and paths that cannot be opened, are a known limit: the platform fully buffers them before the size limit applies. Only `max + 1` bytes are kept afterwards.
