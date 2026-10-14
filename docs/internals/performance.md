# Performance

Two tools keep an eye on speed: the Performance Report inside the editor, and
Criterion benchmarks with limits that CI enforces.

## Targets

What Roxanne aims for, not what it has been measured to do: no check enforces
these numbers yet.

- Startup in under 200 ms on a modest machine.
- Smooth scrolling and editing on files of 1 to 5 MB. Larger files are refused
  for now (the limit is 5 MiB).
- Memory use that stays stable while the editor sits idle.

## The Performance Report

**Tools → Performance Report** shows in the status bar how long these took the
last time they ran:

| Measure | What is timed |
|---|---|
| Startup | building the window state, plugins included |
| Render | refreshing the viewport cache after a change or a scroll |
| Search | searching the open file |
| Search in files | the workspace search, from the request to the results |
| Open | reading a file, from the request to the text in its tab |
| Save | writing a file, from the request to the confirmation |

Nothing is recorded or sent anywhere; the numbers live only until Roxanne
quits.

## Benchmarks

The benchmarks in `benches/` measure the editor core without a window:

| Group | Scenario |
|---|---|
| `text_buffer_build` | building a `TextBuffer` from 1,000 to 200,000 lines |
| `text_buffer_edit` | one insertion in the middle, and one 128-byte deletion, in 10,000 lines |
| `text_buffer_repeated_edits` | 200 insert and delete pairs in 50,000 lines, and walking positions across them |
| `text_buffer_history` | 80 undos then 80 redos in 10,000 lines |
| `viewport_cache_update` | moving a 60-line viewport one line at a time through 10,000 and 50,000 lines |
| `viewport_cache_reuse` | scrolling an 80-line viewport 120 lines down through 100,000 lines, so most lines are already cached |

Run them, then check the results against the limits:

```bash
cargo bench --bench text_buffer --bench viewport -- --noplot
python3 scripts/check_benchmarks.py
```

`scripts/check_benchmarks.py` reads Criterion's estimates from
`target/criterion/` and the limits from `scripts/benchmark-thresholds.toml`, one
`max_mean_ms` (or `max_mean_ns`) per benchmark. It fails when a benchmark has
no result, when a result has no limit, or when a mean is over its limit.
`--allow-unknown` ignores results without a limit, `--thresholds` and
`--criterion-dir` point it at other files.

The **Benchmarks** workflow runs both steps on every pull request and every
push to `main`, on a GitHub-hosted Ubuntu runner.

## Changing a limit

The limits are set for GitHub's runners, which are slower and noisier than a
desktop: each one leaves room above the slowest run seen there. When a change
makes a benchmark legitimately slower, or much faster, run the benchmarks a few
times, update `max_mean_ms` in `scripts/benchmark-thresholds.toml`, and say why
in the pull request. A comment next to a limit records how it was chosen when
that is not obvious.

## Known costs

- Every edit rebuilds the buffer's full-text `String` cache, so an edit costs
  time in proportion to the file size. The comment on the
  `insert_delete_burst` limit records what that costs on CI.
- Highlighting reparses incrementally, but the highlighter still receives a
  copy of the whole text after each edit.
- The workspace search reads every file it does not skip; it stops at 500
  matches.
