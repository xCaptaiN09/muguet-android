# Asset delivery direction

The game will execute on the phone in both modes. Asset delivery controls
where file reads obtain models, textures, audio, and other game content.

| Mode | Initial storage/download | Offline behavior | Development order |
| --- | --- | --- | --- |
| Local files | Install the required browser-build assets upfront | Installed content is available without a server | First playable target |
| Progressive download | Fetch a startup set and cache additional content while playing | Uncached content requires the asset server | Follow after local gameplay works |

Both options are intended project directions, not implemented features of the
first diagnostic APK. Its native checks need no assets or internet access.
The original reported approximately 700 MB startup download does not establish
our Android startup size; that must be measured with the chosen runtime and
content set.

## Hosting candidates for later investigation

[Cloudflare Pages limits](https://developers.cloudflare.com/pages/platform/limits/)
currently allow 20,000 files per Free-plan site, with a maximum of 25 MiB per
file. Large assets therefore need chunking for that approach. The supplied
PlayGTA5 helper code already describes verified 4 MiB chunk packaging.

An inventory-only count with that scheme estimates 16,748 payload files for
the full supplied snapshot, or 15,245 after excluding the same `env_test` and
`verityradio` directories that Muguet skips. These counts deduplicate chunked
files by their declared SHA-256, matching the helper's hash-based chunk paths.
They exclude extra generated helper/index files. No assets were packaged,
uploaded, or fetched to validate these estimates, and no hosting availability
or perpetual free service is promised.

[GitHub Releases](https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases)
supports release archives under 2 GiB per asset. This is useful for downloading
an installation bundle; release ZIP files are not an individual-file asset
endpoint for the native reader. The private code repository remains private.

Before implementing progressive delivery, define and test chunk lookup,
checksums, request retries, bounded cache storage, and behavior when the server
is unavailable. The Android application should allow an explicit asset source
rather than depend on the original site's availability.
