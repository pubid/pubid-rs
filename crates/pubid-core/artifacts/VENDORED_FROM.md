# Vendored artifact provenance

`doi.json` is byte-copied from
`pubid/pubid-grammar@94f1a12fee1c2eba202a896fddd25dc8921cda81`,
`artifacts/doi.json` (see `/pins.yaml`, `grammar_ref`).

- sha256 (file): fab7d55711531e2af2e131d561a03b2d1d244ef498eddf44ae386cb9ecd4bc15
- envelope checksum (self-declared, verified by parsanol on load):
  `sha256:083a4df7711856aa6381ea2345d3e084e2e3ac8d92b5dcb595c637a2a766b1c3`
- artifact edition 2, shape `parsanol-tree/v2`, baked at parsanol
  compiler 1.3.121
- CI fails the build when this file drifts from the pinned grammar ref.

Refresh by re-copying the file from the new grammar ref and updating
`grammar_ref` in `/pins.yaml` together with this note.
