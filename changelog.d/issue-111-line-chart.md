### Added
- LineChart (`controls::LineChart`, `ChartSeries`, `TimeRange`, `ChartHover`,
  `LineChartResponse`, `time_ago`, `bytes_per_second`) behind the new `chart` feature: a
  multi-series time-series chart with a value axis, a time axis, a legend, a hover cursor with
  a value read-out and 5 min / 1 h / 6 h range chips (#111). `ChartSeries` is a fixed-capacity
  ring buffer, series take Fluent palette tokens (alternating colours and dashes in high
  contrast), an optional area fill, and each frame keeps at most two points per pixel column.
  The AccessKit node summarises the visible samples. Gallery page "Charts".
