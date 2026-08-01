## ActivityHeatmap

A GitHub-style contribution grid over a series of dated counts.

### Constructor

```rust
ActivityHeatmap::new().data(vec![Activity::new("2026-01-01", 3)])
```

### Properties

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Data | `.data(d)` | `Vec<Activity>` | One entry per active period |
| Color Scheme | `.color_scheme(c)` | `Color` | Base color of the scale (default `Primary`) |
| Show Tooltip | `.show_tooltip(b)` | `bool` | Show a tooltip per cell (default `true`) |
| Show Month Labels | `.show_month_labels(b)` | `bool` | Label the months (default `true`) |
| Show Day Labels | `.show_day_labels(b)` | `bool` | Label the weekdays (default `true`) |
| Localize | `.localize(b)` | `bool` | Format dates in the browser locale instead of English |
| Interval | `.interval(i)` | `ActivityInterval` | `Daily` (default) or `Hourly` |
| Value Label | `.value_label(s)` | `&str` | Noun for the counted thing, e.g. `"commits"` |
| Start Date | `.start_date(s)` | `&str` | First date rendered, ISO-8601 `YYYY-MM-DD` |
| End Date | `.end_date(s)` | `&str` | Last date rendered, ISO-8601 `YYYY-MM-DD` |
| On Day Click | `.on_day_click(f)` | `Fn(Activity)` | Receives the clicked cell's activity |

### Activity

| Property | Method | Type | Description |
|----------|--------|------|-------------|
| Date | `new(date, count)` | `&str` | ISO-8601 `YYYY-MM-DD` |
| Count | `new(date, count)` | `u32` | Activity count for the period |
| Hour | `.hour(h)` | `u8` | Hour of day, only meaningful when the interval is `Hourly` |

Only active periods need supplying — dates absent from `data` render as zero.

### Example

```rust
ActivityHeatmap::new()
    .data(vec![
        Activity::new("2026-01-01", 3),
        Activity::new("2026-01-02", 7),
        Activity::new("2026-01-03", 1),
    ])
    .value_label("commits")
    .start_date("2026-01-01")
    .end_date("2026-01-31")
    .on_day_click(|activity| println!("{}: {}", activity.date, activity.count))
    .into()
```

### Limitations

Dates are ISO-8601 `String`s rather than a date type: Rusty has no date
abstraction and adds no `chrono` dependency, so Ivy's `DateOnly` is carried as
text. Nothing validates the format or ordering of `date`, `start_date` and
`end_date` — malformed values surface as rendering artifacts rather than errors.
