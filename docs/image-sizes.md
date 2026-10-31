# Image sizes — where each image is fetched and how large it is shown

Cover art sizes are a judgment call: one size per use is wasteful for the cache,
and a single size for everything is too small or too heavy somewhere. The only
basis for that call is how images are actually used, so this file records every
image fetch in the app together with the size it requests and the size it is
displayed at.

**Keep this file current.** When you add, remove or change an image fetch, its
requested size, its `cachedFallbackSizes`, or the displayed size of the element
that shows it, update the table below in the same change.

## How the cover art cache resolves a size

- Requested sizes are the OpenSubsonic `getCoverArt` `size` parameter, in image
  pixels. Displayed sizes in Tailwind classes are CSS px; the screen needs
  `CSS px × devicePixelRatio` image pixels to show an image without upscaling.
- The cache (`src-tauri/src/cover_art_cache.rs`) is keyed by
  `(profile, coverArtId, size)`. A different size is a different entry and a
  different download.
- `cachedFallbackSizes` lets a request reuse an already-cached **larger** entry
  instead of downloading its own size (`resolve_cached_cover_art`,
  `larger_fallback_sizes`). Fallback sizes smaller than or equal to the request
  are ignored, and a fallback never triggers a download: if neither the exact
  size nor a listed larger size is cached, the exact size is fetched.
- Size constants: `CoverArtSizes` in `src/features/albums/CoverArtSizes.ts`
  (`sm` 48, `md` 224, `lg` 512). `useCoverArt` defaults to `md`. The Rust side
  has its own constants (`SMTC_COVER_ART_SIZE`, `ANDROID_NOTIFICATION_COVER_ART_SIZE`).

## Usage

As of 2026-10-31. "Displayed" is the element's CSS size; `lg:` is the
Tailwind `lg` breakpoint (≥ 1024 CSS px wide).

| Where | Source | Requested | Cached fallback | Displayed (CSS px) |
|---|---|---|---|---|
| `ArtistRow` (via `BrowseList`) | artist `coverArtId` | `sm` 48 | `lg`, `md` | 32 × 32, `lg:` 24 × 24, circle |
| `SongItem` (via `QueueList`, `SearchSongList`) | song `coverArtId` (`displayCoverArtId` in search) | `sm` 48 | `lg`, `md` | 32 × 32 |
| `AlbumListItem` | album `coverArtId` | `sm` 48 | `lg`, `md` | 36 × 36 |
| `AlbumGridItem` in `AlbumGrid` | album `coverArtId` | `md` 224 | `lg` | grid cell, min 112 wide (`lg:` min 144), grows to fill the row |
| `AlbumGridItem` in `AlbumHorizontalList` | album `coverArtId` | `md` 224 | `lg` | 112 × 112 (`w-28`) |
| Album page (`routes/browse/album.tsx`) | album `coverArtId` | `lg` 512 | — | 208 wide (`w-52`), `lg:` 168 (`w-42`) |
| Artist page header (`routes/browse/artist.tsx`) | external artist image URL if any (`getArtistInfo2` large → medium → small, then `artistImageUrl`); otherwise artist `coverArtId` | external: as served (no size control); cover art: `lg` 512 | — | full width × 168 (`h-42`), `object-cover` |
| `PlayerBar` (desktop) | current entry `coverArtId` | `lg` 512 | `lg` (no effect) | one image used twice: background full width × 92, thumbnail 48 × 48 |
| `SPPlayerBar` (mini player) | current entry `coverArtId` | `md` 224 | `lg` | background, full width × bar height (row `h-17` = 68) |
| `SPPlayer` (full player) | current entry `coverArtId` | `lg` 512 | — | artwork 184 tall (`h-46`), `object-contain`; background full screen, `object-cover` |
| Windows SMTC (`playback/smtc_windows.rs`) | current entry `coverArtId` | 512 | — (exact key only) | OS media flyout |
| Android notification (`playback/android_runtime.rs`) | current entry `coverArtId` | 512 | — (exact key only) | OS media notification |

Not fetched: `ConnectQueueResolveDialog` renders an `<img src=''>` placeholder
(32 × 32) and requests nothing.

### Shared entries

The same `coverArtId` appears in several places, so these entries can serve
each other:

- Current track: `PlayerBar`, `SPPlayer`, SMTC and the Android notification all
  use the 512 entry; `SPPlayerBar` reuses it through its `lg` fallback.
- Album: the album page fetches 512; `AlbumGridItem` and `AlbumListItem` reuse
  it when cached.
- Artist: the artist page fetches 512 only when there is no external image;
  `ArtistRow` reuses it when cached.
