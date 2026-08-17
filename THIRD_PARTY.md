# Third-party licensing

ReX contains software and font-derived assets with different provenance. The
top-level `LICENSE-MIT` and `LICENSE-APACHE` restore the license files referenced
by the upstream ReX README. They do not override third-party terms.

## XITS font assets

The repository contains XITS-derived font binaries used by ReX, including:

- `rex-xits.otf`
- `rex-xits.woff2`
- `rex-xits_old.otf`
- `tests/out/rex-xits.otf`
- `tests/out/rex-xits_old.otf`

Historical ReX commits identify the source font as `XITS Math` / `xits.otf`.
XITS is distributed under the SIL Open Font License, Version 1.1. See
`LICENSES/XITS-OFL-1.1.txt` for the license text and attribution.

## STIX-family generated font data

`fonts/stix/` and `fonts/stix2/` contain generated metrics and tables derived
from STIX-family font data. STIX fonts are distributed under the SIL Open Font
License, Version 1.1. See `LICENSES/STIX-OFL-1.1.txt` for the license text and
attribution.

## trust CI template

The historical `.travis.yml`, `appveyor.yml`, and `ci/` configuration was
introduced from the `japaric/trust` CI template v0.1.1. That upstream project
is distributed under MIT and Apache-2.0 terms; those license texts are present
at this repository root as `LICENSE-MIT` and `LICENSE-APACHE`.

## Legacy BSD-like notice

The upstream ReX README has stated since 2016 that portions of the project are
covered by "various BSD-like licenses". The current tree does not identify
which files that sentence refers to, and this provenance pass did not find a
BSD license file or a file-level BSD notice in the current ReX tree.

This repository does not attempt to relicense or erase any such terms. Any
original file-specific or third-party license continues to apply. If a future
provenance review identifies the affected files, their notices should be added
here verbatim.
