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
| api    | `870fb6cfd0d3b567dda4f453864cb57f5fb9ab4a8db85992e80c8ac2720a3acb` |
| amca   | `51ec2c39eae0e18e03bafbb4e34084053cfb64edab0a48fc959734b3e0200364` |
| nesc   | `a5e97216ce732f27ed44a60e2430a47316c53ebecbcd0ff52615d400819d433a` |
| ire    | `c5ce8862b73036a3bbee88c07dfbb7155e8e9a91e43a96907bd850456fa9f05f` |
| ansi   | `e1a6bdd21e45b92976e6737ce3d5bdc6f3115b093e3afa2a13972fae473690b9` |
| ecma   | `c00b04261a47f398fcd8596a1610587b39d57b00bc06b49e718e8131c75472ad` |
| gb     | `7724e6e1454eb7a680780e286a085049e85c1ef021ff5a35af16266c613d9b5f` |
| gost   | `830f339aa340cb2bf02687bc3d50a07a9f2946012f4f26aa7231080993b4ae50` |
| adobe  | `5ab76c704865b21e88abaadf90139a71c5a6c342e2532262a309c349f6dbd275` |

Every envelope is artifact edition 2, shape `parsanol-tree/v2`, baked at
parsanol compiler 1.5.1.0 (the #129 re-seal round: the doi grammar is
byte-identical modulo the stamp — same `semantics_hash`, new
`compiler_version`/`checksum`/`validated` block). Each envelope
self-declares its canonical checksum, which parsanol verifies on load;
every envelope here loads and self-tests green under the locked
`parsanol = "0.20"` runtime.

The wave-3 round adds `ansi`, `ecma`, `gb`, `gost`, and `adobe` from
the same pinned ref. `adobe` is vendored without a ported flavor: the
flavor has no testsuite corpus (no `tests/adobe/` in the testsuite),
so the crate carries the envelope for the drift gate while the runner
reports it as vendored-not-ported — no ground truth, no invented
fixture.

CI fails the build when any file drifts from the pinned grammar ref.

Refresh by re-copying the files from the new grammar ref and updating
`grammar_ref` in `/pins.yaml` together with this note.
