# Unavailable value fallback — what to show when a tag has no value

When a tag the app displays has no value, each tag has one fallback, and that
fallback is the same everywhere the tag appears. Different tags may use
different fallbacks.

A value counts as missing when the server returns it as `null` **or** as an
empty string (use `||`, not `??`, when choosing the fallback).

## Basis: the MusicBrainz convention

The fallbacks follow MusicBrainz's special purpose names:

- `[unknown]` means the value exists (the song has an artist, the track has a
  title) but is not known. A missing tag means exactly this: the file does not
  say, so the app cannot tell.
- The square brackets mark the text as a placeholder, not a value as credited.
- MusicBrainz separates `[unknown]` (a title or artist that exists but is not
  known) from `[untitled]` (clearly shown to have no title) and
  `[no artist]` (there is no artist). A missing tag cannot tell these apart, and
  MusicBrainz uses `[unknown]` whenever a title is not clearly shown to be
  absent, so the app always uses `[unknown]`.

From this, a value gets `[unknown]` when it names the item being shown and
every item has one (a song always has a title and an artist). A value that is
supplementary information, which many items legitimately do not have, is
hidden instead: a label for it would add no information.

References:

- MusicBrainz, Special purpose artist:
  <https://musicbrainz.org/doc/Style/Unknown_and_untitled/Special_purpose_artist>
- MusicBrainz, Special purpose track title:
  <https://musicbrainz.org/doc/Style/Unknown_and_untitled/Special_purpose_track_title>
- MusicBrainz, Unknown and untitled (parent guideline):
  <https://musicbrainz.org/doc/Style/Unknown_and_untitled>

## Fallback per tag

Field names are those of the OpenSubsonic `Child` / `AlbumID3` responses.

| Tag | Fallback | Reason |
|---|---|---|
| Song title (`title`) | `[unknown]` | Names the song; every song has one |
| Song artist (`artist`, `displayArtist`, `artists`) | `[unknown]` | Names the song's artist; every song has one |
| Album artist (`displayAlbumArtist`, `albumArtists`, album `artist`) | `[unknown]` | Same as the song artist |
| Album name (`album`, album `name`) | `[unknown]` | Names the album; also used as the name of a group of songs that have no album tag |
| Folder (directory) name | `[unknown]` | Names the folder, like a title |
| Composer and other contributors (`displayComposer`, `contributors`) | Hidden | Credits shown in addition to the artist; most files do not tag them |
| Year (`year`) | Hidden | Supplementary |
| Genre (`genre`, `genres`) | Hidden | Supplementary |
| Track number (`track`) | Hidden (the number column stays blank) | Supplementary; the list order still shows the position |
| Disc number (`discNumber`) | Hidden | Supplementary |
| Song count (album `songCount`) | Hidden (the item is left out) | Supplementary |
| Duration (`duration`) | Undecided | |
| BPM, comment, moods, explicit status (`bpm`, `comment`, `moods`, `explicitStatus`) | Hidden | Supplementary |
| Ratings, play count, dates (`userRating`, `averageRating`, `playCount`, `created`, `starred`, `played`) | Hidden | Supplementary |
| Identifiers and sort keys (`musicBrainzId`, `isrc`, `sortName`) | Hidden | Not display values |

## Where the fallback is applied

The backend passes a missing value through as it is (`null`, or `""` for a
field typed as a string) and never writes a placeholder into the data it
returns. The placeholder is added only where a value is displayed:

- Frontend: `orUnknown()` / `UNKNOWN_VALUE` in `src/utils/unknownValue.ts`.
- Rust, for the places Rust displays itself (Windows SMTC, the Android media
  session): `or_unknown()` / `UNKNOWN_VALUE` in `src-tauri/src/unknown_value.rs`.

When no item is shown at all (nothing is playing, a page is still loading), the
field is left blank instead of showing `[unknown]`.

## Differences between servers

Only Navidrome's handling of the artist and album tags has been checked so far.

### Navidrome

Navidrome fills missing tags on the server side and returns the placeholder as
ordinary data:

- missing artist → `[Unknown Artist]`
- missing album → `[Unknown Album]`

The app receives a non-empty value in these cases, so it shows Navidrome's
string, not `[unknown]`. The app's fallback applies only when the server
returns no value.

References:

- Navidrome `consts.go` (`UnknownArtist`, `UnknownAlbum`):
  <https://github.com/navidrome/navidrome/blob/master/consts/consts.go>
- Navidrome, Tagging Guidelines:
  <https://www.navidrome.org/docs/usage/library/tagging>
