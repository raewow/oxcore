# Generated assets

The directories under here are produced from the World of Warcraft client data
using the unified extractor. They are **not** committed to the repository.

> **Legal**: The minimap textures are Blizzard's copyrighted artwork extracted
> from the game client. They are generated locally for your own private server
> and must **never** be redistributed. Leave this tree gitignored.

## Live-map tiles

A slippy-map tile set for the admin live map, built from the Vanilla 1.12
minimap textures (`textures\Minimap\md5translate.trs`).

Regenerate with:

```bash
cargo run --release --manifest-path tools/extractor/Cargo.toml -- minimap \
  -i "/path/to/WoW" \
  -o crates/web/public/assets
```

This writes `live-map/tiles/*.png` (one 256px tile per Azeroth grid cell) plus
`live-map/tiles/metadata.json`. The Leaflet map in `/admin/live-map` loads
these tiles from `/assets/live-map/tiles/{x}_{y}.png`.