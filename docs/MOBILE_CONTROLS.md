# Experimental mobile controls

Version 0.3.0-mobile replaces the provisional button strip with native Java
Views/Canvas controls. Original white vector icons and translucent circular targets
are inspired by GTA San Andreas mobile. The action mapping and floating-stick
behavior follow c22dev/Muguet `src/touch.rs` at
`ddcc54f44ae2a21b8c6e43ec1c925b55b352ffbd` (GPL-3.0-or-later); this is an Android
adaptation, not a claim of complete iOS overlay parity.

The center remains available for the game. All action targets are at least 48 dp;
layout uses safe system-bar/cutout insets and 8 dp target separation. Geometry tests
cover safe landscape areas from 440×320 dp to 1280×320 dp. Device screenshots and
TalkBack operation remain device-review work, not inferred from JVM geometry tests.
Virtual native accessibility buttons expose action names, click, focus and toggle
state. The Info button reveals counters; loading and failure remain visible while
normal presentation hides technical status. Android Back leaves the session.

| Control | On foot | In vehicle |
| --- | --- | --- |
| Left floating stick X | A / D | A / D steering |
| Left floating stick Y | W / S | Numpad8 / Numpad2 pitch |
| Stick beyond 1.35×58 dp radius | Left Shift sprint | — |
| Primary right circle | Left mouse fire | W gas |
| Lower right circle | Right mouse aim toggle | S brake |
| Above primary | Space jump | Space handbrake |
| Inner circle | F enter | F exit |
| Inner-left circle | R reload | Touch horn |
| Top-right weapon | Tab | Tab |
| Top-right cover/radio | Q cover | Q radio |
| Top-right crouch/look back | Left Ctrl | C |
| Top-right camera | V | V |
| Top-left pause | Escape; show menu keys | Same |
| Top-left phone | Up; show menu keys | Same |
| Top-left menu | Toggle menu keys | Same |
| Top-left switch | Left Alt | Same |
| Menu row | Left / Up / Down / Right, Enter, Backspace | Same |
| Free right-side drag | Relative mouse camera/cursor | Same |

The stick owns one pointer ID in the left 40% below 70 dp, with a .35 directional
deadzone (.6 for vehicle pitch). Camera and action fingers are independent; pointer
indices can reorder without changing owners. Non-wheel action drags also move the
camera as upstream does. Weapon, character-switch, and vehicle-radio holds accept
radial drags: beyond 10 dp they repeatedly push relative mouse movement (up to 40
per 16 ms tick, matching upstream's reference step). This timing is an Android
adaptation; advanced wheel selection confirmation and full game-menu gesture parity
have not been verified. Menu navigation uses explicit buttons rather than swipe
navigation. Vehicle context is polled from the real script tick every 750 ms.

Short keys, mouse actions and horn have a minimum 120 ms hold. A fresh press cancels
its pending release; all owners of the same output are aggregated, including
hardware keyboard keys. Cancellation, context/menu changes, focus loss, pause,
stop, surface loss and session failure immediately clear held inputs and aim latch.
No delayed native input queue survives suspension. Backgrounding ends this first
experimental session in its isolated `:game` process.

## Requested graphics retry

The real session argument builder adds these flags, preserving 960×432, Sandbox,
`-frameLimit=2` (existing 30 FPS target), offline flags and BC CPU fallback:

```
-textureQuality=0 -shaderQuality=1 -grassQuality=0 -reflectionQuality=0
-shadowQuality=1 -lodScale=0.35 -particleQuality=0 -waterQuality=0
```

The requested profile is logged with exact arguments and exported as
`requestedGraphicsProfile: mobile-conservative-v1`. Shader quality zero is avoided
because the pinned upstream normal shader path crashes there. Shadow quality one
is the documented minimum. Parameter ranges were checked against the
[Rockstar command-line reference](https://support.rockstargames.com/articles/2VjbVziQCiTiiVhDbmnexc/full-list-of-command-line-parameters-for-grand-theft-auto-v-on-pc).
These are requested settings, not proof of effective guest settings or lower RSS.

Real local legal/startup rendering was observed with the previous APK. Android
subsequently killed `:game` for LOW_MEMORY (observed RSS 2.9 GB and 2.2 GB on separate
runs); no playable world has been verified. This profile is an experiment and does
not establish an OOM fix. The 3 GiB initial guest memory, 5 GiB cap/reservation,
64 KiB guard, non-moving shared memory, matching AOT configuration, trusted embedded
code digest and original assets/settings remain unchanged. No AI semantic judgment
is needed for these exact control mappings; Jev was not called.
