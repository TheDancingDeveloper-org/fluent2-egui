### Added

- `Toaster::push_keyed` coalesces toasts: while a toast with the same key is showing, a push replaces its content, moves it to the newest slot and restarts its timeout, so a bulk action reads as one toast ("Paused 12 torrents").
- A collapsed "+N more" row above the toast stack counts toasts waiting behind the visible ones.
- `Toaster::set_max_visible` sets how many toasts the stack draws (default 3).
- `Toaster::history` returns the last 100 pushed toasts, oldest first, and `Toaster::clear_history` forgets them, for a status-bar history list.
- Gallery toasts page: a keyed "Pause 12 torrents" button, a "Push 50 toasts" button and a collapsed history list.
