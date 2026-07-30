(() => {
  const TILE_SIZE = 256;
  const WORLD_TILE_SIZE = 533.333333;
  const BOUNDS = [[0, 0], [69 * TILE_SIZE, 85 * TILE_SIZE]];
  let map;
  let markers = new Map();
  let initialized = false;

  function mapPosition(player) {
    const sourceX = 32 - player.position_y / WORLD_TILE_SIZE;
    const sourceY = 32 - player.position_x / WORLD_TILE_SIZE;
    let x;
    let y;
    if (player.map === 0) {
      x = sourceX - 17 + 55;
      y = sourceY - 22 + 19;
    } else if (player.map === 1) {
      x = sourceX - 23 + 8;
      y = sourceY - 9 + 13;
    } else {
      return null;
    }
    return [y * TILE_SIZE, x * TILE_SIZE];
  }

  function icon() {
    return L.divIcon({
      className: "live-map-marker",
      html: '<span></span>',
      iconSize: [14, 14],
      iconAnchor: [7, 7],
    });
  }

  async function refresh() {
    const response = await fetch("/api/admin/live-map/players", { credentials: "same-origin" });
    if (!response.ok) return;
    const players = await response.json();
    const active = new Set();
    for (const player of players) {
      const position = mapPosition(player);
      if (!position) continue;
      active.add(player.guid);
      const label = `${player.name} · level ${player.level} · zone ${player.zone}`;
      const marker = markers.get(player.guid);
      if (marker) {
        marker.setLatLng(position).bindTooltip(label);
      } else {
        markers.set(player.guid, L.marker(position, { icon: icon(), title: player.name }).addTo(map).bindTooltip(label));
      }
    }
    for (const [guid, marker] of markers) {
      if (!active.has(guid)) {
        marker.remove();
        markers.delete(guid);
      }
    }
  }

  function initialize() {
    if (initialized || !window.L || !document.getElementById("live-map")) return;
    initialized = true;
    map = L.map("live-map", {
      crs: L.CRS.Simple,
      minZoom: -4,
      maxZoom: 2,
      maxBounds: BOUNDS,
      maxBoundsViscosity: 1,
      attributionControl: false,
    });
    L.tileLayer("/assets/live-map/tiles/{x}_{y}.png", {
      bounds: BOUNDS,
      tileSize: TILE_SIZE,
      noWrap: true,
      minNativeZoom: 0,
      maxNativeZoom: 0,
    }).addTo(map);
    map.fitBounds(BOUNDS);
    refresh();
    window.setInterval(refresh, 10000);
  }

  document.addEventListener("DOMContentLoaded", initialize);
  new MutationObserver(initialize).observe(document.documentElement, { childList: true, subtree: true });
})();
