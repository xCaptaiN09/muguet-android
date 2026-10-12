# Muguet provenance

Upstream: https://github.com/c22dev/muguet
Immutable source: https://github.com/c22dev/muguet/tree/ddcc54f44ae2a21b8c6e43ec1c925b55b352ffbd
Source archive: https://github.com/c22dev/muguet/archive/ddcc54f44ae2a21b8c6e43ec1c925b55b352ffbd.tar.gz
Author: Constantin Clerc (c22dev). License: GPL-3.0-or-later (root LICENSE).
`SOURCE.sha256` records unmodified pinned upstream source files. Retained local
checkout: ~/.cache/muguet-android/upstream-muguet-ddcc54f. No game content is vendored.

The reusable host adapts src/main.rs runtime functions and imports host.rs, fs.rs,
io.rs, input.rs, audio.rs, gpu.rs, patch.rs, mp.rs, proto.rs, server.rs and
minimap_alpha.wgsl, retaining original attribution headers. Desktop app.rs,
Apple pad/keyboard/touch APIs and launcher configuration are replaced by the
explicit Android platform/session API. The host and GPU source retain the
movie-preload, scenario-collision, pipeline-wait, BC decode and minimap fixes.

The upstream vendor/wgpu-core patch changes only tracker clear() behavior in
src/track/{metadata,buffer,texture}.rs: retain vector lengths and reset only
owned resource slots (and clear complex texture state), avoiding repeated full
resizing. It changes allocation/performance rather than a required renderer API
or correctness contract. The host only uses public wgpu APIs and crates.io
wgpu-core 30.0.1 satisfies them; this optimization is not vendored/activated.
The pinned immutable reference retains the patch for future profiling.

An independent full source tarball (including the vendor patch) is retained at
~/.local/share/muguet-android/game-backup/full-game-v1/source/muguet-ddcc54f.tar.gz:
584910 bytes; SHA-256 edf938cb7e57021a58747d0c03c9324cea09aa8a4b450549a617c22755463f3c.
