### Fixed

- `Card` paints its elevation shadow behind its fill. A resting card had looked grey, because the shadow was drawn over the surface colour.
- The foreground for a filled danger `Badge` is white in light and dark themes. It was unreadable in dark (the fill's own red, about 1.2:1) and low-contrast in light (about 1.7:1). It is now about 6:1 on the fill in both.
