### Fixed

- `Card` paints its elevation shadow behind its fill. A resting card had looked grey, because the shadow was drawn over the surface colour.
- The dark theme's foreground for a filled danger `Badge` is white. It was the fill's own red, so the label was unreadable (about 1.2:1). It is now about 6:1.
