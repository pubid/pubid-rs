# Vendored artifact provenance

The `*.json` envelopes in this directory are byte-copied from
`pubid/pubid-grammar@dcd67cb97d6cdb512ed444584091e4786bb707a5`,
`artifacts/<flavor>.json` (see `/pins.yaml`, `grammar_ref`):

| flavor | sha256 (file) |
| ------ | ------------- |
| doi    | `448dba29b6222e87d41da3a06ae2cc23702822ff25a34dcece2b8215268efd6b` |
| iana   | `4eed6e4bc7585958653ac12df6395099a30126e259b54a1c70f88e1ca5f4aed0` |
| ietf   | `ed459ebebc336daf1af9e1b9ae0f27f19c873ad00fe994165f9920d7de957825` |
| isbn   | `689da0c5fef122b99f1b5d6c02339e46c9443590b9ec9b6ce81ff1b9300e73a0` |
| oasis  | `717fcfd6a61fc56c5f0dd33c181536e07e9bc16a511b415160af859bdea84cc5` |
| ogc    | `dc64972f391f51a537af35315c1954142e81c9b2810734745a92415576f3e3e9` |
| un     | `d201ee6387c0d45f84f18cb995a669d6d0f2d32a0b63f1627173623b0344c88a` |
| w3c    | `38fe8a4edcfa43b2f4d64f04187abaf40666439d7e1ad437ca3d1e3cdd8b6264` |
| xsf    | `b97d3fae6aebef1996c0e98c30d696ad7d6dabc5be9a8f6c9036bd8289e7aaf3` |

Every envelope is artifact edition 2, shape `parsanol-tree/v2`, baked at
parsanol compiler 1.5.1.0 (the #129 re-seal round: the doi grammar is
byte-identical modulo the stamp — same `semantics_hash`, new
`compiler_version`/`checksum`/`validated` block). Each envelope
self-declares its canonical checksum, which parsanol verifies on load;
the wave-2 envelopes all load and self-test green under the locked
`parsanol = "0.20"` runtime.

CI fails the build when any file drifts from the pinned grammar ref.

Refresh by re-copying the files from the new grammar ref and updating
`grammar_ref` in `/pins.yaml` together with this note.
