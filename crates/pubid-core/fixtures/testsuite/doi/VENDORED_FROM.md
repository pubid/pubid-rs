# Vendored fixture provenance

The `*.yaml` files in this directory are byte-copied from
`pubid/pubid-testsuite@adecf10cef5c62b99222e459c4ccccdaedb13774`,
`tests/doi/` (see `/pins.yaml`, `testsuite_ref`):

- sha256 `resource.yaml`:
  31d7941c4e1465fde00e1ab92469a9a222bce84a214bf35f59fb9ad766e97878
- sha256 `_negative.yaml`:
  0a14710dcc63dfc47200fdd7d7b10bfd2e039a8ec52e748cffc78820adeaa44c
- sha256 `_status.yaml`:
  da8b5c732041f6936d83e8f046b33824cea9c87e149da4d107ae8efd143a050f

They make the smoke conformance run self-contained (`cargo test` needs
no network). CI additionally runs the conformance binary against a full
clone of the testsuite at the pinned ref, and fails the build when this
slice drifts from that ref.

Refresh by re-copying the files from the new testsuite ref and updating
`testsuite_ref` in `/pins.yaml` together with this note.
