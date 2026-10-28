# Terminal design

## Theme

A compact data workbench for comparing model and agent measurements. Source data
occupies the main surface; selected-row context stays beside it when space allows.

## Palette

Terminal foreground and background remain the base. Amber `#d4ad6a` identifies
active controls, key hints and metrics; the active tab is an amber pill with dark
text. Selection uses `#30343b`, white bold names and an amber `▌` gutter. Rules,
scrollbar tracks and bar tracks use `#4b5059`.
Provider colors come from source metadata. Green, yellow and magenta distinguish
confirmed, likely and announced states; text labels carry the same information.

## Typography

Use the terminal's configured monospace font. Numeric cells align right.
`Bun.stringWidth` and grapheme segmentation determine cell fit, including CJK.
Details wrap at word boundaries and split long URLs only when necessary.

## Components

Tabs identify each board by a number and name. Tables show rank, model/event and
the active metric before optional columns; when space remains, an inline meter
follows the active metric. Tracker always exposes its stage. Details group
metrics, fields, summary and link under muted headings, with a label column and
hanging indents. Help pairs amber keys with actions. Scrollbars size the thumb by
the visible share. Charts use meters of small squares: `▪` filled cells in the provider color
and a faint `▫` track, with explicit values
and the board maximum. The calendar puts its 90-day summary and selected event
beside the month when space allows, otherwise below it.

## Layout

The shell uses four rows: a tab bar with the brand and source health, a board
title rule with description, status, count, sort and view, then key hints and a
status line below the body. The body fills remaining terminal height.
`src/tui/layout.ts` owns these rows and the table column offsets that both the
renderer and mouse hit tests use. A detail pane of 30% width, clamped to 36-48
columns, appears at 116x16 or above. Enter opens full details at every size.

## Depth

Selection background and one vertical divider separate focus from context.
Avoid nested boxes that consume data columns. The alternate screen restores the
previous terminal content on exit.

## Interaction

Keyboard actions update immediately. Preserve row identity across sorts and
reloads. Render only the visible row window; move that window with selection.
Refreshing keeps existing data. Footer hints contain complete actions. Mouse hit
regions use the same column and viewport calculations as the renderers.

## Responsive behavior

Support 36x8 and larger terminals. Tabs shorten to short labels, then to numbers
around the active label; distant tabs drop out last. Keep the active metric and remove lower-priority columns. At the minimum
height, omit table/chart headers so the selected row remains visible. Help and
details clamp their scroll offsets after resize.

## Maintenance

Inspect actual Ink snapshots at narrow, normal and wide sizes, with Chinese news,
long model names, missing metrics, filters, errors, charts and the reset calendar.
Keep shortcuts in README and in-app help aligned. Fixture previews must be labeled
and must not write caches or fetch the network.
